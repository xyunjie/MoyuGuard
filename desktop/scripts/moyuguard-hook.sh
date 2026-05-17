#!/bin/bash
# MoyuGuard Hook Script v3
# Reads CLI hook stdin JSON, sends to MoyuGuard desktop via Unix Socket.
# Blocks on PermissionRequest and forwards the desktop's response.
# All other events are fire-and-forget.

SOCKET_PATH="/tmp/moyuguard-$(id -u).sock"

# Read stdin into a temp file to avoid shell quoting issues.
TMPFILE=$(mktemp /tmp/moyuguard-hook-XXXXXX.json)
cat > "$TMPFILE"

if [ ! -s "$TMPFILE" ]; then
  rm -f "$TMPFILE"
  echo '{}'
  exit 0
fi

if [ ! -S "$SOCKET_PATH" ]; then
  rm -f "$TMPFILE"
  echo '{}'
  exit 0
fi

# Extract event name — Claude Code sends "hook_event_name"; others send "event_name".
EVENT_NAME=$(python3 -c "
import sys, json
try:
    with open(sys.argv[1]) as f:
        d = json.load(f)
    print(d.get('hook_event_name') or d.get('event_name') or '')
except Exception:
    print('')
" "$TMPFILE" 2>/dev/null)

# Send payload to socket; shutdown write end so server sees EOF, then read response.
unix_rpc() {
    python3 -c "
import socket, sys
try:
    with open(sys.argv[1], 'rb') as f:
        payload = f.read()
    with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as s:
        s.settimeout(86400)
        s.connect(sys.argv[2])
        s.sendall(payload)
        s.shutdown(socket.SHUT_WR)
        chunks = []
        while True:
            chunk = s.recv(65536)
            if not chunk:
                break
            chunks.append(chunk)
        sys.stdout.buffer.write(b''.join(chunks))
except Exception as e:
    sys.stderr.write(str(e) + '\n')
" "$TMPFILE" "$SOCKET_PATH"
}

# Fire-and-forget version (timeout 2s, runs in background).
unix_send() {
    python3 -c "
import socket, sys
try:
    with open(sys.argv[1], 'rb') as f:
        payload = f.read()
    with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as s:
        s.settimeout(2)
        s.connect(sys.argv[2])
        s.sendall(payload)
        s.shutdown(socket.SHUT_WR)
        s.recv(1024)
except Exception:
    pass
" "$TMPFILE" "$SOCKET_PATH" &
}

case "$EVENT_NAME" in
  PermissionRequest)
    RESPONSE=$(unix_rpc)
    rm -f "$TMPFILE"
    if [ -n "$RESPONSE" ]; then
      echo "$RESPONSE"
    else
      echo '{}'
    fi
    ;;
  *)
    unix_send
    rm -f "$TMPFILE"
    echo '{}'
    ;;
esac
