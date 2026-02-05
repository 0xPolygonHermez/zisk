use crate::StatsCosts;

#[derive(Clone, Debug)]
pub struct StatsCostMark {
    pub start: Option<StatsCosts>,
    pub costs: Vec<StatsCosts>,
    pub values: Vec<u64>,
    pub count: u64,
    pub min_value: Option<u64>,
    pub max_value: Option<u64>,
    pub total_value: u128,
    pub arguments: Vec<u64>,
}

impl StatsCostMark {
    pub fn new() -> Self {
        Self {
            start: None,
            costs: Vec::new(),
            count: 0,
            min_value: None,
            max_value: None,
            total_value: 0,
            values: Vec::new(),
            arguments: Vec::new(),
        }
    }
}

impl Default for StatsCostMark {
    fn default() -> Self {
        Self::new()
    }
}
