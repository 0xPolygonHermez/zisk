use anyhow::Result;
use std::{fs, path::Path, process::Command};
use yansi::Paint;
use zisk_build::ZISK_VERSION_MESSAGE;

#[derive(clap::Args)]
#[command(author, about, long_about = None, version = ZISK_VERSION_MESSAGE)]
/// Create a new project that runs inside ZisK
pub(crate) struct NewCmd {
    /// Name of the new project to create
    name: String,
}

impl NewCmd {
    pub(crate) fn run(&self) -> Result<()> {
        let root = Path::new(&self.name);
        let repo_url = "https://{}@github.com/0xPolygonHermez/zisk_template";
        // Create the root directory if it doesn't exist.
        if !root.exists() {
            fs::create_dir(&self.name)?;
        }

        // Check if ZISK_TEMPLATE_BRANCH environment variable is set, and if so, use it as the branch to clone.
        let branch = std::env::var("ZISK_TEMPLATE_BRANCH").ok();

        // Clone the repository.
        let mut cmd = Command::new("git");
        cmd.args(Self::git_clone_args(
            repo_url,
            root.as_os_str().to_str().unwrap(),
            branch.as_deref(),
        ));

        let output = cmd.output().expect("failed to execute command");

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

    /// Assemble the `git clone` argument vector. Pure: the optional `--branch`
    /// is appended only for a non-empty branch, so the env-driven branch
    /// selection can be tested without invoking git.
    fn git_clone_args(repo_url: &str, dir: &str, branch: Option<&str>) -> Vec<String> {
        let mut args = vec![
            "clone".to_string(),
            repo_url.to_string(),
            dir.to_string(),
            "--recurse-submodules".to_string(),
            "--depth=1".to_string(),
        ];
        if let Some(branch) = branch.filter(|b| !b.is_empty()) {
            args.push("--branch".to_string());
            args.push(branch.to_string());
        }
        args
    }
}

#[cfg(test)]
mod tests {
    use super::NewCmd;

    const URL: &str = "https://{}@github.com/0xPolygonHermez/zisk_template";

    #[test]
    fn clone_args_without_branch() {
        let args = NewCmd::git_clone_args(URL, "myproj", None);
        assert_eq!(args, vec!["clone", URL, "myproj", "--recurse-submodules", "--depth=1"]);
        assert!(!args.iter().any(|a| a == "--branch"));
    }

    #[test]
    fn clone_args_empty_branch_is_ignored() {
        let args = NewCmd::git_clone_args(URL, "myproj", Some(""));
        assert!(!args.iter().any(|a| a == "--branch"));
    }

    #[test]
    fn clone_args_with_branch() {
        let args = NewCmd::git_clone_args(URL, "myproj", Some("dev"));
        assert!(args.windows(2).any(|w| w == ["--branch", "dev"]));
    }
}
