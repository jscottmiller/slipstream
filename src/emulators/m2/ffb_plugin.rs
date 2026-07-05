//! Tunes the FFB Arcade Plugin's FFBPlugin.ini (installed alongside the
//! emulator) for the selected wheel. The plugin ships a complete ini; we only
//! patch the force-range keys rather than regenerate the whole file, so new
//! plugin versions keep their own defaults for everything else.

use crate::domain::wheel::WheelProfile;
use anyhow::{Context, Result};
use std::path::Path;

pub fn apply(install_dir: &Path, wheel: &WheelProfile) -> Result<()> {
    let path = install_dir.join("FFBPlugin.ini");
    let text = std::fs::read_to_string(&path)
        .context("FFBPlugin.ini not found — is the FFB Arcade Plugin installed?")?;

    let min = wheel.ffb.min_force.to_string();
    let max = wheel.ffb.max_force.to_string();
    let updated = set_keys(
        &text,
        &[
            ("MinForce", min.as_str()),
            ("MaxForce", max.as_str()),
            ("MinForceDaytona", min.as_str()),
            ("MaxForceDaytona", max.as_str()),
        ],
    );
    std::fs::write(&path, updated).context("writing FFBPlugin.ini")?;
    Ok(())
}

fn set_keys(ini: &str, overrides: &[(&str, &str)]) -> String {
    let mut out: String = ini
        .lines()
        .map(|line| {
            let key = line.split('=').next().unwrap_or("").trim();
            match overrides.iter().find(|(k, _)| *k == key) {
                Some((k, v)) => format!("{k}={v}"),
                None => line.to_string(),
            }
        })
        .collect::<Vec<_>>()
        .join("\r\n");
    out.push_str("\r\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_keys_patches_exact_keys_only() {
        let ini = "[Settings]\r\nGameId=25\r\nMinForce=0\r\nMaxForce=100\r\nMinForceDaytona=0\r\n";
        let out = set_keys(ini, &[("MinForce", "15"), ("MinForceDaytona", "15")]);
        assert!(out.contains("MinForce=15\r\n"));
        assert!(out.contains("MinForceDaytona=15\r\n"));
        // Untouched keys survive; "MinForce" must not clobber "MinForceDaytona".
        assert!(out.contains("GameId=25\r\n"));
        assert!(out.contains("MaxForce=100\r\n"));
    }
}
