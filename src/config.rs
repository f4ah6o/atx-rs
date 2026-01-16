use crate::error::{AtxError, Result};
use crate::runtime::Runtime;
use dirs::home_dir;
use serde_json::json;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

/// Claudeの設定ファイルマネージャー
#[derive(Debug)]
pub struct ClaudeConfig {
    pub path: PathBuf,
}

impl ClaudeConfig {
    /// デフォルトの設定ファイルパスを取得
    pub fn default_path() -> PathBuf {
        home_dir()
            .unwrap_or_else(|| PathBuf::from("~"))
            .join(".claude")
            .join("settings.local.json")
    }

    pub fn new() -> Self {
        ClaudeConfig {
            path: Self::default_path(),
        }
    }

    pub fn from_path(path: PathBuf) -> Self {
        ClaudeConfig { path }
    }

    /// 設定ファイルが存在するか
    pub fn exists(&self) -> bool {
        self.path.exists()
    }

    /// 設定をバックアップ
    pub fn backup(&self, runtime: &Runtime) -> Result<ConfigBackup> {
        let backup_path = runtime.dir.join("backup").join("claude-settings.json.orig");

        if self.exists() {
            fs::copy(&self.path, &backup_path)
                .map_err(|e| AtxError::ConfigBackup(self.path.clone(), e))?;
        } else {
            // 空のバックアップを作成
            fs::write(&backup_path, b"{}")
                .map_err(|e| AtxError::ConfigBackup(backup_path.clone(), e))?;
        }

        Ok(ConfigBackup {
            original_path: self.path.clone(),
            backup_path,
        })
    }

    /// hookを注入
    ///
    /// Claudeのsettings.local.jsonのhooks構造:
    /// ```json
    /// {
    ///   "hooks": {
    ///     "Notification": [...],
    ///     "Stop": [...],
    ///     "UserPromptSubmit": [...]
    ///   }
    /// }
    /// ```
    ///
    /// 各イベントの配列の末尾にatx hookを追加します。
    pub fn inject_hook(&self, _backup: &ConfigBackup) -> Result<()> {
        // 現在のバイナリのフルパスを取得
        let current_exe = std::env::current_exe()
            .map_err(AtxError::Io)?;
        let hook_command = format!("{} hook --engine claude", current_exe.display());

        eprintln!("atx: injecting hook command: {}", hook_command); // デバッグ出力

        let hook_entry = json!({
            "type": "command",
            "command": hook_command
        });

        // 現在の設定を読み込み
        let mut settings: serde_json::Value = if self.exists() {
            let content = fs::read_to_string(&self.path)
                .map_err(AtxError::Io)?;
            serde_json::from_str(&content)
                .map_err(|e| AtxError::ConfigParse(self.path.clone(), e))?
        } else {
            json!({})
        };

        // hooksオブジェクトを取得または作成
        let settings_obj = settings.as_object_mut()
            .ok_or_else(|| AtxError::Other(anyhow::anyhow!("settings is not an object")))?;

        if !settings_obj.contains_key("hooks") {
            settings_obj.insert("hooks".to_string(), json!({}));
        }

        let hooks = settings_obj["hooks"].as_object_mut()
            .ok_or_else(|| AtxError::Other(anyhow::anyhow!("hooks is not an object")))?;

        // 各イベントにhookを追加
        // Claude Codeのフック構造: { hooks: [{ hooks: [{ type: "command", command: "..." }] }] }
        for event in ["Notification", "Stop", "UserPromptSubmit"] {
            let event_array = hooks
                .entry(event)
                .or_insert_with(|| json!([]))
                .as_array_mut()
                .ok_or_else(|| AtxError::Other(anyhow::anyhow!("hooks[{}] is not an array", event)))?;

            // 重複チェック（正しい構造で）
            let already_exists = event_array.iter().any(|entry: &serde_json::Value| {
                entry.get("hooks")
                    .and_then(|h: &serde_json::Value| h.as_array())
                    .map(|arr: &Vec<serde_json::Value>| {
                        arr.iter().any(|hook: &serde_json::Value| {
                            hook.get("command")
                                .and_then(|c: &serde_json::Value| c.as_str())
                                .map(|c: &str| c.contains("atx hook --engine claude"))
                                .unwrap_or(false)
                        })
                    })
                    .unwrap_or(false)
            });

            if !already_exists {
                // 正しい構造でフックを追加
                let hook_wrapper = json!({
                    "hooks": [hook_entry.clone()]
                });
                event_array.push(hook_wrapper);
            }
        }

        // 曻き込み
        let content = serde_json::to_string_pretty(&settings)
            .map_err(|e| AtxError::Other(e.into()))?;

        // 親ディレクトリを作成
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)
                .map_err(AtxError::Io)?;
        }

        fs::write(&self.path, content)
            .map_err(|e| AtxError::ConfigWrite(self.path.clone(), e))?;

        Ok(())
    }

    /// `claude doctor` を実行して設定を検証
    ///
    /// 戻り値:
    /// - Ok(true): 検証成功（doctorが正常終了）
    /// - Ok(false): 検証失敗（doctorがエラーを報告）
    /// - Err: doctorの実行自体に失敗
    pub fn validate_with_doctor(&self) -> Result<bool> {
        let output = Command::new("claude")
            .arg("doctor")
            .stdin(std::process::Stdio::null())
            .output()
            .map_err(|e| AtxError::CommandExec("claude".to_string(), e))?;

        // doctorの出力をチェック
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        // "invalid" が含まれている場合は検証失敗
        let has_invalid = stdout.to_lowercase().contains("invalid")
            || stderr.to_lowercase().contains("invalid");

        if has_invalid {
            eprintln!("claude doctor detected invalid configuration:");
            if !stdout.is_empty() {
                eprintln!("{}", stdout);
            }
            if !stderr.is_empty() {
                eprintln!("{}", stderr);
            }
            return Ok(false);
        }

        // 終了コードもチェック
        Ok(output.status.success())
    }

    /// 設定を復元
    pub fn restore(&self, backup: &ConfigBackup) -> Result<()> {
        if !backup.backup_path.exists() {
            return Ok(()); // バックアップがない = 元々設定がなかった
        }

        // バックアップの内容を確認
        let content = fs::read_to_string(&backup.backup_path)?;
        if content.trim() == "{}" || content.trim().is_empty() {
            // 元々空の設定だったのでファイルを削除
            if self.exists() {
                fs::remove_file(&self.path)?;
            }
        } else {
            // バックアップを復元
            fs::copy(&backup.backup_path, &self.path)
                .map_err(|e| AtxError::ConfigRestore(self.path.clone(), e))?;
        }

        Ok(())
    }
}

