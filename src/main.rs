mod blink;
mod clean;
mod cleanup;
mod cli;
mod color;
mod config;
mod error;
mod hook;
mod runtime;
mod session;
mod settings;
mod tty;

use crate::clean::clean;
use crate::cli::Args;
use crate::config::ClaudeConfig;
use crate::error::{AtxError, Result};
use crate::hook::handle_hook;
use crate::runtime::Runtime;
use crate::session::Session;
use crate::settings::AtxSettings;
use crate::tty::Tty;
use clap::Parser;
use std::env;
use std::process::{Command as StdCommand, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

fn main() -> Result<()> {
    let args = Args::parse();

    match args.command {
        cli::Command::Run(run_cmd) => execute_run(run_cmd),
        cli::Command::Hook(hook_cmd) => execute_hook(hook_cmd),
        cli::Command::Blink(blink_cmd) => execute_blink(blink_cmd),
        cli::Command::Config(config_cmd) => execute_config(config_cmd),
        cli::Command::Clean(clean_cmd) => {
            clean(clean_cmd.dry_run)?;
            Ok(())
        }
    }
}

/// `atx run -- <command>` を実行
fn execute_run(cmd: cli::RunCommand) -> Result<()> {
    if cmd.command.is_empty() {
        return Err(AtxError::Other(anyhow::anyhow!(
            "コマンドが指定されていません"
        )));
    }

    let binary = &cmd.command[0];
    let args = &cmd.command[1..];

    eprintln!("atx: execute_run: binary={:?}", binary); // デバッグ出力

    // セッションを作成
    let session = Session::new();
    eprintln!(
        "atx: session.id={}, session.pid={}",
        session.id, session.pid
    ); // デバッグ出力

    let runtime = Runtime::new(session.pid)?;
    eprintln!("atx: runtime.dir={:?}", runtime.dir); // デバッグ出力

    // TTYを検出・登録
    let tty = Tty::detect();
    eprintln!("atx: detected TTY: {:?}", tty.as_ref().map(|t| &t.path)); // デバッグ出力
    if let Some(ref tty) = tty {
        runtime.register_tty("claude", &session.id, &tty.path)?;
        eprintln!("atx: registered TTY: {}", tty.path.display()); // デバッグ出力
    }

    // 設定のバックアップとhook注入
    let config = ClaudeConfig::new();
    let config_backup = config.backup(&runtime)?;
    config.inject_hook(&config_backup)?;
    let config_backup = Some(config_backup);

    // 環境変数を設定
    unsafe { env::set_var("ATX_SESSION", session.id.as_str()) };

    // クリーンアップハンドラーを設定
    let cleanup_handler =
        cleanup::CleanupHandler::new(runtime.clone(), session.id, tty.clone(), config_backup);

    // Vec<String> を Vec<&str> に変換
    let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();

    // 子プロセスをspawn（stdioを継承）
    let mut child = StdCommand::new(binary)
        .args(&args_refs)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|e| AtxError::CommandExec(binary.clone(), e))?;

    let child_pid = child.id();
    let cleanup_flag = Arc::clone(&cleanup_handler.cleanup_done);

    // シグナルフォワーディングを設定
    setup_signal_forwarding(child_pid, cleanup_flag, runtime, tty)?;

    // 子プロセスの終了を待機
    let status = child
        .wait()
        .map_err(|e| AtxError::Other(anyhow::anyhow!("Failed to wait for child: {}", e)))?;

    // CleanupHandler Dropが自動的に実行される
    if status.success() {
        Ok(())
    } else {
        Err(AtxError::Other(anyhow::anyhow!("Child exited: {}", status)))
    }
}

/// シグナルフォワーディングを設定
fn setup_signal_forwarding(
    _child_pid: u32,
    cleanup_flag: Arc<AtomicBool>,
    runtime: Runtime,
    tty: Option<Tty>,
) -> Result<()> {
    use signal_hook::consts::signal;
    use signal_hook::low_level;

    for sig in &[signal::SIGINT, signal::SIGTERM, signal::SIGHUP] {
        let cleanup_done = Arc::clone(&cleanup_flag);
        let tty_clone = tty.clone();
        let runtime_clone = runtime.clone();

        unsafe {
            low_level::register(*sig, move || {
                if cleanup_done.swap(true, Ordering::SeqCst) {
                    return;
                }

                // 子プロセスにシグナルを転送
                let _ = low_level::raise(*sig);

                // 即時クリーンアップ
                if let Some(ref t) = tty_clone {
                    t.reset_background();
                }
                let _ = crate::blink::stop_blink(&runtime_clone);
            })?;
        }
    }

    Ok(())
}

/// `atx hook --engine <name>` を実行
fn execute_hook(cmd: cli::HookCommand) -> Result<()> {
    eprintln!("atx: execute_hook called, engine={:?}", cmd.engine); // デバッグ出力
    let result = handle_hook(cmd.engine, cmd.payload);

    // hookは失敗してもexit 0
    match result {
        Ok(_) => Ok(()),
        Err(e) => {
            eprintln!("atx: hook error: {:?}", e); // デバッグ出力
            Ok(())
        }
    }
}

/// `atx blink --tty <path> --color-a <color> --color-b <color> --interval <ms>` を実行
fn execute_blink(cmd: cli::BlinkCommand) -> Result<()> {
    let blink_args = blink::BlinkArgs {
        tty_path: std::path::PathBuf::from(cmd.tty),
        color_a: cmd.color_a,
        color_b: cmd.color_b,
        interval_ms: cmd
            .interval
            .parse()
            .map_err(|e| AtxError::Other(anyhow::anyhow!("invalid interval: {}", e)))?,
    };
    blink::run_blink(blink_args)
}

/// `atx config` を実行
fn execute_config(cmd: cli::ConfigCommand) -> Result<()> {
    match cmd.action {
        cli::ConfigAction::Init(_) => {
            let settings = AtxSettings::new();
            settings.generate_default()?;
            println!("設定ファイルを生成しました: {}", settings.path.display());
            Ok(())
        }
        cli::ConfigAction::Path(_) => {
            let settings = AtxSettings::new();
            println!("{}", settings.path.display());
            Ok(())
        }
    }
}
