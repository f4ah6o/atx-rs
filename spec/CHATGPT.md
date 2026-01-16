---
description: `atx` is a helper CLI designed solely for changing the background color of coding agent CLIs like Claude Code / Codex.
---

# agent-attn (atx) CLI 仕様書 v0.2

## 概要

`atx` は、Claude Code / Codex などの coding agent CLI を  
**TTY単位で可視化（背景色変更）**するための、完全ポータブルな補助CLIである。

最大の特徴は **「実行時に設定し、終了時に完全に片付ける」** ことであり、  
ユーザー環境に **恒久的な変更を一切残さない**。

- `cargo install atx` により単一バイナリとして配布される
- `atx run -- claude` のように **既存CLIを一時的にラップ**して実行する
- 実行中のみ hooks / notify を有効化し、終了時に必ず元に戻す
- `atx` が導入した変更は **プロセス終了時に完全除去**される

---

## 設計思想（更新）

### 壊さず・残さず・持ち運べる（v2 定義）

- **壊さず**
  - 既存の `claude` / `codex` 設定を上書きしない
  - hooks / notify は *差分として一時適用* される
- **残さず**
  - 実行終了時（正常終了 / Ctrl+C / kill）に
    - hooks / notify 設定を元に戻す
    - 一時ファイル・TTY登録を全削除
- **持ち運べる**
  - 単一バイナリ
  - `$HOME` 以外に依存しない
  - CI / SSH / 一時環境でも動作可能

---

## インストール

```bash
cargo install atx
```

- 単一バイナリ `atx`
- 依存最小（libc / serde_json 程度）

---

## 一時管理ディレクトリ（実行中のみ存在）

```
$XDG_RUNTIME_DIR/atx-<pid>/   # 推奨
or
/tmp/atx-<pid>/

├─ ttys/
│   ├─ claude/
│   │   └─ <session_id>.tty
│   └─ codex/
│       └─ <session_id>.tty
├─ backup/
│   ├─ claude-settings.json.orig
│   └─ codex-config.toml.orig
└─ meta.json
```

- **プロセス終了時にディレクトリごと削除**
- 予期せぬ終了時も OS の tmp cleanup 対象

---

## セッションモデル

- `atx run` 実行ごとに **session_id** を生成  
  - 形式: `<unix_ns>-<pid>`
- session_id は以下に保存される  
  - 環境変数: `ATX_SESSION`
  - tty registry: `ttys/<engine>/<session_id>.tty`
- **永続保存はしない**

---

## コマンド仕様

### 1. atx run

#### 概要
既存 CLI を **一時的に拡張**して起動する。

- 起動時:
  - hooks / notify を *一時適用*
  - TTY を登録
- 実行中:
  - hook から `atx hook` が呼ばれる
- 終了時:
  - hooks / notify を **完全に元へ戻す**
  - 背景色をリセット
  - 一時ディレクトリ削除

#### 形式

```bash
atx run -- <command> [args...]
```

#### 例

```bash
atx run -- claude
atx run -- codex
atx run -- claude -c "explain this code"
```

#### 実装要点

- `ttyname(0)` を取得し登録
- 元設定を一時ディレクトリに **バックアップ**
- hooks / notify を **プロセス存続中のみ有効化**
- `EXIT / INT / TERM / HUP` で必ずロールバック

---

### 2. atx hook

#### 概要
一時的に有効化された hook / notify から呼ばれ、  
イベントに応じて背景色を変更する。

#### 形式

```bash
atx hook --engine claude
atx hook --engine codex --payload '<json>'
```

#### 特徴

- stdin が空でも壊れない
- JSON 不正でも exit 0
- **一時ディレクトリ外には一切書き込まない**

---

## エンジン別イベント対応

### Claude

| hook_event_name | notification_type | 動作 |
|-----------------|-------------------|------|
| Notification    | permission_prompt | 背景: 赤 |
| Notification    | idle_prompt       | 背景: 黄 |
| Stop            | -                 | 背景: 青 |
| UserPromptSubmit| -                 | 背景リセット |

### Codex（予定）

| type                | 動作 |
|---------------------|------|
| agent-turn-complete | 背景: 青 |

---

## hooks / notify の一時適用戦略（重要）

### Claude

- 対象ファイル: `~/.claude/settings.local.json`（local スコープの .claude/*.local.*）
- 手順:
  1. 実行開始時に全文を backup/
  2. hooks に **atx hook** を追加
  3. 終了時に backup を完全復元

※ `_managed_by` マーカーは不要  
※ 差分管理ではなく **スナップショット方式**

### Codex

- 対象ファイル: `~/.codex/config.toml`
- notify を一時追記 → 終了時に原状復帰

---

## 非機能要件

- **どの経路でも終了時に必ず原状復帰**
- hook 側の失敗が本体に影響しない
- tmux / 非 tmux 両対応
- OSC 非対応端末でも黙って何もしない
- root 権限不要

---

## clean コマンド（仕様変更）

### 方針

`atx` は原則 **clean を必要としない**。

### 役割

```bash
atx clean
```

- 想定用途:
  - クラッシュ等で一時ディレクトリが残った場合の保険
- 動作:
  - `/tmp/atx-*`
  - `$XDG_RUNTIME_DIR/atx-*`
  を削除するのみ
- **ユーザー設定ファイルは一切触らない**

---

## 将来拡張

- `atx config`（色ルール）
- `atx doctor`（設定整合性チェック）
- Codex permission / approval 対応
- Windows (ConPTY) 対応

---

## まとめ

`atx` は、

- 実行時にだけ環境を拡張し
- 終了時にすべてを消し
- 何も残さず次の環境へ移れる

**「可視化だけを持ち運ぶ」**ための、  
最小・安全・ガチポータブルなエージェント補助CLIである。
