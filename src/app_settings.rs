//! Persistent app-level settings (independent of any single project/vault).
//!
//! Stored at `dirs::config_dir()/fotobuch/settings.toml` per XDG convention.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    #[serde(default = "schema_v1")]
    pub version: u32,
    /// Last opened vault (auto-loaded on next start).
    pub last_vault: Option<PathBuf>,
    /// Recently opened vaults, newest first. Capped at 5.
    #[serde(default)]
    pub recent_vaults: Vec<PathBuf>,
}

fn schema_v1() -> u32 {
    SCHEMA_VERSION
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            version: SCHEMA_VERSION,
            last_vault: None,
            recent_vaults: vec![],
        }
    }
}

impl AppSettings {
    /// Load from the OS config directory. Returns `Default` when no file exists yet.
    pub fn load() -> Result<Self> {
        let path = settings_path()?;
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to read {}", path.display()))?;
        let settings: Self = toml::from_str(&content).context("Failed to parse settings.toml")?;
        Ok(settings)
    }

    /// Atomically persist settings to disk.
    pub fn save(&self) -> Result<()> {
        let path = settings_path()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create config dir {}", parent.display()))?;
        }
        let content = toml::to_string_pretty(self).context("Failed to serialize settings")?;
        let tmp = path.with_extension("toml.tmp");
        std::fs::write(&tmp, &content)
            .with_context(|| format!("Failed to write tmp file {}", tmp.display()))?;
        std::fs::rename(&tmp, &path)
            .with_context(|| format!("Failed to rename {} → {}", tmp.display(), path.display()))?;
        Ok(())
    }

    /// Record `vault` as the last and most-recent vault. Caps list at 5.
    pub fn add_recent_vault(&mut self, vault: &Path) {
        self.recent_vaults.retain(|v| v != vault);
        self.recent_vaults.insert(0, vault.to_path_buf());
        self.recent_vaults.truncate(5);
        self.last_vault = Some(vault.to_path_buf());
    }

    /// Remove a vault that is no longer accessible.
    pub fn purge_vault(&mut self, vault: &Path) {
        self.recent_vaults.retain(|v| v != vault);
        if self.last_vault.as_deref() == Some(vault) {
            self.last_vault = self.recent_vaults.first().cloned();
        }
    }
}

fn settings_path() -> Result<PathBuf> {
    let config_dir = dirs::config_dir()
        .ok_or_else(|| anyhow::anyhow!("Cannot determine OS config directory"))?;
    Ok(config_dir.join("fotobuch").join("settings.toml"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn with_tmp_settings<F: FnOnce(&Path) -> AppSettings>(f: F) {
        let tmp = TempDir::new().unwrap();
        let _ = f(tmp.path());
    }

    #[test]
    fn app_settings_round_trip_preserves_recent_list_order() {
        let tmp = TempDir::new().unwrap();
        let settings_file = tmp.path().join("fotobuch").join("settings.toml");

        let mut s = AppSettings::default();
        let vault_a = PathBuf::from("/vaults/alpha");
        let vault_b = PathBuf::from("/vaults/beta");
        let vault_c = PathBuf::from("/vaults/gamma");
        s.add_recent_vault(&vault_a);
        s.add_recent_vault(&vault_b);
        s.add_recent_vault(&vault_c);

        // Manually write to the tmp path
        std::fs::create_dir_all(settings_file.parent().unwrap()).unwrap();
        let content = toml::to_string_pretty(&s).unwrap();
        std::fs::write(&settings_file, &content).unwrap();

        // Parse back — must use the same path
        let loaded: AppSettings = toml::from_str(&content).unwrap();
        assert_eq!(loaded.recent_vaults[0], vault_c);
        assert_eq!(loaded.recent_vaults[1], vault_b);
        assert_eq!(loaded.recent_vaults[2], vault_a);
        assert_eq!(loaded.last_vault, Some(vault_c));
    }

    #[test]
    fn add_recent_vault_deduplicates_and_moves_to_front() {
        let mut s = AppSettings::default();
        let v1 = PathBuf::from("/a");
        let v2 = PathBuf::from("/b");
        s.add_recent_vault(&v1);
        s.add_recent_vault(&v2);
        s.add_recent_vault(&v1); // re-add v1 → should move to front
        assert_eq!(s.recent_vaults[0], v1);
        assert_eq!(s.recent_vaults[1], v2);
        assert_eq!(s.recent_vaults.len(), 2);
    }

    #[test]
    fn add_recent_vault_caps_at_five() {
        let mut s = AppSettings::default();
        for i in 0..7u8 {
            s.add_recent_vault(&PathBuf::from(format!("/vault/{i}")));
        }
        assert_eq!(s.recent_vaults.len(), 5);
    }

    #[test]
    fn purge_vault_removes_from_recent_and_updates_last() {
        let mut s = AppSettings::default();
        let v1 = PathBuf::from("/a");
        let v2 = PathBuf::from("/b");
        s.add_recent_vault(&v1);
        s.add_recent_vault(&v2);
        s.purge_vault(&v2); // v2 was last_vault
        assert!(!s.recent_vaults.contains(&v2));
        assert_eq!(s.last_vault, Some(v1));
    }
}
