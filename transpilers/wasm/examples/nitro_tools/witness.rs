use super::*;
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::{
    de::{self, MapAccess, SeqAccess, Visitor},
    Deserialize, Deserializer,
};
use serde_json::Value;
use std::{collections::BTreeMap, fmt};
use tiny_keccak::{Hasher, Keccak};

#[derive(clap::Args)]
pub struct Args {
    #[arg(long)]
    block_file: PathBuf,
    #[arg(long)]
    replay_wasm: PathBuf,
    #[arg(long)]
    output: PathBuf,
    #[arg(long)]
    expected: PathBuf,
    /// Additional delayed messages as a JSON array of Number and DataB64 objects.
    #[arg(long)]
    checkpoint_messages: Option<PathBuf>,
    /// Starting accumulator; zero is only a fixture anchor, not L1 authentication.
    #[arg(long, value_parser = bytes32, default_value = "0x0000000000000000000000000000000000000000000000000000000000000000")]
    sequencer_before: [u8; 32],
    #[arg(long, value_parser = bytes32, default_value = "0x0000000000000000000000000000000000000000000000000000000000000000")]
    delayed_before: [u8; 32],
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct State {
    block_hash: String,
    send_root: String,
    batch: u64,
    pos_in_batch: u64,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Message {
    number: u64,
    data_b64: String,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Recording {
    start_state: State,
    expected_end_state: State,
    batch_info: Vec<Message>,
    has_delayed_msg: bool,
    #[serde(default)]
    delayed_msg_b64: String,
    delayed_msg_nr: Option<u64>,
    preimages_b64: BTreeMap<String, BTreeMap<String, String>>,
}

// Reject duplicate keys even inside ignored compiled caches, before typed deserialization.
struct Unique(Value);
impl<'de> Deserialize<'de> for Unique {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        struct Json;
        impl<'de> Visitor<'de> for Json {
            type Value = Unique;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("JSON with unique object keys")
            }
            fn visit_bool<E: de::Error>(self, v: bool) -> std::result::Result<Unique, E> {
                Ok(Unique(v.into()))
            }
            fn visit_i64<E: de::Error>(self, v: i64) -> std::result::Result<Unique, E> {
                Ok(Unique(v.into()))
            }
            fn visit_u64<E: de::Error>(self, v: u64) -> std::result::Result<Unique, E> {
                Ok(Unique(v.into()))
            }
            fn visit_f64<E: de::Error>(self, v: f64) -> std::result::Result<Unique, E> {
                serde_json::Number::from_f64(v)
                    .map(|n| Unique(Value::Number(n)))
                    .ok_or_else(|| E::custom("nonfinite JSON number"))
            }
            fn visit_str<E: de::Error>(self, v: &str) -> std::result::Result<Unique, E> {
                Ok(Unique(v.into()))
            }
            fn visit_unit<E: de::Error>(self) -> std::result::Result<Unique, E> {
                Ok(Unique(Value::Null))
            }
            fn visit_seq<A: SeqAccess<'de>>(
                self,
                mut seq: A,
            ) -> std::result::Result<Unique, A::Error> {
                let mut values = Vec::new();
                while let Some(Unique(v)) = seq.next_element()? {
                    values.push(v);
                }
                Ok(Unique(Value::Array(values)))
            }
            fn visit_map<A: MapAccess<'de>>(
                self,
                mut map: A,
            ) -> std::result::Result<Unique, A::Error> {
                let mut values = serde_json::Map::new();
                while let Some((key, Unique(value))) = map.next_entry::<String, Unique>()? {
                    if values.insert(key.clone(), value).is_some() {
                        return Err(de::Error::custom(format!("duplicate JSON key: {key}")));
                    }
                }
                Ok(Unique(Value::Object(values)))
            }
        }
        deserializer.deserialize_any(Json)
    }
}

fn parse<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> Result<T> {
    let Unique(value) = serde_json::from_slice(bytes)?;
    Ok(serde_json::from_value(value)?)
}

fn bytes32(value: &str) -> std::result::Result<[u8; 32], String> {
    let mut out = [0; 32];
    let value = value.strip_prefix("0x").ok_or("expected 0x-prefixed bytes32")?;
    hex::decode_to_slice(value, &mut out).map_err(|_| "expected 0x-prefixed bytes32")?;
    Ok(out)
}

fn state(state: &State) -> Result<Vec<u8>> {
    let mut out = bytes32(&state.block_hash).map_err(anyhow::Error::msg)?.to_vec();
    out.extend(bytes32(&state.send_root).map_err(anyhow::Error::msg)?);
    out.extend(state.batch.to_le_bytes());
    out.extend(state.pos_in_batch.to_le_bytes());
    Ok(out)
}

