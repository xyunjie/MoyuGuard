import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { CheckIcon, ClockIcon, InfoIcon, TrashIcon, XIcon } from "./Icons";

interface BackendLogEntry {
  id: string;
  timestamp: number;
  tool_name: string;
  summary: string;
  risk_level: string;
  operation: string;
  decision: string;
  reason: string;
}

const toolLabels: Record<string, string> = {
  claude_code: "Claude Code",
  aider: "Aider",
  codex: "Codex",
};

function formatToolName(name: string): string {
  if (!name) return "Unknown";
  if (name.includes(":")) {
    const [source, tool] = name.split(":", 2);
    return `${toolLabels[source] || source} · ${tool}`;
  }
  return toolLabels[name] || name;
}

function formatTime(ms: number): string {
  return new Date(ms).toLocaleString("zh-CN", {
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
    hour12: false,
  });
}

function AuthLog() {
  const [logs, setLogs] = useState<BackendLogEntry[]>([]);
  const [confirmingClear, setConfirmingClear] = useState(false);

  useEffect(() => {
    const refresh = async () => {
      try {
        const entries = await invoke<BackendLogEntry[]>("get_log_entries");
        setLogs(entries);
      } catch (e) {
        console.error("Failed to load logs:", e);
      }
    };
    refresh();

    const unlisten = listen<BackendLogEntry>("log-appended", (event) => {
      setLogs((prev) => [event.payload, ...prev]);
    });

    return () => {
      unlisten.then((f) => f());
    };
  }, []);

  const handleClear = async () => {
    try {
      await invoke("clear_log");
      setLogs([]);
    } catch (e) {
      console.error("Failed to clear:", e);
    } finally {
      setConfirmingClear(false);
    }
  };

  return (
    <div className="auth-log">
      <div className="auth-log-header">
        <h2>
          <ClockIcon size={15} />
          授权日志
          {logs.length > 0 && <span className="log-count">{logs.length}</span>}
        </h2>
        {logs.length > 0 && (
          <div className="log-clear-area">
            {confirmingClear ? (
              <span className="log-clear-confirm">
                <span className="log-clear-confirm-text">确认清空？</span>
                <button className="btn-mock btn-danger" onClick={handleClear}>确认</button>
                <button className="btn-mock" onClick={() => setConfirmingClear(false)}>取消</button>
              </span>
            ) : (
              <button className="btn-mock" onClick={() => setConfirmingClear(true)} aria-label="清空日志">
                <TrashIcon size={13} />
                清空
              </button>
            )}
          </div>
        )}
      </div>

      {logs.length === 0 ? (
        <div className="empty-state" role="status">
          <div className="empty-icon">
            <ClockIcon size={24} />
          </div>
          <div className="empty-title">暂无授权记录</div>
          <div className="empty-hint">已处理的授权请求会按时间倒序显示在这里，重启后保留</div>
        </div>
      ) : (
        <div className="log-list">
          {logs.map((log) => {
            const cls =
              log.decision === "approved"
                ? "approved"
                : log.decision === "rejected" || log.decision === "timeout"
                ? "rejected"
                : "info";
            return (
              <div key={log.id} className={`log-entry ${cls}`}>
                <div className="log-icon">
                  {cls === "approved" && <CheckIcon size={13} />}
                  {cls === "rejected" && <XIcon size={13} />}
                  {cls === "info" && <InfoIcon size={13} />}
                </div>
                <span className="log-time">{formatTime(log.timestamp)}</span>
                <div className="log-body">
                  <div className="log-headline">
                    <span className="log-tool">{formatToolName(log.tool_name)}</span>
                    <span className={`log-decision log-decision-${cls}`}>
                      {log.decision === "approved" ? "已批准" : log.decision === "rejected" ? "已拒绝" : log.decision === "timeout" ? "超时" : log.decision}
                    </span>
                  </div>
                  <div className="log-summary">{log.summary}</div>
                </div>
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}

export default AuthLog;
