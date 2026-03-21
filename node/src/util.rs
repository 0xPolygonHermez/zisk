pub(crate) fn ms_to_timestamp(ms: u64) -> prost_types::Timestamp {
    prost_types::Timestamp { seconds: (ms / 1000) as i64, nanos: ((ms % 1000) * 1_000_000) as i32 }
}

pub(crate) fn timestamp_to_ms(ts: Option<prost_types::Timestamp>) -> Option<u64> {
    ts.map(|t| (t.seconds as u64) * 1000 + (t.nanos as u64) / 1_000_000)
}
