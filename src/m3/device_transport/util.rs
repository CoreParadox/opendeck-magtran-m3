use std::path::{Path, PathBuf};
use std::fs::read_to_string;

use anyhow::{Context, Result, bail};

pub(crate) fn hidraw_path_from_syspath(syspath: &Path) -> Result<PathBuf> {
    let uevent = read_to_string(syspath.join("uevent"))
        .with_context(|| format!("failed to read uevent for {}", syspath.display()))?;

    for line in uevent.lines() {
        if let Some(dev) = line.strip_prefix("DEVNAME=") && !dev.is_empty() {
            return Ok(PathBuf::from(format!("/dev/{dev}")));
        }
    }
    
    bail!("no DEVNAME in uevent for {}", syspath.display())
}
