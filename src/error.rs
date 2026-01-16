use std::io;
use std::path::PathBuf;

/// atx 固有のエラー型
#[derive(Debug, thiserror::Error)]
pub enum AtxError {
    #[error("TTYの検出に失敗しました: {0}")]
    TtyDetection(String),

    #[error("環境変数 ATX_SESSION が設定されていません")]
    SessionNotSet,

    #[error("無効なセッションID形式: {0}")]
    InvalidSessionId(String),

    #[error("ランタイムディレクトリの作成に失敗しました: {0:?}: {1}")]
    RuntimeDir(PathBuf, #[source] io::Error),

    #[error("設定ファイルのバックアップに失敗しました: {0:?}: {1}")]
    ConfigBackup(PathBuf, #[source] io::Error),

    #[error("設定ファイルの復元に失敗しました: {0:?}: {1}")]
    ConfigRestore(PathBuf, #[source] io::Error),

    #[error("フックペイロードのパースに失敗しました: {0}")]
    InvalidPayload(#[source] serde_json::Error),

    #[error("コマンドの実行に失敗しました: {0}: {1}")]
    CommandExec(String, #[source] io::Error),

    #[error("設定ファイルのパースに失敗しました: {0:?}: {1}")]
    ConfigParse(PathBuf, #[source] serde_json::Error),

    #[error("設定ファイルの書き込みに失敗しました: {0:?}: {1}")]
    ConfigWrite(PathBuf, #[source] io::Error),

    #[error("IOエラー: {0}")]
    Io(#[from] io::Error),

    #[error("その他のエラー: {0}")]
    Other(#[from] anyhow::Error),
}

pub type Result<T> = std::result::Result<T, AtxError>;