fn keccak(bytes: &[u8]) -> [u8; 32] {
    let mut out = [0; 32];
    let mut hash = Keccak::v256();
    hash.update(bytes);
    hash.finalize(&mut out);
    out
}

fn hash_parts(parts: &[&[u8]]) -> [u8; 32] {
    keccak(&parts.concat())
}

fn blob(out: &mut Vec<u8>, bytes: &[u8]) -> Result<()> {
    let len = u32::try_from(bytes.len()).context("message/preimage exceeds u32 length")?;
    out.extend(len.to_le_bytes());
    out.extend(bytes);
    Ok(())
}

fn encode(
    request: &Recording,
    wasm: &[u8],
    extra: &[Message],
    seq_before: [u8; 32],
    delayed_before: [u8; 32],
) -> Result<(Vec<u8>, Vec<u8>)> {
    // shortcut: one recorded transition from one batch, extend with observed multi-batch recordings.
    anyhow::ensure!(
        request.has_delayed_msg != request.delayed_msg_b64.is_empty(),
        "HasDelayedMsg must agree with the recorded delayed message"
    );
    let start = state(&request.start_state)?;
    let end = state(&request.expected_end_state)?;
    let seq_start = request.start_state.batch;
    let seq_end = seq_start.checked_add(1).context("sequencer count overflow")?;
    let end_batch = request.expected_end_state.batch;
    let end_position = request.expected_end_state.pos_in_batch;
    anyhow::ensure!(
        (end_batch == seq_start
            && request.start_state.pos_in_batch.checked_add(1) == Some(end_position))
            || (end_batch == seq_end && end_position == 0),
        "expected a forward transition within one recorded batch"
    );
    anyhow::ensure!(
        request.batch_info.len() == 1 && request.batch_info[0].number == seq_start,
        "expected exactly the starting sequencer batch"
    );
    let message = STANDARD.decode(&request.batch_info[0].data_b64)?;
    anyhow::ensure!(message.len() >= 40, "short sequencer header");
    let checkpoint = u64::from_be_bytes(message[32..40].try_into()?);
    let delayed_start = if request.has_delayed_msg {
        request.delayed_msg_nr.context("missing DelayedMsgNr")?
    } else {
        checkpoint
    };
    let delayed_end = delayed_start
        .checked_add(u64::from(request.has_delayed_msg))
        .context("delayed count overflow")?;
    anyhow::ensure!(
        checkpoint >= delayed_end,
        "batch checkpoint does not cover the consumed delayed message"
    );
    anyhow::ensure!(
        checkpoint - delayed_start <= 1024,
        "delayed checkpoint evidence exceeds the message bound"
    );
    let mut delayed = Vec::new();
    if request.has_delayed_msg {
        delayed.push((delayed_start, STANDARD.decode(&request.delayed_msg_b64)?));
    }
    for entry in extra {
        delayed.push((entry.number, STANDARD.decode(&entry.data_b64)?));
    }
    delayed.sort();
    let mut next = delayed_start;
    for (number, data) in &delayed {
        anyhow::ensure!(
            *number == next && *number < checkpoint,
            "expected delayed checkpoint message {next}, got {number}"
        );
        anyhow::ensure!(data.len() >= 113, "short delayed message {number}");
        next += 1;
    }
    anyhow::ensure!(
        next == checkpoint,
        "missing delayed checkpoint messages [{next}, {checkpoint})"
    );
    let mut preimages = Vec::new();
    for (raw_type, entries) in &request.preimages_b64 {
        let ty: u8 = raw_type.parse().context("preimage type must fit u8")?;
        for (raw_key, raw_value) in entries {
            let key = STANDARD.decode(raw_key)?;
            anyhow::ensure!(key.len() == 32, "preimage key must contain 32 bytes");
            preimages.push((ty, key, STANDARD.decode(raw_value)?));
        }
    }
    preimages.sort();
    anyhow::ensure!(
        preimages.len() <= 8192
            && preimages.windows(2).all(|p| (p[0].0, &p[0].1) != (p[1].0, &p[1].1)),
        "duplicate preimage or preimage bound exceeded"
    );
    let mut payload = b"NZKWIT01".to_vec();
    payload.extend(&start);
    payload.extend(seq_before);
    payload.extend(delayed_before);
    payload.extend(seq_start.to_le_bytes());
    payload.extend(delayed_start.to_le_bytes());
    payload.extend((preimages.len() as u32).to_le_bytes());
    payload.extend(1u32.to_le_bytes());
    payload.extend((delayed.len() as u32).to_le_bytes());
    for (ty, key, value) in preimages {
        payload.push(ty);
        payload.extend(key);
        blob(&mut payload, &value)?;
    }
    payload.extend(seq_start.to_le_bytes());
    blob(&mut payload, &message)?;
    let mut checkpoint_acc = delayed_before;
    let mut consumed_acc = delayed_before;
    for (number, data) in delayed {
        payload.extend(number.to_le_bytes());
        blob(&mut payload, &data)?;
        let message_hash = hash_parts(&[&data[..1], &data[13..113], &keccak(&data[113..])]);
        checkpoint_acc = hash_parts(&[&checkpoint_acc, &message_hash]);
        if number < delayed_end {
            consumed_acc = checkpoint_acc;
        }
    }
    anyhow::ensure!(payload.len() <= 64 * 1024 * 1024, "witness exceeds 64 MiB");
    let seq_acc = hash_parts(&[
        &seq_before,
        &keccak(&message),
        if checkpoint == 0 { &[0; 32] } else { &checkpoint_acc },
    ]);
    let seq_claim = hash_parts(&[
        b"ARBZSEQ1",
        &start,
        &end,
        &seq_before,
        &seq_acc,
        &seq_start.to_le_bytes(),
        &seq_end.to_le_bytes(),
    ]);
    let delayed_claim = hash_parts(&[
        b"ARBZDEL1",
        &start,
        &end,
        &delayed_before,
        &consumed_acc,
        &delayed_start.to_le_bytes(),
        &delayed_end.to_le_bytes(),
    ]);
    let mut expected = Sha256::digest(wasm).to_vec();
    expected.extend(start);
    expected.extend(end);
    expected.extend(seq_claim);
    expected.extend(delayed_claim);
    Ok((payload, expected))
}

