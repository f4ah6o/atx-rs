#!/usr/bin/env bash
# agent-attn.sh (multi-session)
# Claude Code hooks -> change terminal background color via OSC 11/111.
# Writes to a per-session tty recorded by cclaude.
#
# Env:
#   CLAUDE_ATTN_SESSION set by cclaude
#
# This script never fails hooks (always exit 0).

set -euo pipefail

REG_DIR="${HOME}/.cache/claude_ttys"
SESSION_ID="${CLAUDE_ATTN_SESSION:-}"

# Resolve target tty from session
TTY=""
if [[ -n "$SESSION_ID" && -f "${REG_DIR}/${SESSION_ID}.tty" ]]; then
  TTY="$(cat "${REG_DIR}/${SESSION_ID}.tty" 2>/dev/null || true)"
fi

# If session is missing (e.g., you launched plain `claude`), do nothing safely.
if [[ -z "$TTY" || ! -w "$TTY" ]]; then
  exit 0
fi

# Read hook payload (may be empty depending on event)
payload="$(cat 2>/dev/null || true)"

action_json="$(
python3 - "$payload" <<'PY' 2>/dev/null || true
import json, sys

raw = sys.argv[1] if len(sys.argv) > 1 else ""
raw = (raw or "").strip()
d = {}
if raw:
    try:
        d = json.loads(raw)
    except Exception:
        d = {}

event = d.get("hook_event_name", "") or ""
nt    = d.get("notification_type", "") or ""

# ---- Customize colors here ----
WAIT_BG = "#002b36"   # "your turn" after Stop
IDLE_BG = "#3b2a00"   # idle prompt
PERM_BG = "#4a0000"   # permission prompt

action = {"mode": "noop"}

if event == "Notification" and nt == "permission_prompt":
    action = {"mode":"set", "bg": PERM_BG}
elif event == "Notification" and nt == "idle_prompt":
    action = {"mode":"set", "bg": IDLE_BG}
elif event == "Stop":
    action = {"mode":"set", "bg": WAIT_BG}
elif event == "UserPromptSubmit":
    action = {"mode":"reset"}

print(json.dumps(action))
PY
)"

[[ -n "${action_json:-}" ]] || exit 0

mode="$(
python3 -c 'import json,sys; print(json.loads(sys.argv[1]).get("mode","noop"))' "$action_json" 2>/dev/null || echo noop
)"
bg="$(
python3 -c 'import json,sys; print(json.loads(sys.argv[1]).get("bg",""))' "$action_json" 2>/dev/null || echo ""
)"

# OSC helpers (background only)
set_bg()   { printf '\033]11;%s\007' "$1" >"$TTY" 2>/dev/null || true; }
reset_bg() { printf '\033]111\007'        >"$TTY" 2>/dev/null || true; }

case "$mode" in
  set)   [[ -n "$bg" ]] && set_bg "$bg" ;;
  reset) reset_bg ;;
  *)     : ;;
esac

exit 0

