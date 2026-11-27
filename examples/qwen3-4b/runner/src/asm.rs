//! Runs the stages on the assembly emulator, `ziskemuasm`, built for the guest ELF, instead of
//! ziskemu. One server in fast mode (`--gen=0`) serves every stage: for each one a client copies
//! the input into the server's input shared memory and asks for an emulation, and the server
//! writes the guest's output buffer to a fixed file (`--dump-guest-buffer`), which is then moved
//! to the stage's capture. The steps and the emulation time come from the server's metrics
//! (`-m`); the assembly emulator has no cost model, so there is no cost.
use anyhow::{bail, ensure, Context, Result};
use std::{
    fs::{self, File},
    io::{Read, Seek, SeekFrom},
    net::{Ipv4Addr, TcpListener, TcpStream},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant},
};

/// How long the server may take to map its memory and start listening.
const STARTUP_TIMEOUT: Duration = Duration::from_secs(300);

pub struct AsmServer {
    binary: PathBuf,
    child: Child,
    port: u16,
    prefix: String,
    /// Where the server writes the guest buffer after each emulation.
    capture: PathBuf,
    log: PathBuf,
    /// How much of the server's log the previous stages have consumed.
    log_offset: u64,
    /// Any input file: the client reads one even when it only asks the server to shut down.
    shutdown_input: PathBuf,
}

impl AsmServer {
    pub fn start(binary: &Path, output: &Path, shutdown_input: &Path) -> Result<Self> {
        // A free port, and a prefix of shared memories and semaphores no other server uses
        let port = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))?.local_addr()?.port();
        let prefix = format!("QWEN{}", std::process::id());
        let capture = output.join("asm-capture.bin");
        let log = output.join("asm-server.log");
        let log_file = File::create(&log)?;
        let child = Command::new(binary)
            .args(["-s", "--gen=0", "-m", "--shm_prefix", &prefix, "-p", &port.to_string()])
            .arg("--dump-guest-buffer")
            .arg(&capture)
            .stdout(log_file.try_clone()?)
            .stderr(log_file)
            .spawn()
            .with_context(|| format!("starting {}", binary.display()))?;
        let mut server = Self {
            binary: binary.into(),
            child,
            port,
            prefix,
            capture,
            log,
            log_offset: 0,
            shutdown_input: shutdown_input.into(),
        };
        server.wait_until_listening()?;
        Ok(server)
    }

    /// The server's log is block buffered, so readiness is checked by connecting: the server
    /// closes a connection that sends no request and waits for the next one.
    fn wait_until_listening(&mut self) -> Result<()> {
        let start = Instant::now();
        loop {
            if TcpStream::connect((Ipv4Addr::LOCALHOST, self.port)).is_ok() {
                return Ok(());
            }
            if let Some(status) = self.child.try_wait()? {
                bail!("ziskemuasm server exited with {status}; see {}", self.log.display());
            }
            ensure!(
                start.elapsed() < STARTUP_TIMEOUT,
                "ziskemuasm server not listening after {STARTUP_TIMEOUT:?}; see {}",
                self.log.display()
            );
            thread::sleep(Duration::from_millis(100));
        }
    }

    fn client(&self, input: &Path) -> Command {
        let mut command = Command::new(&self.binary);
        command
            .args(["-c", "--gen=0", "--shm_prefix", &self.prefix, "-p", &self.port.to_string()])
            .arg("-i")
            .arg(input);
        command
    }

    /// Runs one stage: `input` in, the guest buffer to `capture`, its client's output to `log`.
    pub fn execute(
        &mut self,
        name: &str,
        input: &Path,
        capture: &Path,
        log: &Path,
    ) -> Result<Metrics> {
        if self.capture.exists() {
            fs::remove_file(&self.capture)?;
        }
        let log_file = File::create(log)?;
        let status = self
            .client(input)
            .stdout(log_file.try_clone()?)
            .stderr(log_file)
            .status()
            .with_context(|| format!("starting ziskemuasm client for {name}"))?;
        let server_log = self.new_server_log()?;
        ensure!(
            status.success(),
            "{name}: ziskemuasm client exited with {status}; see {} and {}",
            log.display(),
            self.log.display()
        );
        ensure!(
            self.capture.exists(),
            "{name}: the server wrote no guest buffer; see {}",
            self.log.display()
        );
        fs::rename(&self.capture, capture)?;
        metrics(&server_log).with_context(|| format!("{name}: metrics in {}", self.log.display()))
    }

    /// The server's log written since the previous stage.
    fn new_server_log(&mut self) -> Result<String> {
        let mut file = File::open(&self.log)?;
        file.seek(SeekFrom::Start(self.log_offset))?;
        let mut text = String::new();
        self.log_offset += file.read_to_string(&mut text)? as u64;
        Ok(text)
    }

    fn remove_shared_memory(&self) {
        let Ok(entries) = fs::read_dir("/dev/shm") else { return };
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if name.starts_with(&format!("{}_", self.prefix))
                || name.starts_with(&format!("sem.{}_", self.prefix))
            {
                let _ = fs::remove_file(entry.path());
            }
        }
    }
}