fn frame(payload: &[u8]) -> Vec<u8> {
    let mut out = (payload.len() as u64).to_le_bytes().to_vec();
    out.extend(payload);
    out.resize(out.len().next_multiple_of(8), 0);
    out
}

pub fn prepare(args: Args) -> Result<()> {
    let request = parse(&fs::read(args.block_file)?)?;
    let extra: Vec<Message> = match args.checkpoint_messages {
        Some(path) => parse(&fs::read(path)?)?,
        None => Vec::new(),
    };
    let (payload, expected) = encode(
        &request,
        &fs::read(args.replay_wasm)?,
        &extra,
        args.sequencer_before,
        args.delayed_before,
    )?;
    for path in [&args.output, &args.expected] {
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            fs::create_dir_all(parent)?;
        }
    }
    fs::write(&args.output, frame(&payload))?;
    fs::write(&args.expected, expected)?;
    println!(
        "wrote {} ({} witness bytes) and {} (256 bytes)",
        args.output.display(),
        payload.len(),
        args.expected.display()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn fixture() -> Recording {
        let state = json!({"BlockHash": format!("0x{}", "00".repeat(32)), "SendRoot": format!("0x{}", "00".repeat(32)), "Batch": 0, "PosInBatch": 0});
        let mut end = state.clone();
        end["Batch"] = 1.into();
        serde_json::from_value(json!({"StartState": state, "ExpectedEndState": end,
            "PreimagesB64": {"0": {STANDARD.encode([b'a'; 32]): "YWJj"}},
            "BatchInfo": [{"Number": 0, "DataB64": STANDARD.encode([0; 40])}],
            "HasDelayedMsg": false, "DelayedMsgB64": "", "UserWasms": {"wasm": {}}}))
        .unwrap()
    }
    fn encoded(r: &Recording) -> Result<(Vec<u8>, Vec<u8>)> {
        encode(r, b"wasm", &[], [0; 32], [0; 32])
    }
    #[test]
    fn witness_layout_and_expected_state_are_separate() {
        let mut r = fixture();
        let (payload, expected) = encoded(&r).unwrap();
        assert_eq!(&payload[..8], b"NZKWIT01");
        assert_eq!(payload.len(), 180 + 37 + 3 + 12 + 40);
        assert_eq!(&payload[168..180], &[1, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(expected.len(), 256);
        assert_eq!(&expected[..32], Sha256::digest(b"wasm").as_slice());
        let framed = frame(&payload);
        assert_eq!(framed.len() % 8, 0);
        assert_eq!(u64::from_le_bytes(framed[..8].try_into().unwrap()), payload.len() as u64);
        r.expected_end_state.block_hash = format!("0x{}", "01".repeat(32));
        let altered = encoded(&r).unwrap();
        assert_eq!(altered.0, payload);
        assert_ne!(altered.1, expected);
    }
    #[test]
    fn partial_batch_and_checkpoint_evidence_keep_consumption_separate() {
        let mut r = fixture();
        r.start_state.pos_in_batch = 1;
        r.expected_end_state.batch = 0;
        r.expected_end_state.pos_in_batch = 2;
        assert!(encoded(&r).is_ok());
        r.has_delayed_msg = true;
        r.delayed_msg_nr = Some(0);
        let mut delayed = vec![0; 113];
        delayed.extend(b"message");
        r.delayed_msg_b64 = STANDARD.encode(&delayed);
        let mut batch = vec![0; 32];
        batch.extend(2u64.to_be_bytes());
        r.batch_info[0].data_b64 = STANDARD.encode(batch);
        let mut extra = vec![Message {
            number: 1,
            data_b64: STANDARD.encode([delayed.as_slice(), b"suffix"].concat()),
        }];
        let (payload, expected) = encode(&r, b"wasm", &extra, [0; 32], [0; 32]).unwrap();
        assert_eq!(&payload[176..180], &2u32.to_le_bytes());
        extra[0].data_b64 = STANDARD.encode([delayed.as_slice(), b"changed"].concat());
        let changed = encode(&r, b"wasm", &extra, [0; 32], [0; 32]).unwrap().1;
        assert_ne!(expected[192..224], changed[192..224]);
        assert_eq!(expected[224..], changed[224..]);
        let (anchored, claim) = encode(&r, b"wasm", &extra, [b's'; 32], [b'd'; 32]).unwrap();
        assert_eq!(&anchored[88..120], &[b's'; 32]);
        assert_eq!(&anchored[120..152], &[b'd'; 32]);
        assert_eq!(claim[..192], expected[..192]);
        assert_ne!(claim[192..224], expected[192..224]);
        assert_ne!(claim[224..], expected[224..]);
        for invalid in [
            vec![],
            vec![Message { number: 2, ..extra[0].clone() }],
            vec![extra[0].clone(), extra[0].clone()],
            vec![Message { number: 1, data_b64: STANDARD.encode([0; 112]) }],
        ] {
            assert!(encode(&r, b"wasm", &invalid, [0; 32], [0; 32]).is_err());
        }
    }
    #[test]
    fn rejects_invalid_encodings_duplicates_and_overflow() {
        for s in [
            format!("0x{}  ", "00".repeat(31)),
            format!("0x{}", "00".repeat(31)),
            format!("0x{}", "gg".repeat(32)),
        ] {
            assert!(bytes32(&s).is_err());
        }
        for json in [r#"{"x":1,"x":2}"#, r#"{"UserWasms":{"x":1,"x":2}}"#] {
            assert!(parse::<Value>(json.as_bytes()).is_err());
        }
        let original = fixture();
        let mut bad = original.clone();
        bad.has_delayed_msg = true;
        assert!(encoded(&bad).is_err());
        bad = original.clone();
        bad.batch_info.clear();
        assert!(encoded(&bad).is_err());
        bad = original.clone();
        bad.batch_info[0].data_b64 = "AA==".into();
        assert!(encoded(&bad).is_err());
        bad = original.clone();
        bad.batch_info[0].data_b64 = "not-base64".into();
        assert!(encoded(&bad).is_err());
        bad = original.clone();
        bad.expected_end_state.batch = 2;
        assert!(encoded(&bad).is_err());
        bad = original.clone();
        bad.start_state.batch = u64::MAX;
        assert!(encoded(&bad).is_err());
        bad = original.clone();
        bad.expected_end_state.batch = 0;
        bad.expected_end_state.pos_in_batch = 2;
        assert!(encoded(&bad).is_err());
        bad = original.clone();
        bad.preimages_b64.insert("00".into(), bad.preimages_b64["0"].clone());
        assert!(encoded(&bad).is_err());
        bad = original;
        bad.preimages_b64.insert("256".into(), BTreeMap::new());
        assert!(encoded(&bad).is_err());
    }
    #[test]
    fn keccak_matches_known_vectors() {
        assert_eq!(
            hex::encode(keccak(b"")),
            "c5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470"
        );
        assert_eq!(
            hex::encode(keccak(b"abc")),
            "4e03657aea45a94fc7d47ba826c8d667c0d1e6e33a64a036ec44f58fa12d6c45"
        );
    }
}
