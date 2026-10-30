use std::collections::BTreeMap;

/// `--metadata KEY=VALUE`, repeatable, for every command that submits a job to a coordinator.
#[derive(clap::Args, Debug, Default)]
pub struct JobMetadataArgs {
    /// Attach a KEY=VALUE label to the job, which the coordinator keeps with it and logs when it
    /// completes. Repeat for more labels; a key given twice keeps its last value
    #[arg(long = "metadata", value_name = "KEY=VALUE", value_parser = parse_metadata_entry)]
    entries: Vec<(String, String)>,
}

impl JobMetadataArgs {
    /// The labels by key; empty when none was given.
    pub fn to_map(&self) -> BTreeMap<String, String> {
        self.entries.iter().cloned().collect()
    }
}

/// Splits `KEY=VALUE` at its first `=`, so the value may hold more.
fn parse_metadata_entry(entry: &str) -> Result<(String, String), String> {
    match entry.split_once('=') {
        Some((key, value)) if !key.is_empty() => Ok((key.to_string(), value.to_string())),
        _ => Err(format!("expected KEY=VALUE with a non-empty KEY, got `{entry}`")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[derive(Parser)]
    struct Cli {
        #[command(flatten)]
        metadata: JobMetadataArgs,
    }

    fn parse(args: &[&str]) -> Result<BTreeMap<String, String>, clap::Error> {
        Cli::try_parse_from(std::iter::once("cli").chain(args.iter().copied()))
            .map(|cli| cli.metadata.to_map())
    }

    #[test]
    fn collects_every_entry() {
        assert_eq!(
            parse(&["--metadata", "team=zkvm", "--metadata", "run=42"]).unwrap(),
            BTreeMap::from([
                ("run".to_string(), "42".to_string()),
                ("team".to_string(), "zkvm".to_string()),
            ])
        );
    }

    #[test]
    fn is_empty_without_the_flag() {
        assert!(parse(&[]).unwrap().is_empty());
    }

    #[test]
    fn splits_at_the_first_equals_sign() {
        assert_eq!(
            parse(&["--metadata", "query=a=b", "--metadata", "empty="]).unwrap(),
            BTreeMap::from([
                ("empty".to_string(), String::new()),
                ("query".to_string(), "a=b".to_string()),
            ])
        );
    }

    #[test]
    fn keeps_the_last_value_of_a_repeated_key() {
        assert_eq!(
            parse(&["--metadata", "run=1", "--metadata", "run=2"]).unwrap(),
            BTreeMap::from([("run".to_string(), "2".to_string())])
        );
    }

    #[test]
    fn rejects_an_entry_without_a_key() {
        assert!(parse(&["--metadata", "=value"]).is_err());
        assert!(parse(&["--metadata", "no-equals-sign"]).is_err());
    }
}
