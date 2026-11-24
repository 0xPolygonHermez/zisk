//! The host memory this process's witness phase may use, and the margin left out of it.
//!
//! Proofman reserves its own host buffers at startup; what the witness phase allocates while a
//! proof runs comes on top. [`host_memory_share`] measures how much of the memory still available
//! this process may count on, after the [`HostMemoryMargin`] the prover was built with.

use std::str::FromStr;

use sysinfo::System;

use super::collect_budget::parse_size;

/// The least memory the default margin leaves.
const MIN_MARGIN: u64 = 2 << 30;

/// Memory left out of what the witness phase may use, for everything else that grows while a
/// proof runs. It is the host's: the processes on it share what is left.
///
/// Without one, the margin is the larger of 2 GiB and 5 % of the total memory. A margin given
/// replaces that rule entirely, with no floor. It parses from a share of the total memory (`5%`)
/// or a size (`4G`, `512M`, `64K`, plain bytes; powers of 1024). Within a cgroup whose limit is
/// below the host's memory, the total is that limit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostMemoryMargin {
    /// This percentage of the total memory; above 100 counts as 100.
    Percent(u8),
    /// This many bytes.
    Bytes(u64),
}

impl HostMemoryMargin {
    /// The bytes this margin leaves out of `total` bytes of memory.
    pub fn bytes_of(self, total: u64) -> u64 {
        match self {
            Self::Percent(percent) => total.saturating_mul(u64::from(percent.min(100))) / 100,
            Self::Bytes(bytes) => bytes,
        }
    }
}

impl FromStr for HostMemoryMargin {
    type Err = ParseHostMemoryMarginError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let invalid = || ParseHostMemoryMarginError(value.to_owned());
        match value.trim().strip_suffix('%') {
            Some(percent) => match percent.trim().parse::<u8>() {
                Ok(percent) if percent <= 100 => Ok(Self::Percent(percent)),
                _ => Err(invalid()),
            },
            None => parse_size(value).map(Self::Bytes).ok_or_else(invalid),
        }
    }
}

/// A [`HostMemoryMargin`] that does not parse.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error(
    "`{0}` is not a memory margin: give a share of the total memory, from `0%` to `100%`, or a \
     size such as `4G`, `512M` or `64K`"
)]
pub struct ParseHostMemoryMarginError(String);

/// This process's share of the host memory available now: what `margin` (the default one when
/// `None`) leaves, split among `node_processes`. `None` when the platform reports no memory
/// figures, so that no caller holds back work on a guess.
pub(crate) fn host_memory_share(
    node_processes: usize,
    margin: Option<HostMemoryMargin>,
) -> Option<u64> {
    let mut system = System::new();
    system.refresh_memory();
    let cgroup = system.cgroup_limits().map(|limits| (limits.total_memory, limits.free_memory));
    let (total, available) =
        within_cgroup(system.total_memory(), system.available_memory(), cgroup);
    reported_share(total, available, node_processes, margin)
}

/// [`process_share`] of what the platform reported, or `None` when it reported no total: it has no
/// memory figures. Nothing available is a figure, of a host or a cgroup that is full.
fn reported_share(
    total: u64,
    available: u64,
    node_processes: usize,
    margin: Option<HostMemoryMargin>,
) -> Option<u64> {
    (total > 0).then(|| process_share(total, available, node_processes, margin))
}

/// `(total, available)` for this process: the host's, bounded by the cgroup's `(limit, free)` when
/// there is one.
fn within_cgroup(total: u64, available: u64, cgroup: Option<(u64, u64)>) -> (u64, u64) {
    match cgroup {
        Some((limit, free)) => (total.min(limit), available.min(free)),
        None => (total, available),
    }
}

