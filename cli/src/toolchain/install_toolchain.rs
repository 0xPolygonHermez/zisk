use crate::download_file;
use anyhow::Result;
use clap::Parser;
use dirs::home_dir;
use rand::{distr::Alphanumeric, Rng};
use reqwest::Client;
use std::{
    fs::{self},
    io::Read,
    process::Command,
};

#[cfg(target_family = "unix")]
use std::os::unix::fs::PermissionsExt;

use crate::{get_target, get_toolchain_download_url, is_supported_target, RUSTUP_TOOLCHAIN_NAME};

#[derive(Parser)]
#[command(name = "install-toolchain", about = "Install the cargo-zisk toolchain.")]
pub struct InstallToolchainCmd {
    version: Option<String>,
}

impl InstallToolchainCmd {
    pub fn run(&self) -> Result<()> {
        // Setup client.
        let client = Client::builder()
            .user_agent("Mozilla/5.0")
            .timeout(std::time::Duration::from_secs(60))
            .build()?;

        // Setup variables.
        let root_dir = home_dir().unwrap().join(".zisk");
        match fs::read_dir(&root_dir) {
            Ok(entries) =>
            {
                #[allow(clippy::manual_flatten)]
                for entry in entries {
                    if let Ok(entry) = entry {
                        let entry_path = entry.path();
                        let entry_name = entry_path.file_name().unwrap();
                        if entry_path.is_dir()
                            && entry_name != "bin"
                            && entry_name != "circuits"
                            && entry_name != "toolchains"
                            && entry_name != "provingKey"
                            && entry_name != "verifyKey"
                            && entry_name != "cache"
                            && entry_name != "zisk"
                        {
                            if let Err(err) = fs::remove_dir_all(&entry_path) {
                                println!("Failed to remove directory {entry_path:?}: {err}");
                            }
                        } else if entry_path.is_file() {
                            if let Err(err) = fs::remove_file(&entry_path) {
                                println!("Failed to remove file {entry_path:?}: {err}");
                            }
                        }
                    }
                }
            }
            Err(_) => println!("No existing ~/.zisk directory to remove."),
        }
        println!("Successfully cleaned up ~/.zisk directory.");
        match fs::create_dir_all(&root_dir) {
            Ok(_) => println!("Successfully created ~/.zisk directory."),
            Err(err) => println!("Failed to create ~/.zisk directory: {err}"),
        };

        assert!(
            is_supported_target(),
            "Unsupported architecture. Please build the toolchain from source."
        );
        let target = get_target();
        let toolchain_asset_name = format!("rust-toolchain-{target}.tar.gz");
        let toolchain_archive_path = root_dir.join(toolchain_asset_name.clone());
        let toolchain_dir = root_dir.join(&target);

        let source_toolchain_dir = std::env::var("ZISK_TOOLCHAIN_SOURCE_DIR");
        match source_toolchain_dir {
            Ok(source_toolchain_dir) => {
                // Copy the toolchain from the source directory.
                let mut source_toolchain_file = fs::canonicalize(source_toolchain_dir)?;
                source_toolchain_file.push(&toolchain_asset_name);
                fs::copy(&source_toolchain_file, &toolchain_archive_path)?;
                println!("Successfully copied toolchain from source directory.");
            }
            Err(_) => {
                // Download the toolchain.
                let rt = tokio::runtime::Runtime::new()?;

                let toolchain_download_url =
                    rt.block_on(get_toolchain_download_url(&target, &self.version));

                let mut file = fs::File::create(&toolchain_archive_path)?;
                rt.block_on(download_file(&client, toolchain_download_url.as_str(), &mut file))
                    .unwrap();
            }
        }

        // Remove the existing toolchain from rustup, if it exists.
        let mut child = Command::new("rustup")
            .current_dir(&root_dir)
            .args(["toolchain", "remove", RUSTUP_TOOLCHAIN_NAME])
            .stdout(std::process::Stdio::piped())
            .spawn()?;
        let res = child.wait();
        match res {
            Ok(_) => {
                let mut stdout = child.stdout.take().unwrap();
                let mut content = String::new();
                stdout.read_to_string(&mut content).unwrap();
                if !content.contains("no toolchain installed") {
                    println!("Successfully removed existing toolchain.");
                }
            }
            Err(_) => println!("Failed to remove existing toolchain."),
        }

        // Unpack the toolchain.
        fs::create_dir_all(toolchain_dir.clone())?;
        Command::new("tar")
            .current_dir(&root_dir)
            .args(["-xzf", &toolchain_asset_name, "-C", &toolchain_dir.to_string_lossy()])
            .status()?;

        // Move the toolchain to a randomly named directory in the 'toolchains' folder
        let toolchains_dir = root_dir.join("toolchains");
        fs::create_dir_all(&toolchains_dir)?;
        let random_string: String =
            rand::rng().sample_iter(&Alphanumeric).take(10).map(char::from).collect();
        let new_toolchain_dir = toolchains_dir.join(random_string);
        fs::rename(&toolchain_dir, &new_toolchain_dir)?;

        // Link the new toolchain directory to rustup
        Command::new("rustup")
            .current_dir(&root_dir)
            .args([
                "toolchain",
                "link",
                RUSTUP_TOOLCHAIN_NAME,
                &new_toolchain_dir.to_string_lossy(),
            ])
            .status()?;
        println!("Successfully linked toolchain to rustup.");

        // Ensure permissions.
        let bin_dir = new_toolchain_dir.join("bin");
        let rustlib_bin_dir = new_toolchain_dir.join(format!("lib/rustlib/{target}/bin"));
        for entry in fs::read_dir(bin_dir)?.chain(fs::read_dir(rustlib_bin_dir)?) {
            let entry = entry?;
            if entry.path().is_file() {
                let mut perms = entry.metadata()?.permissions();
                perms.set_mode(0o755);
                fs::set_permissions(entry.path(), perms)?;
            }
        }

        // Delete the downloaded archive.
        fs::remove_file(&toolchain_archive_path)?;

        Ok(())
    }
}
