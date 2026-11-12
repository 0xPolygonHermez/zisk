//! A requested stop: SIGTERM, SIGINT, or SIGUSR1, the one `mpirun` passes on.
//!
//! `mpirun` cannot pass SIGTERM on: it takes one as an abort, sends the ranks SIGTERM itself
//! and SIGKILL a second later, and exits with status 1 whatever they do. SIGUSR1 it forwards
//! to every rank, leaving the rest to them; with `orte_allowed_exit_without_sync` set, it then
//! exits with status 0 once they all have. See the worker's systemd unit.
//!
//! Every rank stops on its own, with no MPI call: the service manager signals every process
//! of the unit and `mpirun` every rank, so none needs another to tell it, and a rank waiting
//! on a peer that has already exited, in a collective or in the MPI teardown, would wait for
//! ever. A rank cancels its job in flight, kills its ASM services, unlinks their shared
//! memory and semaphores, and exits with status 0 there and then (`_exit`), skipping its
//! destructors and `atexit` handlers: MPI's teardown waits for every rank, and the threads
//! of a cancelled job may still be running.
//!
//! It runs on a thread of its own, since a peer rank's main thread waits in an MPI receive
//! that only rank 0's next broadcast ends.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;
use std::time::Duration;

use signal_hook::consts::{SIGINT, SIGTERM, SIGUSR1};
use signal_hook::iterator::Signals;
use tracing::{info, warn};

/// How long a stop waits for the ASM services it kills to exit.
const SERVICES_EXIT_TIMEOUT: Duration = Duration::from_secs(1);

/// Past this, a stop exits with whatever it has left undone; the next start unlinks any
/// shared memory left behind.
const STOP_TIMEOUT: Duration = Duration::from_secs(5);

static STOPPING: AtomicBool = AtomicBool::new(false);
static CANCEL: OnceLock<Box<dyn Fn() + Send + Sync>> = OnceLock::new();

/// Stop the process on SIGTERM, SIGINT or SIGUSR1; see the module documentation.
///
/// Call it first thing, so that a stop while the prover starts, which takes minutes, goes the
/// same way.
pub fn stop_on_signals() -> std::io::Result<()> {
    let mut signals = Signals::new([SIGTERM, SIGINT, SIGUSR1])?;
    std::thread::Builder::new().name("stop-on-signal".into()).spawn(move || {
        // Only the first: a later one, as `mpirun` sends after the service manager's, would
        // find the stop already under way.
        if let Some(signal) = signals.forever().next() {
            stop(match signal {
                SIGINT => "SIGINT",
                SIGUSR1 => "SIGUSR1",
                _ => "SIGTERM",
            })
        }
    })?;
    Ok(())
}

/// Have a stop call `cancel` before anything else, to cancel the job in flight. Set once.
pub(crate) fn cancel_on_stop(cancel: impl Fn() + Send + Sync + 'static) {
    if CANCEL.set(Box::new(cancel)).is_err() {
        warn!("A stop already has its cancellation; ignoring another");
    }
}

/// Whether a stop has begun.
pub(crate) fn is_stopping() -> bool {
    STOPPING.load(Ordering::SeqCst)
}

/// Wait for the stop under way to end the process.
pub(crate) fn wait_for_exit() -> ! {
    loop {
        std::thread::park();
    }
}

fn stop(signal: &str) -> ! {
    STOPPING.store(true, Ordering::SeqCst);
    info!("Received {signal}; stopping");

    let deadline = std::thread::Builder::new().name("stop-deadline".into()).spawn(|| {
        std::thread::sleep(STOP_TIMEOUT);
        // On stderr, unbuffered: the process ends right after.
        eprintln!("zisk-worker: the stop did not complete within {STOP_TIMEOUT:?}; exiting");
        exit_now()
    });
    if let Err(e) = deadline {
        warn!("Failed to bound the stop by {STOP_TIMEOUT:?}: {e}");
    }

    if let Some(cancel) = CANCEL.get() {
        cancel();
    }
    zisk_asm_runner::AsmServices::release_all_for_exit(SERVICES_EXIT_TIMEOUT);
    info!("Stopped");
    exit_now()
}

/// Exit with status 0 at once; see the module documentation.
fn exit_now() -> ! {
    unsafe { libc::_exit(0) }
}

#[cfg(all(test, target_os = "linux", target_arch = "x86_64"))]
mod tests {
    use super::*;
    use std::ffi::CString;
    use std::io::{BufRead, BufReader};
    use std::process::{Child, Command, ExitStatus, Stdio};
    use std::time::Instant;