impl Drop for AsmServer {
    /// Asks the server to shut down, without an emulation (`--mt 0`), and kills it if it does
    /// not. Then removes what it leaves in /dev/shm under its prefix: even after a shutdown, its
    /// control memories and a semaphore.
    fn drop(&mut self) {
        let _ = self
            .client(&self.shutdown_input)
            .args(["--mt", "0", "--shutdown"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        let start = Instant::now();
        while !matches!(self.child.try_wait(), Ok(Some(_))) {
            if start.elapsed() > Duration::from_secs(30) {
                let _ = self.child.kill();
                let _ = self.child.wait();
                break;
            }
            thread::sleep(Duration::from_millis(100));
        }
        self.remove_shared_memory();
    }
}

/// One emulation's metrics, from the server's line
/// `Duration = <us> us, ..., steps = <steps>, step duration = ..., tp = ... steps/s, ...`.
pub struct Metrics {
    pub steps: u64,
    /// The emulation alone, without loading the input or the client's round trip.
    pub duration_us: u64,
}

fn metrics(log: &str) -> Result<Metrics> {
    let mut found = None;
    for line in log.lines().filter(|line| line.contains("] Duration = ")) {
        ensure!(found.is_none(), "more than one emulation");
        let field = |name: &str| -> Result<u64> {
            let (_, rest) = line.split_once(name).with_context(|| format!("no {name:?}"))?;
            let digits = rest.split([',', ' ']).next().unwrap_or_default();
            digits.parse().with_context(|| format!("{name:?} in {line:?}"))
        };
        found = Some(Metrics { steps: field(" steps = ")?, duration_us: field("] Duration = ")? });
    }
    found.context("no metrics line")
}

#[cfg(test)]
mod tests {
    use super::metrics;

    #[test]
    fn parses_the_metrics_line() {
        let log =
            "[ASM QWEN1107508_FT 2026-11-26 17:07:29.549368] Waiting for incoming connections \
            to port 33489...\n[ASM QWEN1107508_FT 2026-11-26 17:07:46.114518] Duration = 16136595 \
            us, realloc counter = 0, wait prec counter = 0, wait input counter = 0, steps = \
            1486414551, step duration = 10 ns, tp = 92114510 steps/s, trace size = 0x0 - 0x0 = 0 \
            B(0% of 6442450944), end=1, error=0, max steps=68719476736, chunk size=262144, \
            prec_written=0, prec_read=0\n";
        let m = metrics(log).unwrap();
        assert_eq!((m.steps, m.duration_us), (1486414551, 16136595));
        assert!(metrics("no metrics here\n").is_err());
        assert!(metrics(&format!("{log}{log}")).is_err());
    }
}
