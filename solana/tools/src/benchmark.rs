use crate::{ensure, root, Result};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs, path::Path};

const PATTERNS: [(&str, &str); 7] = [
    ("valid_cu", "invalid=false CU="),
    ("late_invalid_cu", "invalid=true CU="),
    ("consumer_valid_cu", "consumer valid CU="),
    ("consumer_invalid_cu", "consumer invalid CU="),
    ("record_valid_cu", "record valid CU="),
    ("consumer_recorded_cu", "consumer recorded CU="),
    ("largest_upload_tx_bytes", "largest_tx="),
];
type Measurements = BTreeMap<String, u64>;

fn parse(log: &str) -> Result<Measurements> {
    let mut result = Measurements::new();
    for (key, prefix) in PATTERNS {
        let values: Vec<_> = log
            .match_indices(prefix)
            .map(|(at, _)| {
                log[at + prefix.len()..]
                    .chars()
                    .take_while(char::is_ascii_digit)
                    .collect::<String>()
            })
            .collect();
        ensure(values.len() == 1, &format!("expected exactly one {key} measurement"))?;
        result.insert(key.into(), values[0].parse()?);
    }
    Ok(result)
}

fn validate(actual: &Measurements, baseline: &Measurements) -> Result<()> {
    for (key, _) in PATTERNS {
        let value = *actual.get(key).ok_or_else(|| format!("missing measurement: {key}"))?;
        ensure(value > 0, &format!("invalid measurement: {key}"))?;
        if key.ends_with("_cu") {
            ensure(
                key != "consumer_recorded_cu" || value < 50_000,
                "saved-result consumption exhausted the application budget",
            )?;
            ensure(value < 1_400_000, &format!("{key} reached transaction ceiling"))?;
            let previous = *baseline.get(key).ok_or_else(|| format!("missing baseline: {key}"))?;
            ensure(
                u128::from(value) * 100 <= u128::from(previous) * 105,
                &format!("{key} regressed by more than 5%"),
            )?;
        } else {
            ensure(value <= 1232, "upload transaction exceeds packet limit")?;
        }
    }
    Ok(())
}

fn baseline_measurements(report: &serde_json::Value) -> Result<Measurements> {
    PATTERNS
        .iter()
        .map(|(key, _)| {
            let value = report["measurements"][key]
                .as_u64()
                .ok_or_else(|| format!("missing numeric baseline: {key}"))?;
            Ok(((*key).to_owned(), value))
        })
        .collect()
}

pub fn run(log: &Path) -> Result<()> {
    let actual = parse(&fs::read_to_string(log)?)?;
    let baseline: serde_json::Value =
        serde_json::from_slice(&fs::read(root().join("benchmarks/integration.json"))?)?;
    validate(&actual, &baseline_measurements(&baseline)?)?;
    let mut hashes = BTreeMap::new();
    for path in [
        "target/deploy/zisk_solana_program.so",
        "target/deploy/zisk_consumer_example.so",
        "Cargo.lock",
        "runtime-tests/Cargo.lock",
        "package-lock.json",
    ] {
        hashes.insert(path, hex::encode(Sha256::digest(fs::read(root().join(path))?)));
    }
    let report = json!({"measurements": actual, "hashes": hashes});
    fs::create_dir_all(root().join(".cache"))?;
    fs::write(root().join(".cache/benchmark.json"), serde_json::to_string_pretty(&report)? + "\n")?;
    println!("{}", serde_json::to_string(&actual)?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    const LOG: &str = "invalid=false CU=1000\ninvalid=true CU=1010\nconsumer valid CU=1100 headroom=1398900\nconsumer invalid CU=1090\nrecord valid CU=1100\nconsumer recorded CU=1200\nupload chunks=8 bytes=6752 rent=48198000 largest_tx=1150";

    #[test]
    fn requires_one_of_every_measurement() {
        assert!(parse("build completed").is_err());
        assert!(parse(&format!("{LOG}\ninvalid=false CU=1000")).is_err());
        assert!(parse(&LOG.replace("CU=1000", "CU=-1")).is_err());
        let values = parse(LOG).unwrap();
        validate(&values, &values).unwrap();
        assert_eq!(values["consumer_valid_cu"], 1100);
        let report =
            serde_json::from_slice(&fs::read(root().join("benchmarks/integration.json")).unwrap())
                .unwrap();
        let baseline = baseline_measurements(&report).unwrap();
        validate(&baseline, &baseline).unwrap();
    }

    #[test]
    fn rejects_regressions_and_resource_limits() {
        let baseline = parse(LOG).unwrap();
        for (key, value) in [
            ("valid_cu", 1051),
            ("consumer_valid_cu", 1_400_000),
            ("consumer_recorded_cu", 50_000),
            ("largest_upload_tx_bytes", 1233),
            ("valid_cu", 0),
            ("valid_cu", u64::MAX),
        ] {
            let mut changed = baseline.clone();
            changed.insert(key.into(), value);
            assert!(validate(&changed, &baseline).is_err(), "{key}={value}");
        }
        let mut boundary = baseline.clone();
        boundary.insert("valid_cu".into(), 1050);
        boundary.insert("largest_upload_tx_bytes".into(), 1232);
        validate(&boundary, &baseline).unwrap();
    }
}