    /// Set in the child process of `a_signal_stops_the_process_with_status_0`.
    const STOP_CHILD: &str = "ZISK_WORKER_TEST_STOP_CHILD";

    fn shm_exists(name: &str) -> bool {
        let name = CString::new(name).unwrap();
        let fd = unsafe { libc::shm_open(name.as_ptr(), libc::O_RDONLY, 0) };
        if fd >= 0 {
            unsafe { libc::close(fd) };
        }
        fd >= 0
    }

    /// Whether `pid` is gone, or a zombie waiting for its new parent to reap it.
    fn is_dead(pid: u32) -> bool {
        match std::fs::read_to_string(format!("/proc/{pid}/stat")) {
            Ok(stat) => {
                stat.rsplit_once(')').is_some_and(|(_, rest)| rest.trim_start().starts_with('Z'))
            }
            Err(_) => true,
        }
    }

    fn exit_within(child: &mut Child, timeout: Duration) -> Option<ExitStatus> {
        let deadline = Instant::now() + timeout;
        loop {
            if let Some(status) = child.try_wait().unwrap() {
                return Some(status);
            }
            if Instant::now() >= deadline {
                return None;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    /// The child half of `a_signal_stops_the_process_with_status_0`, in a process of its own
    /// since a stop ends it. Does nothing unless that test runs it.
    ///
    /// Wired as the worker: a stop that cancels a job, whose failure then tries to abort the
    /// process as `abort_wedged_worker` would; an ASM service with a segment of its; and a
    /// main thread that never returns, as a peer rank's waiting in an MPI receive.
    #[test]
    fn stop_child() {
        if std::env::var_os(STOP_CHILD).is_none() {
            return;
        }
        stop_on_signals().unwrap();
        cancel_on_stop(|| {
            println!("cancelled");
            std::thread::spawn(|| {
                if is_stopping() {
                    wait_for_exit();
                }
                std::process::exit(75)
            });
        });

        let shm_prefix = format!("ZISK_{}_0_h0", std::process::id());
        let mut service = Command::new("sh")
            .args(["-c", "read line", "sh", "--shm_prefix", &shm_prefix])
            .stdin(Stdio::piped())
            .spawn()
            .unwrap();
        // Held open, or the stand-in reads its end and exits.
        let _stdin = service.stdin.take();
        let service_pid = service.id();
        // Waited on by its owner, as a service is by its handle.
        std::thread::spawn(move || service.wait());
        let segment = CString::new(format!("{shm_prefix}_input")).unwrap();
        let fd = unsafe { libc::shm_open(segment.as_ptr(), libc::O_CREAT | libc::O_RDWR, 0o600) };
        assert!(fd >= 0, "create the stand-in segment");
        unsafe { libc::close(fd) };

        println!("service {service_pid}");
        println!("ready");
        wait_for_exit()
    }

    /// Every stop signal ends the process with status 0, having cancelled its job,
    /// killed its services and unlinked their segments, whatever its other threads do.
    #[test]
    fn a_signal_stops_the_process_with_status_0() {
        for signal in [SIGTERM, SIGINT, SIGUSR1] {
            let mut child = Command::new(std::env::current_exe().unwrap())
                .args(["--exact", "stop::tests::stop_child", "--nocapture"])
                .env(STOP_CHILD, "1")
                .stdout(Stdio::piped())
                .spawn()
                .unwrap();
            let mut stdout = BufReader::new(child.stdout.take().unwrap()).lines();
            let mut service = None;
            for line in stdout.by_ref() {
                let line = line.unwrap();
                if let Some(pid) = line.strip_prefix("service ") {
                    service = pid.parse::<u32>().ok();
                }
                if line == "ready" {
                    break;
                }
            }
            let service = service.expect("the child reports its service");
            let segment = format!("ZISK_{}_0_h0_input", child.id());

            unsafe { libc::kill(child.id() as libc::pid_t, signal) };
            let status = exit_within(&mut child, Duration::from_secs(10));
            if status.is_none() {
                let _ = child.kill();
                let _ = child.wait();
            }
            let rest: Vec<String> = stdout.map_while(Result::ok).collect();
            let deadline = Instant::now() + Duration::from_secs(5);
            while !is_dead(service) && Instant::now() < deadline {
                std::thread::sleep(Duration::from_millis(10));
            }

            assert_eq!(status.and_then(|s| s.code()), Some(0), "signal {signal}: {status:?}");
            assert!(rest.iter().any(|line| line == "cancelled"), "the job is cancelled: {rest:?}");
            assert!(is_dead(service), "the service is killed");
            assert!(!shm_exists(&segment), "{segment} is unlinked");
        }
    }
}
