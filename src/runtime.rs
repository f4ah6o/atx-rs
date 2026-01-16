use crate::error::{AtxError, Result};
use crate::session::SessionId;
use dirs::runtime_dir;
use std::fs;
use std::path::{Path, PathBuf};

/// ランタイムディレクトリマネージャー
#[derive(Debug, Clone)]
pub struct Runtime {
    pub pid: u32,
    pub dir: PathBuf,
    /// Dropでクリーンアップするかどうか
    /// openで開いた場合はfalse（クリーンアップしない）
    owns_cleanup: bool,
}

impl Runtime {
    /// 新しいランタイムディレクトリを作成
    pub fn new(pid: u32) -> Result<Self> {
        let base = runtime_dir().unwrap_or_else(|| PathBuf::from("/tmp"));
        let dir = base.join(format!("atx-{}", pid));

        // ディレクトリ作成
        fs::create_dir_all(&dir)
            .map_err(|e| AtxError::RuntimeDir(dir.clone(), e))?;

        // サブディレクトリ作成
        fs::create_dir_all(dir.join("ttys/claude"))
            .map_err(|e| AtxError::RuntimeDir(dir.clone(), e))?;
        fs::create_dir_all(dir.join("ttys/codex"))
            .map_err(|e| AtxError::RuntimeDir(dir.clone(), e))?;
        fs::create_dir_all(dir.join("backup"))
            .map_err(|e| AtxError::RuntimeDir(dir.clone(), e))?;
        fs::create_dir_all(dir.join("blink"))
            .map_err(|e| AtxError::RuntimeDir(dir.clone(), e))?;

        Ok(Runtime {
            pid,
            dir,
            owns_cleanup: true,
        })
    }

    /// 既存のランタイムディレクトリを開く（作成しない）
    /// hookから呼ばれる時は、元のatx runプロセスのPIDでディレクトリを探す
    pub fn open(pid: u32) -> Result<Self> {
        let base = runtime_dir().unwrap_or_else(|| PathBuf::from("/tmp"));
        let dir = base.join(format!("atx-{}", pid));

        if !dir.exists() {
            return Err(AtxError::RuntimeDir(
                dir.clone(),
                std::io::Error::new(std::io::ErrorKind::NotFound, "Runtime directory not found"),
            ));
        }

        Ok(Runtime {
            pid,
            dir,
            owns_cleanup: false, // 読み取り専用、クリーンアップしない
        })
    }

    /// TTY登録ファイルのパスを取得
    pub fn tty_file(&self, engine: &str, session_id: &SessionId) -> PathBuf {
        self.dir
            .join("ttys")
            .join(engine)
            .join(format!("{}.tty", session_id.as_str()))
    }

    /// TTYを登録
    pub fn register_tty(&self, engine: &str, session_id: &SessionId, tty_path: &Path) -> Result<()> {
        let tty_file = self.tty_file(engine, session_id);
        fs::write(&tty_file, tty_path.to_string_lossy().as_bytes())?;
        Ok(())
    }

    /// TTYパスをセッションから読み取り
    pub fn get_tty_for_session(&self, engine: &str, session_id: &SessionId) -> Option<String> {
        let tty_file = self.tty_file(engine, session_id);
        fs::read_to_string(&tty_file).ok()
    }

    /// blink PIDファイルのパスを取得
    pub fn blink_pid_file(&self, session_id: &SessionId) -> PathBuf {
        self.dir
            .join("blink")
            .join(format!("{}.pid", session_id.as_str()))
    }

    /// ランタイムディレクトリを削除
    pub fn cleanup(&self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

impl Drop for Runtime {
    fn drop(&mut self) {
        if self.owns_cleanup {
            self.cleanup();
        }
    }
}

/// 孤立したランタイムディレクトリを検索
pub fn find_orphaned_runtimes() -> Vec<PathBuf> {
    let mut orphans = Vec::new();

    let bases = vec![
        runtime_dir().unwrap_or_else(|| PathBuf::from("/tmp")),
        PathBuf::from("/tmp"),
    ];

    for base in bases {
        if let Ok(entries) = fs::read_dir(base) {
            for entry in entries.flatten() {
                let name = entry.file_name();
                let name_str = name.to_string_lossy();
                if name_str.starts_with("atx-") {
                    let path = entry.path();
                    // PID部分を抽出してチェック
                    if let Some(pid_str) = name_str.strip_prefix("atx-") {
                        if let Ok(pid) = pid_str.parse::<u32>() {
                            // プロセスが存在するかチェック
                            if !process_exists(pid) {
                                orphans.push(path);
                            }
                        }
                    }
                }
            }
        }
    }

    orphans
}

/// プロセスが存在するかチェック（/procベース）
fn process_exists(pid: u32) -> bool {
    Path::new(&format!("/proc/{}", pid)).exists()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runtime_creation() {
        let rt = Runtime::new(std::process::id()).unwrap();
        assert!(rt.dir.exists());
        assert!(rt.dir.join("ttys/claude").exists());
        assert!(rt.dir.join("ttys/codex").exists());
        assert!(rt.dir.join("backup").exists());
    }

    #[test]
    fn test_tty_registration() {
        let rt = Runtime::new(std::process::id()).unwrap();
        let session_id = crate::session::SessionId::generate();
        let tty_path = std::path::PathBuf::from("/dev/pts/0");

        rt.register_tty("claude", &session_id, &tty_path).unwrap();

        let retrieved = rt.get_tty_for_session("claude", &session_id);
        assert_eq!(retrieved, Some("/dev/pts/0".to_string()));
    }
}
