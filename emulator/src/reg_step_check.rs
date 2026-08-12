//! Fast register step-distance check (`--reg-step-check`).
//!
//! Unlike the full `--reg-step-distance` report, which lives in [`crate::stats`] and needs the
//! statistics path (`-X`), this is a self-contained counter cheap enough to run on the fast
//! emulation path: it only keeps the last access step of each register and a couple of counters, so
//! a whole program can be simulated quickly just to answer "does any instance overflow?".
//!
//! Execution is split into instances of 2^`instance_bits` steps. Every instance starts with a flush
//! that accesses every register, so the distance of an access is measured from the later of the
//! previous access to that register and the start of the current instance. An instance is reported
//! as overflowing when it holds at least one such distance above the limit.

use riscv::RiscVRegisters;
use zisk_core::REGS_IN_MAIN_TOTAL_NUMBER;

/// Per-register step-distance overflow counter, aggregated per instance.
pub struct RegStepCheck {
    /// A distance strictly above this limit overflows the instance.
    limit: u64,
    /// Instance size in bits: one instance spans 2^`instance_bits` steps.
    instance_bits: u32,
    /// Step of the last access (read or write) of each register; 0 until first accessed, which is
    /// also the step the program starts at.
    last_step: [u64; REGS_IN_MAIN_TOTAL_NUMBER],
    /// Number of distinct instances holding at least one distance above the limit.
    instances_over: u64,
    /// Last instance already counted in `instances_over`, so several overflows inside the same
    /// instance are counted once. `u64::MAX` means none counted yet.
    last_instance_over: u64,
    /// Largest distance seen, to give the single output line some context.
    max_dist: u64,
    /// Per register, how many of its distances were above the limit, so a failing run can name the
    /// registers involved. Only touched when a distance overflows, which keeps the common path
    /// untouched.
    reg_over: [u64; REGS_IN_MAIN_TOTAL_NUMBER],
    /// Per register, its largest distance above the limit; 0 for registers that never overflowed.
    reg_max_dist: [u64; REGS_IN_MAIN_TOTAL_NUMBER],
    /// Per register, the last instance already counted in `reg_instances_over`.
    reg_last_instance_over: [u64; REGS_IN_MAIN_TOTAL_NUMBER],
    /// Per register, in how many distinct instances it went over the limit.
    reg_instances_over: [u64; REGS_IN_MAIN_TOTAL_NUMBER],
}

impl RegStepCheck {
    /// Creates the checker for the given distance limit and instance size in bits (clamped to a
    /// representable instance).
    pub fn new(limit: u64, instance_bits: u32) -> Self {
        Self {
            limit,
            instance_bits: instance_bits.min(63),
            last_step: [0; REGS_IN_MAIN_TOTAL_NUMBER],
            instances_over: 0,
            last_instance_over: u64::MAX,
            max_dist: 0,
            reg_over: [0; REGS_IN_MAIN_TOTAL_NUMBER],
            reg_max_dist: [0; REGS_IN_MAIN_TOTAL_NUMBER],
            reg_last_instance_over: [u64::MAX; REGS_IN_MAIN_TOTAL_NUMBER],
            reg_instances_over: [0; REGS_IN_MAIN_TOTAL_NUMBER],
        }
    }

    /// Accounts one register access at `step`. The distance is measured from the later of the
    /// previous access and the flush that opens the current instance; distances between the flush
    /// and a later flush are irrelevant here, an untouched register is always re-read by the flush.
    #[inline(always)]
    pub fn on_access(&mut self, reg: usize, step: u64) {
        debug_assert!(reg < REGS_IN_MAIN_TOTAL_NUMBER);
        let instance = step >> self.instance_bits;
        let base = self.last_step[reg].max(instance << self.instance_bits);
        self.last_step[reg] = step;

        let dist = step - base;
        if dist > self.max_dist {
            self.max_dist = dist;
        }
        if dist > self.limit {
            self.reg_over[reg] += 1;
            if dist > self.reg_max_dist[reg] {
                self.reg_max_dist[reg] = dist;
            }
            if self.reg_last_instance_over[reg] != instance {
                self.reg_last_instance_over[reg] = instance;
                self.reg_instances_over[reg] += 1;
            }
            if self.last_instance_over != instance {
                self.last_instance_over = instance;
                self.instances_over += 1;
            }
        }
    }

    /// Number of instances holding at least one distance above the limit.
    pub fn instances_over(&self) -> u64 {
        self.instances_over
    }

    /// Total number of instances an execution of `end_step` steps spans.
    pub fn total_instances(&self, end_step: u64) -> u64 {
        (end_step >> self.instance_bits) + 1
    }

