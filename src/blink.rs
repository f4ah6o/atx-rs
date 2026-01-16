/// チカチカ機能（バックグラウンドプロセス）
use crate::color::Color;
use crate::error::{AtxError, Result};
use crate::runtime::Runtime;
use crate::session::SessionId;
use crate::settings::BlinkConfig;
use crate::tty::Tty;
use anyhow::anyhow;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use std::time::Duration;

/// blinkサブコマンドの引数
#[derive(Debug, Clone)]
pub struct BlinkArgs {
    pub tty_path: PathBuf,
    pub color_a: String,
    pub color_b: String,
    pub interval_ms: u64,
}

impl BlinkArgs {
    pub fn from_cli_args() -> Result<Self> {
        let args: Vec<String> = std::env::args().collect();
        let mut tty_path = None;
        let mut color_a = None;
        let mut color_b = None;
        let mut interval_ms = None;

        let mut i = 0;
        while i < args.len() {
            match args[i].as_str() {
                "--tty" => {
                    if i + 1 < args.len() {
                        tty_path = Some(PathBuf::from(&args[i + 1]));
                        i += 2;
                    } else {
                        return Err(AtxError::Other(anyhow!("--tty requires an argument")));
                    }
                }
                "--color-a" => {
                    if i + 1 < args.len() {
                        color_a = Some(args[i + 1].clone());
                        i += 2;
                    } else {
                        return Err(AtxError::Other(anyhow!("--color-a requires an argument")));
                    }
                }
                "--color-b" => {
                    if i + 1 < args.len() {
                        color_b = Some(args[i + 1].clone());
                        i += 2;
                    } else {
                        return Err(AtxError::Other(anyhow!("--color-b requires an argument")));
                    }
                }
                "--interval" => {
                    if i + 1 < args.len() {
                        interval_ms = Some(args[i + 1].parse::<u64>()
                            .map_err(|e| AtxError::Other(anyhow!("invalid interval: {}", e)))?);
                        i += 2;
                    } else {
                        return Err(AtxError::Other(anyhow!("--interval requires an argument")));
                    }
                }
                _ => {
                    i += 1;
                }
            }
        }

        Ok(BlinkArgs {
            tty_path: tty_path.ok_or_else(|| AtxError::Other(anyhow!("--tty is required")))?,
            color_a: color_a.ok_or_else(|| AtxError::Other(anyhow!("--color-a is required")))?,
            color_b: color_b.ok_or_else(|| AtxError::Other(anyhow!("--color-b is required")))?,
            interval_ms: interval_ms.ok_or_else(|| AtxError::Other(anyhow!("--interval is required")))?,
        })
    }
}

/// チカチカプロセスを実行
pub fn run_blink(args: BlinkArgs) -> Result<()> {
    let tty = Tty { path: args.tty_path };
    let color_a = Color::Hex(Box::leak(args.color_a.into_boxed_str()));
    let color_b = Color::Hex(Box::leak(args.color_b.into_boxed_str()));
    let duration = Duration::from_millis(args.interval_ms);

    loop {
        tty.set_color(color_a);
        thread::sleep(duration);
        tty.set_color(color_b);
        thread::sleep(duration);
    }
}

/// blinkプロセスを起動
pub fn start_blink(
    runtime: &Runtime,
    session_id: &SessionId,
    tty_path: &Path,
    config: &BlinkConfig,
) -> Result<()> {
    if !config.enabled {
        return Ok(());
    }

    // 既存のblinkプロセスを停止
    stop_blink(runtime)?;

    // 現在のバイナリのフルパスを取得
    let current_exe = std::env::current_exe()
        .map_err(AtxError::Io)?;

    // blinkプロセスを起動
    let child = Command::new(&current_exe)
        .arg("blink")
        .arg("--tty").arg(tty_path)
        .arg("--color-a").arg(&config.color_a)
        .arg("--color-b").arg(&config.color_b)
        .arg("--interval").arg(config.interval_ms.to_string())
        .spawn()
        .map_err(|e| AtxError::Other(anyhow!("blinkプロセスの起動に失敗: {}", e)))?;

    // PIDを保存
    let pid_file = runtime.blink_pid_file(session_id);
    if let Some(parent) = pid_file.parent() {
        fs::create_dir_all(parent)
            .map_err(AtxError::Io)?;
    }
    let mut file = File::create(&pid_file)
        .map_err(AtxError::Io)?;
    writeln!(file, "{}", child.id())
        .map_err(AtxError::Io)?;

    eprintln!("atx: started blink process (PID={})", child.id());

    Ok(())
}

/// blinkプロセスを停止
pub fn stop_blink(runtime: &Runtime) -> Result<()> {
    // すべてのセッションのblinkプロセスを停止
    let blink_dir = runtime.dir.join("blink");

    if !blink_dir.exists() {
        return Ok(());
    }

    let entries = fs::read_dir(&blink_dir)
        .map_err(AtxError::Io)?;

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("pid") {
            // PIDファイルを読み取り
            if let Ok(pid_str) = fs::read_to_string(&path) {
                if let Ok(pid) = pid_str.trim().parse::<u32>() {
                    // プロセスにSIGTERMを送信
                    eprintln!("atx: stopping blink process (PID={})", pid);
                    let _ = unsafe {
                        libc::kill(pid as i32, libc::SIGTERM);
                    };
                    // PIDファイルを削除
                    let _ = fs::remove_file(&path);
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stop_blink_no_dir() {
        let runtime = Runtime::new(std::process::id()).unwrap();

        // blinkディレクトリが存在しない場合は何もしない
        let result = stop_blink(&runtime);
        assert!(result.is_ok());
    }
}
