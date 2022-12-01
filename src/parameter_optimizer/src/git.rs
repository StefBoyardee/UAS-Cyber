use std::fmt;
use std::io::Read;
use std::process::Command;

pub struct RepoInfo {
    pub url: String,
    pub path: String,
    pub commit_hash: String,
}

#[derive(Debug)]
pub struct GitError {
    command: String,
    message: String,
    exit_code: std::process::ExitStatus,
}

impl GitError {
    pub fn new(
        command: String,
        message: impl Into<String>,
        exit_code: std::process::ExitStatus,
    ) -> Self {
        GitError {
            command,
            message: message.into(),
            exit_code,
        }
    }
}

impl std::error::Error for GitError {}

impl fmt::Display for GitError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Error running: {}, Exit Code: {}: {}",
            self.command, self.exit_code, self.message
        )
    }
}

pub fn setup_repo(info: &RepoInfo) -> Result<bool, crate::Error> {
    let mut needs_configure = false;
    if !std::path::Path::new(&info.path).exists() {
        println!("Cloning repo: {}", info.url);
        run_git_command(&["clone", info.url.as_str(), info.path.as_str()], "./")?;
        needs_configure = true;
    }
    let current_hash = run_git_command(&["rev-parse", "HEAD"], info.path.as_str())?;

    println!("Checkout complete!");
    if current_hash != info.commit_hash {
        println!("Hashes differ");
        let _ = run_git_command(&["checkout", info.commit_hash.as_str()], info.path.as_str())?;
        //We just checked out a new commit so reconfigure!
        Ok(true)
    } else {
        Ok(needs_configure)
    }
}

fn run_git_command(args: &[&str], current_dir: &str) -> Result<String, crate::Error> {
    let mut process = Command::new("git")
        .current_dir(current_dir)
        .args(args)
        .spawn()?;

    let exit_code = process.wait()?;
    let mut buf = String::new();
    if let Some(mut stdout) = process.stdout {
        let _ = stdout.read_to_string(&mut buf)?;
    }
    if !exit_code.success() {
        let message = args.join(" ");
        Err(GitError::new(message, buf, exit_code).into())
    } else {
        Ok(buf)
    }
}
