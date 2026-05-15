#!/bin/bash
# MoyuGuard Hook Script v1
# Reads CLI hook stdin JSON, sends to MoyuGuard desktop via Unix Socket.
# For PreToolUse events, blocks until phone approval/denial.

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

EVENT_NAME=$(echo "$INPUT" | python3 -c "import sys,json; print(json.load(sys.stdin).get('event_name',''))" 2>/dev/null)

case "$EVENT_NAME" in
  PreToolUse|UserPromptSubmit)
    RESPONSE=$(echo "$INPUT" | nc -U -w 120 "$SOCKET_PATH" 2>/dev/null)
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