/// This process's part of `available` bytes, out of `total`: what `margin` (the default one when
/// `None`) leaves, split among `node_processes`.
fn process_share(
    total: u64,
    available: u64,
    node_processes: usize,
    margin: Option<HostMemoryMargin>,
) -> u64 {
    let margin = match margin {
        Some(margin) => margin.bytes_of(total),
        None => MIN_MARGIN.max(total / 20),
    };
    available.saturating_sub(margin) / node_processes.max(1) as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    const GIB: u64 = 1 << 30;

    #[test]
    fn a_margin_parses_as_a_share_or_a_size() {
        let parse = |value: &str| value.parse::<HostMemoryMargin>();
        assert_eq!(parse("5%"), Ok(HostMemoryMargin::Percent(5)));
        assert_eq!(parse(" 10 % "), Ok(HostMemoryMargin::Percent(10)));
        assert_eq!(parse("0%"), Ok(HostMemoryMargin::Percent(0)));
        assert_eq!(parse("100%"), Ok(HostMemoryMargin::Percent(100)));
        assert_eq!(parse("4G"), Ok(HostMemoryMargin::Bytes(4 << 30)));
        assert_eq!(parse("512m"), Ok(HostMemoryMargin::Bytes(512 << 20)));
        assert_eq!(parse("0"), Ok(HostMemoryMargin::Bytes(0)));
        for invalid in ["101%", "5.5%", "-1%", "%", "", "lots", "4X", "-4G"] {
            assert_eq!(
                parse(invalid),
                Err(ParseHostMemoryMarginError(invalid.to_owned())),
                "{invalid:?} is not a margin"
            );
        }
        let message = parse("120%").unwrap_err().to_string();
        assert!(message.contains("`120%`"), "the error names the value: {message}");
    }

    #[test]
    fn a_share_is_of_the_total_memory() {
        assert_eq!(HostMemoryMargin::Percent(10).bytes_of(64 * GIB), 64 * GIB / 10);
        assert_eq!(HostMemoryMargin::Percent(0).bytes_of(64 * GIB), 0);
        assert_eq!(HostMemoryMargin::Percent(100).bytes_of(1000), 1000);
        assert_eq!(HostMemoryMargin::Percent(200).bytes_of(1000), 1000, "above 100 counts as 100");
        assert_eq!(HostMemoryMargin::Bytes(GIB).bytes_of(64 * GIB), GIB);
    }

    #[test]
    fn a_margin_given_replaces_the_default_rule() {
        let share = |total, margin| process_share(total, 40 * GIB, 1, margin);
        // Default: the larger of 2 GiB and 5 % of the total.
        assert_eq!(share(16 * GIB, None), 38 * GIB, "2 GiB floor");
        assert_eq!(share(64 * GIB, None), 40 * GIB - 64 * GIB / 20, "5 %");
        // Given: as is, with no floor.
        assert_eq!(share(64 * GIB, Some(HostMemoryMargin::Percent(10))), 40 * GIB - 64 * GIB / 10);
        assert_eq!(share(64 * GIB, Some(HostMemoryMargin::Bytes(GIB))), 39 * GIB);
        assert_eq!(share(64 * GIB, Some(HostMemoryMargin::Bytes(0))), 40 * GIB);
        assert_eq!(share(64 * GIB, Some(HostMemoryMargin::Bytes(50 * GIB))), 0, "nothing left");
    }

    #[test]
    fn the_margin_is_the_hosts_and_its_rest_is_split_among_the_processes() {
        let margin = Some(HostMemoryMargin::Bytes(4 * GIB));
        assert_eq!(process_share(64 * GIB, 40 * GIB, 2, margin), 18 * GIB);
        assert_eq!(process_share(64 * GIB, 40 * GIB, 0, margin), 36 * GIB, "no processes is one");
    }

    #[test]
    fn only_a_missing_total_means_no_memory_figures() {
        assert_eq!(reported_share(0, 0, 1, None), None, "no figures");
        assert_eq!(reported_share(0, 40 * GIB, 1, None), None, "no figures");
        assert_eq!(reported_share(64 * GIB, 0, 1, None), Some(0), "full: a budget of nothing");
        assert_eq!(reported_share(64 * GIB, 40 * GIB, 1, None), Some(40 * GIB - 64 * GIB / 20));
    }

    #[test]
    fn a_cgroup_limit_bounds_the_total_and_the_available_memory() {
        // A 32 GiB container on a 512 GiB host: a share is of the container's memory.
        let (total, available) = within_cgroup(512 * GIB, 400 * GIB, Some((32 * GIB, 20 * GIB)));
        assert_eq!((total, available), (32 * GIB, 20 * GIB));
        assert_eq!(
            process_share(total, available, 1, Some(HostMemoryMargin::Percent(5))),
            20 * GIB - 32 * GIB / 20
        );
        // A cgroup without a limit reports the host's memory as its limit.
        assert_eq!(
            within_cgroup(64 * GIB, 40 * GIB, Some((64 * GIB, 50 * GIB))),
            (64 * GIB, 40 * GIB)
        );
        assert_eq!(within_cgroup(64 * GIB, 40 * GIB, None), (64 * GIB, 40 * GIB));
    }
}
