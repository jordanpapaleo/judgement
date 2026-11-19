//! The user-approved `~/.local/bin/judgement` symlink into the installed
//! bundle. Never edits shell profiles and never replaces an existing file.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Name of the CLI inside `Judgement.app/Contents/MacOS/` (externalBin).
const BUNDLED_CLI: &str = "judgement-cli";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct CliLink {
    pub link_path: String,
    pub target: Option<String>,
    pub linked: bool,
    /// Something else already exists at `link_path`; it is never overwritten.
    pub conflict: Option<String>,
    /// Stable support is only for installs under /Applications.
    pub in_applications: bool,
}

fn bundled_cli() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let cli = exe.parent()?.join(BUNDLED_CLI);
    cli.exists().then_some(cli)
}

fn link_path() -> Option<PathBuf> {
    std::env::var_os("HOME").map(|home| Path::new(&home).join(".local/bin/judgement"))
}

pub fn status() -> CliLink {
    let target = bundled_cli();
    let link = link_path();
    let current = link.as_ref().and_then(|l| std::fs::read_link(l).ok());
    let exists = link.as_ref().is_some_and(|l| l.symlink_metadata().is_ok());
    let linked = matches!((&current, &target), (Some(c), Some(t)) if c == t);
    let conflict = (exists && !linked).then(|| match &current {
        Some(other) => format!("already links to {}", other.display()),
        None => "a file that isn't a symlink is already there".to_string(),
    });
    CliLink {
        link_path: link.map(|l| l.display().to_string()).unwrap_or_default(),
        target: target.as_ref().map(|t| t.display().to_string()),
        linked,
        conflict,
        in_applications: target
            .as_ref()
            .is_some_and(|t| t.starts_with("/Applications/")),
    }
}

pub fn install() -> Result<CliLink, String> {
    let before = status();
    if before.linked {
        return Ok(before);
    }
    if let Some(conflict) = before.conflict {
        return Err(format!("{}: {conflict}; remove it first", before.link_path));
    }
    let target = bundled_cli().ok_or("the bundled CLI wasn't found next to the app")?;
    let link = link_path().ok_or("HOME is not set")?;
    if let Some(dir) = link.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    std::os::unix::fs::symlink(&target, &link).map_err(|e| e.to_string())?;
    Ok(status())
}
