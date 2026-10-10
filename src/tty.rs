use crate::color::{Color, ColorAction};
use crate::error::Result;
use std::fs::OpenOptions;
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

/// TTY デバイスを表す
#[derive(Debug, Clone)]
pub struct Tty {
    pub path: PathBuf,
}

impl Tty {
    /// 現在のTTYを検出
    pub fn detect() -> Option<Self> {
        // 優先: ttyname(0) -> /dev/tty
        if let Ok(path) = std::fs::read_link("/proc/self/fd/0")
            && (path.starts_with("/dev/tty") || path.starts_with("/dev/pts"))
        {
            return Some(Tty { path });
        }
        // フォールバック: /dev/tty
        if Path::new("/dev/tty").exists() {
            return Some(Tty {
                path: PathBuf::from("/dev/tty"),
            });
        }
        None
    }

    /// TTYが書き込み可能かチェック
    pub fn is_writable(&self) -> bool {
        // 実際に開いてみる
        OpenOptions::new().write(true).open(&self.path).is_ok()
    }

    /// OSCシーケンスをTTYに書き込む
    pub fn write_osc(&self, data: &[u8]) -> Result<()> {
        let mut file = OpenOptions::new()
            .write(true)
            .custom_flags(libc::O_NOCTTY)
            .open(&self.path)
            .map_err(|e| {
                eprintln!("atx: TTY open failed ({}): {}", self.path.display(), e);
                e
            })?;

        file.write_all(data).map_err(|e| {
            eprintln!("atx: TTY write failed ({}): {}", self.path.display(), e);
            e
        })?;

        Ok(())
    }

    /// OSCシーケンスをTTYに書き込む（失敗してもエラーを返さない）
    pub fn write_osc_silent(&self, data: &[u8]) {
        eprintln!(
            "atx: Writing OSC sequence to {}: {:?}",
            self.path.display(),
            data
        );
        let _ = self.write_osc(data);
    }

    /// 背景色を設定
    pub fn set_background(&self, color: Color) {
        let seq = color.to_osc_set_sequence();
        if !seq.is_empty() {
            self.write_osc_silent(seq.as_bytes());
        }
    }

    /// 背景色をリセット
    pub fn reset_background(&self) {
        self.write_osc_silent(Color::to_osc_reset_sequence().as_bytes());
    }

    /// ColorActionを実行
    pub fn apply_action(&self, action: ColorAction) {
        match action {
            ColorAction::Set(color) => self.set_background(color),
            ColorAction::Reset => self.reset_background(),
            ColorAction::NoOp => {}
        }
    }

    /// 色を設定（set_backgroundのエイリアス）
    pub fn set_color(&self, color: Color) {
        self.set_background(color);
    }

    /// 色をリセット（reset_backgroundのエイリアス）
    pub fn reset_color(&self) {
        self.reset_background();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tty_detect() {
        let tty = Tty::detect();
        // CI環境などでNoneになる可能性がある
        if let Some(tty) = tty {
            assert!(tty.path.starts_with("/dev"));
        }
    }

    #[test]
    fn test_osc_sequences() {
        // OSCシーケンスのフォーマットチェック
        let set_seq = Color::PERMISSION_RED.to_osc_set_sequence();
        assert!(set_seq.starts_with("\x1b]11;"));
        assert!(set_seq.ends_with("\x07"));

        let reset_seq = Color::to_osc_reset_sequence();
        assert_eq!(reset_seq, "\x1b]111\x07");
    }
}
