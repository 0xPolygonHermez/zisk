//! Starting the services' processes.
//!
//! A service asks to be killed when its parent dies (`PR_SET_PDEATHSIG`, set by
//! `build_service_command`), so that none outlives a crashed parent with its segments locked in
//! memory. On Linux its parent is the thread that started it, not the process: a service started
//! from a thread that later exits is killed with that thread, though the process lives on.
//! Programs are set up from such threads, for instance the tokio blocking-pool threads the
//! distributed worker runs setups on, which exit after 10 s idle. So every service is started
//! from one thread that lives as long as the process.

use std::io;
use std::process::{Child, Command};
use std::sync::{mpsc, OnceLock};
use std::thread;

type SpawnRequest = (Command, mpsc::Sender<io::Result<Child>>);

/// Start `command` from the spawner thread, which never exits.
pub(super) fn spawn(command: Command) -> io::Result<Child> {
    let gone = || io::Error::other("the thread that starts the ASM services is gone");
    let (reply, response) = mpsc::channel();
    spawner()?.send((command, reply)).map_err(|_| gone())?;
    response.recv().map_err(|_| gone())?
}

/// The spawner thread's queue, starting the thread on first use.
fn spawner() -> io::Result<&'static mpsc::Sender<SpawnRequest>> {
    static SPAWNER: OnceLock<mpsc::Sender<SpawnRequest>> = OnceLock::new();
    if let Some(spawner) = SPAWNER.get() {
        return Ok(spawner);
    }
    let (requests, queue) = mpsc::channel::<SpawnRequest>();
    thread::Builder::new().name("asm-svc-spawner".into()).spawn(move || {
        // Its sender lives in a static, so the queue, and this thread, end with the process.
        for (mut command, reply) in queue {
            // A service nobody waits for any more would run unseen: stop it.
            if let Err(mpsc::SendError(Ok(mut child))) = reply.send(command.spawn()) {
                let _ = child.kill();
                let _ = child.wait();
            }
        }
    })?;
    // Of two first callers racing here, the loser's queue loses its only sender, so its thread
    // ends having started nothing.
    Ok(SPAWNER.get_or_init(|| requests))
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;
    use std::os::unix::process::{CommandExt, ExitStatusExt};
    use std::process::ExitStatus;
    use std::time::{Duration, Instant};

    /// A process that asks to be killed with its parent, as every service does.
    fn dies_with_its_parent() -> Command {
        let mut command = Command::new("sh");
        command.args(["-c", "exec sleep 60"]);
        unsafe {
            command.pre_exec(|| {
                libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL);
                Ok(())
            });
        }
        command
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
            thread::sleep(Duration::from_millis(10));
        }
    }

    /// Started from a thread that then exits, as a setup on a tokio blocking-pool thread is, a
    /// service must outlive that thread, and does only when the spawner starts it.
    #[test]
    fn a_service_outlives_the_thread_that_asked_for_it() {
        let mut direct = thread::spawn(|| dies_with_its_parent().spawn()).join().unwrap().unwrap();
        let mut spawned = thread::spawn(|| spawn(dies_with_its_parent())).join().unwrap().unwrap();

        let killed = exit_within(&mut direct, Duration::from_secs(5));
        assert_eq!(
            killed.and_then(|status| status.signal()),
            Some(libc::SIGKILL),
            "a process started from a thread is killed when that thread exits"
        );
        let survived = exit_within(&mut spawned, Duration::from_millis(200)).is_none();
        let _ = spawned.kill();
        let _ = spawned.wait();
        assert!(survived, "a service started by the spawner outlives the thread that asked for it");
    }
}
