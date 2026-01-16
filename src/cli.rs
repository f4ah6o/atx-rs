use clap::{Parser, Subcommand, ValueEnum};

/// atx - Coding Agent CLI の背景色を可視化する補助ツール
#[derive(Parser, Debug)]
#[command(name = "atx")]
#[command(author, version, about, long_about = None)]
pub struct Args {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// 既存CLIを一時的に拡張して実行
    Run(RunCommand),
    /// フックイベントを処理して背景色を変更
    Hook(HookCommand),
    /// チカチカプロセス（バックグラウンドで色を交互に切り替え）
    Blink(BlinkCommand),
    /// 設定ファイルを管理
    Config(ConfigCommand),
    /// 孤立した一時ディレクトリを削除
    Clean(CleanCommand),
}

#[derive(Parser, Debug)]
pub struct RunCommand {
    /// 実行するコマンドと引数
    #[arg(last = true, required = true)]
    pub command: Vec<String>,
}

#[derive(Parser, Debug)]
pub struct HookCommand {
    /// エンジンタイプ
    #[arg(long)]
    pub engine: Engine,

    /// JSONペイロード（テスト/デバッグ用）
    #[arg(long)]
    pub payload: Option<String>,
}

#[derive(Parser, Debug)]
pub struct CleanCommand {
    /// ドライラン - 削除対象を表示するだけで実際には削除しない
    #[arg(long)]
    pub dry_run: bool,
}

#[derive(Parser, Debug)]
pub struct BlinkCommand {
    /// TTYパス
    #[arg(long)]
    pub tty: String,

    /// チカチカ色1
    #[arg(long)]
    pub color_a: String,

    /// チカチカ色2
    #[arg(long)]
    pub color_b: String,

    /// 切り替え間隔（ミリ秒）
    #[arg(long)]
    pub interval: String,
}

/// デフォルト設定ファイルを生成
#[derive(Parser, Debug)]
pub struct ConfigInitCommand {}

/// 設定ファイルのパスを表示
#[derive(Parser, Debug)]
pub struct ConfigPathCommand {}

#[derive(Subcommand, Debug)]
pub enum ConfigAction {
    Init(ConfigInitCommand),
    Path(ConfigPathCommand),
}

#[derive(Parser, Debug)]
pub struct ConfigCommand {
    #[command(subcommand)]
    pub action: ConfigAction,
}

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum Engine {
    /// Claude Code
    Claude,
    /// OpenAI Codex (将来実装)
    Codex,
}

impl Engine {
    pub fn as_str(&self) -> &'static str {
        match self {
            Engine::Claude => "claude",
            Engine::Codex => "codex",
        }
    }
}
