/// atx自体の設定管理
///
/// 設定ファイル: `$HOME/.config/atx/config.toml`
use crate::error::{AtxError, Result};
use dirs::{config_dir, home_dir};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Deserialize, Serialize)]
pub struct Settings {
    #[serde(default)]
    pub colors: ColorsConfig,
    #[serde(default)]
    pub blink: BlinkConfig,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            colors: ColorsConfig::default(),
            blink: BlinkConfig::default(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ColorsConfig {
    #[serde(default = "default_permission_color")]
    pub permission: String,
    #[serde(default = "default_idle_color")]
    pub idle: String,
}

impl Default for ColorsConfig {
    fn default() -> Self {
        ColorsConfig {
            permission: default_permission_color(),
            idle: default_idle_color(),
        }
    }
}

fn default_permission_color() -> String {
    "#4a0000".to_string()
}

fn default_idle_color() -> String {
    "#3b2a00".to_string()
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct BlinkConfig {
    #[serde(default = "default_blink_enabled")]
    pub enabled: bool,
    #[serde(default = "default_blink_interval")]
    pub interval_ms: u64,
    #[serde(default = "default_blink_color_a")]
    pub color_a: String,
    #[serde(default = "default_blink_color_b")]
    pub color_b: String,
}

impl Default for BlinkConfig {
    fn default() -> Self {
        BlinkConfig {
            enabled: default_blink_enabled(),
            interval_ms: default_blink_interval(),
            color_a: default_blink_color_a(),
            color_b: default_blink_color_b(),
        }
    }
}

fn default_blink_enabled() -> bool {
    true
}

fn default_blink_interval() -> u64 {
    500
}

fn default_blink_color_a() -> String {
    "#002b36".to_string()
}

fn default_blink_color_b() -> String {
    "#1a3a00".to_string()
}

/// 設定ファイルのマネージャー
#[derive(Debug)]
pub struct AtxSettings {
    pub path: PathBuf,
}

impl AtxSettings {
    /// デフォルトの設定ファイルパスを取得
    pub fn default_path() -> PathBuf {
        config_dir()
            .unwrap_or_else(|| home_dir().unwrap().join(".config"))
            .join("atx")
            .join("config.toml")
    }

    pub fn new() -> Self {
        AtxSettings {
            path: Self::default_path(),
        }
    }

    pub fn from_path(path: PathBuf) -> Self {
        AtxSettings { path }
    }

    /// 設定ファイルが存在するか
    pub fn exists(&self) -> bool {
        self.path.exists()
    }

    /// 設定を読み込み
    pub fn load(&self) -> Result<Settings> {
        if !self.exists() {
            // ファイルが存在しない場合はデフォルト値を返す
            return Ok(Settings::default());
        }

        let content = fs::read_to_string(&self.path)
            .map_err(AtxError::Io)?;

        toml::from_str(&content)
            .map_err(|e| AtxError::Other(anyhow::anyhow!("設定ファイルのパースに失敗: {}", e)))
    }

    /// デフォルト設定ファイルを生成
    pub fn generate_default(&self) -> Result<()> {
        let settings = Settings::default();
        let content = toml::to_string_pretty(&settings)
            .map_err(|e| AtxError::Other(e.into()))?;

        // 親ディレクトリを作成
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::write(&self.path, content)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_default_settings() {
        let settings = Settings::default();
        assert_eq!(settings.colors.permission, "#4a0000");
        assert_eq!(settings.colors.idle, "#3b2a00");
        assert!(settings.blink.enabled);
        assert_eq!(settings.blink.interval_ms, 500);
        assert_eq!(settings.blink.color_a, "#002b36");
        assert_eq!(settings.blink.color_b, "#1a3a00");
    }

    #[test]
    fn test_load_nonexistent() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("nonexistent.toml");
        let settings = AtxSettings::from_path(path);
        // ファイルが存在しない場合はデフォルト値を返す
        let loaded = settings.load().unwrap();
        assert_eq!(loaded.colors.permission, "#4a0000");
    }

    #[test]
    fn test_generate_and_load() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("config.toml");
        let settings = AtxSettings::from_path(path.clone());

        // デフォルト設定を生成
        settings.generate_default().unwrap();
        assert!(path.exists());

        // 読み込み
        let loaded = settings.load().unwrap();
        assert_eq!(loaded.colors.permission, "#4a0000");
        assert_eq!(loaded.colors.idle, "#3b2a00");
    }

    #[test]
    fn test_parse_custom_settings() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("custom.toml");

        let custom_content = r##"
            [colors]
            permission = "#ff0000"
            idle = "#ffff00"

            [blink]
            enabled = false
            interval_ms = 1000
            color_a = "#0000ff"
            color_b = "#00ff00"
        "##;

        fs::write(&path, custom_content).unwrap();

        let settings = AtxSettings::from_path(path);
        let loaded = settings.load().unwrap();

        assert_eq!(loaded.colors.permission, "#ff0000");
        assert_eq!(loaded.colors.idle, "#ffff00");
        assert!(!loaded.blink.enabled);
        assert_eq!(loaded.blink.interval_ms, 1000);
        assert_eq!(loaded.blink.color_a, "#0000ff");
        assert_eq!(loaded.blink.color_b, "#00ff00");
    }
}