    /// Registers that went over the limit, worst first: `(register, instances, distances, max
    /// distance)`. Empty when the check passed.
    pub fn registers_over(&self) -> Vec<(usize, u64, u64, u64)> {
        let mut regs: Vec<(usize, u64, u64, u64)> = (0..REGS_IN_MAIN_TOTAL_NUMBER)
            .filter(|&r| self.reg_over[r] > 0)
            .map(|r| (r, self.reg_instances_over[r], self.reg_over[r], self.reg_max_dist[r]))
            .collect();
        regs.sort_by(|a, b| b.3.cmp(&a.3).then(a.0.cmp(&b.0)));
        regs
    }

    /// Verdict of the check, for an execution that ended at `end_step`: one line, plus a second one
    /// naming the registers involved when it failed.
    pub fn report(&self, end_step: u64) -> String {
        let over = self.instances_over;
        let verdict = if over == 0 {
            format!("OK, no instance over the {} steps limit", self.limit)
        } else {
            format!("EXCEEDED in {} of {} instances", over, self.total_instances(end_step))
        };
        let mut report = format!(
            "REGISTER STEP CHECK: {verdict} (limit={} instance=2^{}={} steps max_dist={})",
            self.limit,
            self.instance_bits,
            1u64 << self.instance_bits,
            self.max_dist,
        );
        let regs = self.registers_over();
        if !regs.is_empty() {
            let detail: Vec<String> = regs
                .iter()
                .map(|&(r, instances, times, max_dist)| {
                    let name = RiscVRegisters::name_from_usize(r).unwrap_or("?");
                    format!("x{r}({name}) max={max_dist} in {instances} inst/{times} gaps")
                })
                .collect();
            report.push_str(&format!(
                "\nREGISTER STEP CHECK: registers over the limit: {}",
                detail.join(", ")
            ));
        }
        report
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_one_instance_per_overflow_and_measures_from_the_instance_start() {
        // Instances of 16 steps, distances above 5 overflow.
        let mut check = RegStepCheck::new(5, 4);

        check.on_access(1, 0); // first access, distance 0 from the program start
        check.on_access(1, 3); // distance 3
        check.on_access(1, 20); // instance 1: measured from its start (16), not from step 3 -> 4
        assert_eq!(check.instances_over(), 0);

        check.on_access(1, 40); // instance 2: 40 - 32 = 8 > 5
        assert_eq!(check.instances_over(), 1);

        check.on_access(2, 41); // 41 - 32 = 9 > 5, but the same instance is not counted twice
        assert_eq!(check.instances_over(), 1);

        check.on_access(2, 60); // instance 3: 60 - 48 = 12 > 5
        assert_eq!(check.instances_over(), 2);

        assert_eq!(check.total_instances(60), 4);
        assert!(check.report(60).starts_with("REGISTER STEP CHECK: EXCEEDED in 2 of 4 instances"));

        // Both registers overflowed once; x2 did it by a wider margin, so it is reported first.
        assert_eq!(check.registers_over(), vec![(2, 2, 2, 12), (1, 1, 1, 8)]);
    }

    #[test]
    fn registers_that_never_overflow_are_not_reported() {
        let mut check = RegStepCheck::new(5, 4);
        check.on_access(1, 2); // never overflows
        check.on_access(1, 6);
        check.on_access(3, 15); // 15 - 0 = 15 > 5, twice in two different instances
        check.on_access(3, 40); // 40 - 32 = 8 > 5
        assert_eq!(check.registers_over(), vec![(3, 2, 2, 15)]);
        assert!(check
            .report(40)
            .contains("registers over the limit: x3(gp) max=15 in 2 inst/2 gaps"));
    }

    #[test]
    fn a_distance_equal_to_the_limit_does_not_overflow() {
        let mut check = RegStepCheck::new(5, 8);
        check.on_access(1, 2); // 2 steps from the program start
        check.on_access(1, 7); // exactly the limit
        assert_eq!(check.instances_over(), 0);
        check.on_access(1, 13); // one step above it
        assert_eq!(check.instances_over(), 1);
    }

    #[test]
    fn an_instance_can_be_counted_again_after_a_quiet_one() {
        // Instance 0 and instance 2 overflow, instance 1 does not: the counter must not collapse
        // them, and must not miss the second one either.
        let mut check = RegStepCheck::new(5, 4);
        check.on_access(1, 10); // instance 0: 10 - 0 = 10 > 5
        check.on_access(1, 20); // instance 1: 20 - 16 = 4
        check.on_access(1, 46); // instance 2: 46 - 32 = 14 > 5
        assert_eq!(check.instances_over(), 2);
    }
}
