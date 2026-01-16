use crate::config::{ClaudeConfig, ConfigBackup};
use crate::runtime::Runtime;
use crate::session::SessionId;
use crate::tty::Tty;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// クリーンアップハンドラー
///
/// 実行終了時に:
/// 1. 背景色をリセット
/// 2. 設定ファイルを復元
/// 3. 一時ディレクトリを削除
#[derive(Debug)]
pub struct CleanupHandler {
    runtime: Runtime,
    session_id: SessionId,
    tty: Option<Tty>,
    config_backup: Option<ConfigBackup>,
    pub cleanup_done: Arc<AtomicBool>,
}

impl CleanupHandler {
    pub fn new(
        runtime: Runtime,
        session_id: SessionId,
        tty: Option<Tty>,
        config_backup: Option<ConfigBackup>,
    ) -> Self {
        CleanupHandler {
            runtime,
            session_id,
            tty,
            config_backup,
            cleanup_done: Arc::new(AtomicBool::new(false)),
        }
    }

    /// クリーンアップを実行
    pub fn cleanup(&self) {
        if self.cleanup_done.swap(true, Ordering::SeqCst) {
            return; // 既に実行済み
        }

        // 1. blinkプロセスを停止
        let _ = crate::blink::stop_blink(&self.runtime);

        // 2. 背景色をリセット
        if let Some(ref tty) = self.tty {
            tty.reset_background();
        }

        // 3. 設定ファイルを復元
        if let Some(ref backup) = self.config_backup {
            let config = ClaudeConfig::from_path(backup.original_path.clone());
            let _ = config.restore(backup); // エラーは無視
        }

        // 4. 一時ディレクトリ削除は Drop で行われる
    }

    /// シグナルハンドラーを登録
    pub fn register_signal_handlers(&self) -> std::io::Result<()> {
        use signal_hook::consts::signal;

        let cleanup_flag = Arc::clone(&self.cleanup_done);
        let runtime = self.runtime.clone();

        // SIGINT, SIGTERM, SIGHUP を監視
        for sig in &[signal::SIGINT, signal::SIGTERM, signal::SIGHUP] {
            let cleanup_done = Arc::clone(&cleanup_flag);
            let tty = self.tty.clone();
            let runtime_clone = runtime.clone();

            // シグナルハンドラー内で実行する関数
            unsafe {
                signal_hook::low_level::register(*sig, move || {
                    if cleanup_done.swap(true, Ordering::SeqCst) {
                        return; // 既に実行済み
                    }
                    // blinkプロセスを停止
                    let _ = crate::blink::stop_blink(&runtime_clone);
                    // 背景色リセット（最小限の操作）
                    if let Some(ref tty) = tty {
                        tty.reset_background();
                    }
                    // 他のクリーンアップはプロセス終了後に行う
                })?;
            }
        }

        Ok(())
    }

    /// 緊急クリーンアップ（シグナルハンドラー内で実行）
    pub fn emergency_cleanup(&self) {
        if self.cleanup_done.swap(true, Ordering::SeqCst) {
            return;
        }

        // 1. blinkプロセスを停止
        let _ = crate::blink::stop_blink(&self.runtime);

        // 2. 背景色をリセット
        if let Some(ref tty) = self.tty {
            tty.reset_background();
        }
    }
}

impl Drop for CleanupHandler {
    fn drop(&mut self) {
        self.cleanup();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cleanup_double_call() {
        let runtime = Runtime::new(std::process::id()).unwrap();
        let session_id = SessionId::generate();
        let tty = Tty::detect();
        let handler = CleanupHandler::new(runtime, session_id, tty, None);

        // 2回呼んでもパニックしない
        handler.cleanup();
        handler.cleanup();
        assert!(handler.cleanup_done.load(Ordering::SeqCst));
    }
}
