use crate::blink;
use crate::cli::Engine;
use crate::color::{Color, ColorAction};
use crate::error::{AtxError, Result};
use crate::runtime::Runtime;
use crate::session::SessionId;
use crate::settings::{AtxSettings, ColorsConfig};
use crate::tty::Tty;
use serde::Deserialize;
use std::io::{self, Read};
use std::path::PathBuf;

/// Claudeのフックイベント
#[derive(Debug, Deserialize)]
#[serde(tag = "hook_event_name")]
#[serde(rename_all = "PascalCase")]
enum ClaudeEvent {
    #[serde(rename = "Notification")]
    Notification { notification_type: String },
    #[serde(rename = "Stop")]
    Stop,
    #[serde(rename = "UserPromptSubmit")]
    UserPromptSubmit,
}

/// 汎用的なフックペイロード（未知のイベント用）
#[derive(Debug, Deserialize)]
struct GenericPayload {
    #[serde(default)]
    hook_event_name: Option<String>,
}

/// フックイベントを処理
pub fn handle_hook(engine: Engine, payload: Option<String>) -> Result<ColorAction> {
    let session_id = SessionId::from_env()?;

    // セッションIDから元のatx runプロセスのPIDを抽出
    // hookはClaude Codeのプロセスから呼ばれるため、
    // セッションIDに含まれるPIDを使用してランタイムディレクトリを特定する
    let original_pid = session_id
        .extract_pid()
        .ok_or_else(|| AtxError::InvalidSessionId(session_id.as_str().to_string()))?;

    let _runtime = Runtime::open(original_pid)?;

    // TTYパスを取得
    let tty_path = _runtime
        .get_tty_for_session(engine.as_str(), &session_id)
        .ok_or_else(|| AtxError::TtyDetection("Session TTY file not found".to_string()))?;

    // 設定を読み込み
    let settings = AtxSettings::new();
    let config = settings.load()?;

    // ペイロードを読み取り
    let payload_text = match payload {
        Some(p) => p,
        None => {
            let mut buf = String::new();
            io::stdin().read_to_string(&mut buf)?;
            buf
        }
    };

    // イベントをパースして色アクションを決定
    let action = match engine {
        Engine::Claude => parse_claude_event(&payload_text, &_runtime, &session_id, &tty_path, &config)?,
        Engine::Codex => ColorAction::NoOp, // 将来実装
    };

    // 色を適用（TTYデバイスにOSCシーケンスを書き込む）
    let tty = Tty { path: PathBuf::from(&tty_path) };
    match action {
        ColorAction::Set(color) => {
            tty.set_color(color);
        }
        ColorAction::Reset => {
            tty.reset_color();
        }
        ColorAction::NoOp => {}
    }

    Ok(action)
}

/// Claudeのイベントをパース
fn parse_claude_event(
    payload: &str,
    runtime: &Runtime,
    session_id: &SessionId,
    tty_path: &str,
    config: &crate::settings::Settings,
) -> Result<ColorAction> {
    let payload = payload.trim();
    if payload.is_empty() {
        return Ok(ColorAction::NoOp);
    }

    // まず汎用パースを試す
    if let Ok(generic) = serde_json::from_str::<GenericPayload>(payload) {
        match generic.hook_event_name.as_deref() {
            Some("Notification") => {
                // 再パースして notification_type を取得
                if let Ok(event) = serde_json::from_str::<ClaudeEvent>(payload) {
                    return Ok(claude_event_to_action(&event, &config.colors));
                }
            }
            Some("Stop") => {
                // blinkを停止してリセット
                let _ = blink::stop_blink(runtime);
                return Ok(ColorAction::Reset);
            }
            Some("UserPromptSubmit") => {
                // blinkを開始
                let _ = blink::start_blink(runtime, session_id, &PathBuf::from(tty_path), &config.blink);
                return Ok(ColorAction::NoOp);
            }
            _ => {}
        }
    }

    // 直接ClaudeEventとしてパース
    match serde_json::from_str::<ClaudeEvent>(payload) {
        Ok(event) => Ok(claude_event_to_action(&event, &config.colors)),
        // JSON不正でもエラーにはしない（hookを失敗させない）
        Err(_) => Ok(ColorAction::NoOp),
    }
}

/// Claudeイベントを色アクションに変換
fn claude_event_to_action(event: &ClaudeEvent, colors: &ColorsConfig) -> ColorAction {
    match event {
        ClaudeEvent::Notification { notification_type } => match notification_type.as_str() {
            "permission_prompt" => {
                // 設定ファイルの色を使用
                ColorAction::Set(Color::Hex(Box::leak(colors.permission.clone().into_boxed_str())))
            }
            "idle_prompt" => {
                // 設定ファイルの色を使用
                ColorAction::Set(Color::Hex(Box::leak(colors.idle.clone().into_boxed_str())))
            }
            _ => ColorAction::NoOp,
        },
        ClaudeEvent::Stop => ColorAction::Reset,
        ClaudeEvent::UserPromptSubmit => ColorAction::NoOp,
    }
}

#[cfg(test)]
mod tests {
    // TODO: 新しいシグネチャに対応したテストを実装する
    // parse_claude_event は runtime, session_id, tty_path, config が必要
}
