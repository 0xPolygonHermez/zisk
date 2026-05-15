// Liveness test guest: reads two u64 records and either runs short-mode
// (return immediately) or long-mode (busy-loop ~10s on prod hardware). The
// host driver uses the modes to exercise the worker's post-failure recovery:
//   - short-mode: baseline successful execute.
//   - long-mode:  long enough that the host can cancel mid-flight and observe
//                 the worker recover for the next job.
//   - malformed second record: bincode in `ziskos::io::read::<u64>()` panics,
//                 the JIT signal handler reports a task failure, and the worker
//                 must recover for the next job.

#![no_main]
ziskos::entrypoint!(main);

fn main() {
    // Record 1: mode (0 = short, 1 = long).
    let mode: u64 = ziskos::io::read();
    // Record 2: arbitrary value carried through the run. If the host writes a
    // malformed record here, this read panics — that's the "invalid input"
    // path the recovery flow must absorb.
    let value: u64 = ziskos::io::read();

    if mode == 1 {
        // Busy-loop sized to take a few seconds on production hardware, well
        // above the host's cancel-after-2s mark. Tune `ITER` if needed.
        const ITER: u64 = 200_000_000;
        let mut acc: u64 = value;
        let mut i: u64 = 0;
        while i < ITER {
            acc = acc.wrapping_add(i.wrapping_mul(0x9E37_79B9_7F4A_7C15));
            i = i.wrapping_add(1);
        }
        println!("liveness-guest: long-mode finished, acc={acc}");
    } else {
        println!("liveness-guest: short-mode mode={mode} value={value}");
    }
}
