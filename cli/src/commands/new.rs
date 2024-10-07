use anyhow::Result;
use clap::Parser;
use std::{fs, path::Path, process::Command};
use yansi::Paint;

#[derive(Parser)]
#[command(name = "new", about = "Setup a new project that runs inside the ZisK.")]
pub struct NewCmd {
    name: String,
}

impl NewCmd {
    pub fn run(&self) -> Result<()> {
        let root = Path::new(&self.name);
        let zisk_token = std::env::var("ZISK_TOKEN");
        let repo_url = match zisk_token {
            Ok(zisk_token) => {
                println!("Detected ZISK_TOKEN, using it to clone zisk_template");
                format!("https://{}@github.com/0xPolygonHermez/zisk_template", zisk_token)
            }
            Err(_) => {
                println!("No ZISK_TOKEN detected. If you get throttled by Github, set it to bypass the rate limit.");
                "ssh://git@github.com/0xPolygonHermez/zisk_template".to_string()
            }
        };
        // Create the root directory if it doesn't exist.
        if !root.exists() {
            fs::create_dir(&self.name)?;
        }

        // Clone the repository.
        let output = Command::new("git")
            .arg("clone")
            .arg(repo_url)
            .arg(root.as_os_str())
            .arg("--recurse-submodules")
            .arg("--depth=1")
            .output()
            .expect("failed to execute command");
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("failed to clone repository: {}", stderr));
        }

        // Remove the .git directory.
        fs::remove_dir_all(root.join(".git"))?;

        println!(
            "    \x1b[1m{}\x1b[0m {} ({})",
            Paint::green("Initialized"),
            self.name,
            std::fs::canonicalize(root).expect("failed to canonicalize").to_str().unwrap()
        );

        Ok(())
    }
}
