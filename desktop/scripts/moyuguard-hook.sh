#!/bin/bash
# MoyuGuard Hook Script v2
# Reads CLI hook stdin JSON, sends to MoyuGuard desktop via Unix Socket.
# Blocks on PermissionRequest (Claude Code re-reads settings.json for this
# event live, so it works in already-running sessions) and forwards the
# desktop's response to the CLI tool. All other events are fire-and-forget.

SOCKET_PATH="/tmp/moyuguard-$(id -u).sock"

INPUT=$(cat)

if [ -z "$INPUT" ]; then
  echo '{}'
  exit 0
fi

if [ ! -S "$SOCKET_PATH" ]; then
  echo '{}'
  exit 0
fi

EVENT_NAME=$(echo "$INPUT" | python3 -c "
import sys, json
try:
    d = json.load(sys.stdin)
    print(d.get('event_name') or d.get('hook_event_name') or '')
except Exception:
    print('')
" 2>/dev/null)

case "$EVENT_NAME" in
  PermissionRequest)
    # Wait up to 24h for user decision. The desktop responds with the
    # Claude Code-specific permission shape: hookSpecificOutput.decision.behavior
    RESPONSE=$(echo "$INPUT" | nc -U -w 86400 "$SOCKET_PATH" 2>/dev/null)
    if [ -n "$RESPONSE" ]; then
      echo "$RESPONSE"
    else
      echo '{}'
    fi
    ;;
  *)
    (echo "$INPUT" | nc -U -w 2 "$SOCKET_PATH" >/dev/null 2>&1) &
    echo '{}'
    ;;
esac
