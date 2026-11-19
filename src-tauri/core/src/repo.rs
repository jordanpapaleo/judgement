//! Repo identity: the canonical Git common dir, so linked worktrees of one
//! repository share one identity (§7).

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// Bounded so the CLI stays inside its 3-second budget even if Git hangs.
const GIT_TIMEOUT: Duration = Duration::from_millis(1500);

/// The canonical Git common dir containing `path`, or `None` outside Git
/// (or when Git is missing or too slow).
pub fn repo_root(path: &Path) -> Option<String> {
    let mut child = Command::new("git")
        .arg("-C")
        .arg(path)
        .args(["rev-parse", "--path-format=absolute", "--git-common-dir"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let deadline = Instant::now() + GIT_TIMEOUT;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(5)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
        }
    };
    if !status.success() {
        return None;
    }
    let mut out = String::new();
    child.stdout.take()?.read_to_string(&mut out).ok()?;
    let dir = PathBuf::from(out.trim());
    let canonical = dir.canonicalize().ok()?;
    Some(canonical.to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn git(dir: &Path, args: &[&str]) {
        let ok = Command::new("git")
            .arg("-C")
            .arg(dir)
            .args(args)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .unwrap()
            .success();
        assert!(ok, "git {args:?}");
    }

    #[test]
    fn worktrees_share_one_identity() {
        let tmp = tempfile::tempdir().unwrap();
        let main = tmp.path().join("main");
        std::fs::create_dir(&main).unwrap();
        git(&main, &["init", "-q", "-b", "main"]);
        git(
            &main,
            &[
                "-c",
                "user.name=t",
                "-c",
                "user.email=t@t",
                "commit",
                "-q",
                "--allow-empty",
                "-m",
                "init",
            ],
        );
        let wt = tmp.path().join("wt");
        git(&main, &["worktree", "add", "-q", wt.to_str().unwrap()]);
        std::fs::create_dir(main.join("sub")).unwrap();

        let root = repo_root(&main).expect("main is a repo");
        assert!(root.ends_with("/main/.git"), "{root}");
        assert_eq!(repo_root(&wt), Some(root.clone()));
        assert_eq!(repo_root(&main.join("sub")), Some(root));
    }

    #[test]
    fn outside_git_is_none() {
        let tmp = tempfile::tempdir().unwrap();
        assert_eq!(repo_root(tmp.path()), None);
    }
}