#[derive(Debug)]
pub struct ConfigBackup {
    pub original_path: PathBuf,
    pub backup_path: PathBuf,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_claude_config_inject() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("settings.json");

        // 初期設定を作成
        let initial_settings = r#"{"model": "opus"}"#;
        fs::write(&config_path, initial_settings).unwrap();

        let config = ClaudeConfig::from_path(config_path.clone());

        // モックRuntimeを作成
        let runtime_dir = temp_dir.path().join("runtime");
        fs::create_dir_all(runtime_dir.join("backup")).unwrap();

        // 手動でバックアップファイルを作成
        let backup_path = runtime_dir.join("backup").join("claude-settings.json.orig");
        fs::write(&backup_path, initial_settings).unwrap();

        let backup = ConfigBackup {
            original_path: config_path.clone(),
            backup_path: backup_path.clone(),
        };

        // hookを注入
        config.inject_hook(&backup).unwrap();

        // 確認
        let content = fs::read_to_string(&config_path).unwrap();
        let settings: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert!(settings["hooks"].is_object());
        assert!(settings["hooks"]["Notification"].is_array());
        assert_eq!(settings["model"], "opus");
    }

    #[test]
    fn test_claude_config_restore_empty() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("settings.json");

        let config = ClaudeConfig::from_path(config_path.clone());

        let backup = ConfigBackup {
            original_path: config_path.clone(),
            backup_path: temp_dir.path().join("empty.json"),
        };

        // 空のバックアップを作成
        fs::write(&backup.backup_path, "{}").unwrap();

        // 復元
        config.restore(&backup).unwrap();

        // 元々空だったのでファイルは存在しない（削除された）
        assert!(!config_path.exists());
    }
}
