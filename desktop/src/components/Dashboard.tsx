import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  ActivityIcon,
  AlertIcon,
  CheckIcon,
  DeviceIcon,
  FileIcon,
  GitIcon,
  InfoIcon,
  PackageIcon,
  PlugIcon,
  ShieldIcon,
  SparkleIcon,
  TerminalIcon,
  TrashIcon,
  XIcon,
  ZapIcon,
} from "./Icons";

interface FileChange {
  path: string;
  change_type: string;
  diff: string;
  additions: number;
  deletions: number;
}

interface AuthRequest {
  request_id: string;
  tool_name: string;
  operation: string;
  risk_level: string;
  summary: string;
  file_count: number;
  files?: FileChange[];
  raw_command?: string;
  timeout_seconds: number;
}

interface DashboardProps {
  pendingRequests: AuthRequest[];
  connectedCount: number;
  onMockRequest: () => void;
}

const toolLabels: Record<string, string> = {
  claude_code: "Claude Code",
  aider: "Aider",
  codex: "Codex",
};

const operationLabels: Record<string, string> = {
  file_write: "写入文件",
  file_delete: "删除文件",
  shell_execute: "执行命令",
  git_push: "Git Push",
  package_install: "安装包",
  config_modify: "修改配置",
};

const riskLabels: Record<string, string> = {
  low: "LOW",
  medium: "MEDIUM",
  high: "HIGH",
  critical: "CRITICAL",
};

function OperationIcon({ operation }: { operation: string }) {
  const size = 12;
  switch (operation) {
    case "file_write":
      return <FileIcon size={size} />;
    case "file_delete":
      return <TrashIcon size={size} />;
    case "shell_execute":
      return <TerminalIcon size={size} />;
    case "git_push":
      return <GitIcon size={size} />;
    case "package_install":
      return <PackageIcon size={size} />;
    default:
      return <InfoIcon size={size} />;
  }
}

function RiskIcon({ level }: { level: string }) {
  const size = 11;
  if (level === "critical" || level === "high") return <AlertIcon size={size} />;
  if (level === "medium") return <InfoIcon size={size} />;
  return <ShieldIcon size={size} />;
}

function formatToolName(name: string): string {
  if (!name) return "Unknown";
  if (name.includes(":")) {
    const [source, tool] = name.split(":", 2);
    return `${toolLabels[source] || source} · ${tool}`;
  }
  return toolLabels[name] || name;
}

function Dashboard({ pendingRequests, connectedCount, onMockRequest }: DashboardProps) {
  const [expanded, setExpanded] = useState<Set<string>>(new Set());
  const safeRisk = (level: string) =>
    ["low", "medium", "high", "critical"].includes(level) ? level : "medium";

  const toggleExpand = (id: string) => {
    setExpanded((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };

  const [busy, setBusy] = useState<Set<string>>(new Set());

  const handleDecide = async (id: string, decision: "approve" | "reject") => {
    if (busy.has(id)) return;
    setBusy((prev) => new Set(prev).add(id));
    try {
      await invoke(decision === "approve" ? "approve_request" : "reject_request", {
        requestId: id,
      });
    } catch (e) {
      console.error(`Failed to ${decision}:`, e);
      setBusy((prev) => {
        const next = new Set(prev);
        next.delete(id);
        return next;
      });
    }
  };

  return (
    <div className="dashboard">
      <div className="stats-row">
        <div className={`stat-card ${connectedCount > 0 ? "success" : ""}`}>
          <div className="stat-info">
            <div className="stat-label">已连接设备</div>
            <div className="stat-value">{connectedCount}</div>
          </div>
          <div className="stat-icon">
            <DeviceIcon size={20} />
          </div>
        </div>
        <div className={`stat-card ${pendingRequests.length > 0 ? "accent" : ""}`}>
          <div className="stat-info">
            <div className="stat-label">待授权请求</div>
            <div className="stat-value">{pendingRequests.length}</div>
          </div>
          <div className="stat-icon">
            <ZapIcon size={20} />
          </div>
        </div>
      </div>

      <div className="section">
        <div className="section-header">
          <h2>
            <ActivityIcon size={15} />
            待授权请求
          </h2>
          <button
            className="btn-mock"
            onClick={onMockRequest}
            aria-label="发送模拟请求用于测试"
          >
            <SparkleIcon size={13} />
            模拟请求
          </button>
        </div>

        {pendingRequests.length === 0 ? (
          <div className="empty-state" role="status">
            <div className="empty-icon">
              <ShieldIcon size={26} />
            </div>
            <div className="empty-title">一切安全，放心摸鱼</div>
            <div className="empty-hint">
              AI 工具执行危险操作时会在此等待，由你的手机决定放行或拒绝
            </div>
          </div>
        ) : (
          <div className="request-list">
            {pendingRequests.map((req) => {
              const risk = safeRisk(req.risk_level);
              const isExpanded = expanded.has(req.request_id);
              const hasDetails =
                (req.files && req.files.length > 0) || (req.raw_command && req.raw_command.length > 0);
              return (
                <div
                  key={req.request_id}
                  className={`request-card risk-${risk}`}
                >
                  <div className="request-header">
                    <span className="tool-badge">
                      <PlugIcon size={11} />
                      {formatToolName(req.tool_name)}
                    </span>
                    <span className={`risk-badge risk-${risk}`}>
                      <RiskIcon level={risk} />
                      {riskLabels[risk] || risk.toUpperCase()}
                    </span>
                  </div>
                  <div className="request-summary">{req.summary}</div>
                  <div className="request-meta">
                    <span className="meta-item">
                      <OperationIcon operation={req.operation} />
                      {operationLabels[req.operation] || req.operation}
                    </span>
                    <span className="meta-item">
                      <FileIcon size={12} />
                      {req.file_count} 个文件
                    </span>
                    <span className="meta-item">
                      <ClockSmall />
                      {req.timeout_seconds}s
                    </span>
                  </div>
                  {hasDetails && (
                    <button
                      className="details-toggle"
                      onClick={() => toggleExpand(req.request_id)}
                      aria-expanded={isExpanded}
                    >
                      <ChevronIcon expanded={isExpanded} />
                      {isExpanded ? "收起详情" : "查看详情"}
                    </button>
                  )}
                  {hasDetails && isExpanded && (
                    <div className="request-details">
                      {req.raw_command && (
                        <div className="detail-block">
                          <div className="detail-label">命令</div>
                          <pre className="code-block">{req.raw_command}</pre>
                        </div>
                      )}
                      {req.files && req.files.length > 0 && (
                        <div className="detail-block">
                          <div className="detail-label">文件变更</div>
                          {req.files.map((f, i) => (
                            <div key={i} className="file-change">
                              <div className="file-header">
                                <span className="file-path">{f.path}</span>
                                <span className="file-stats">
                                  <span className="add">+{f.additions}</span>
                                  <span className="del">−{f.deletions}</span>
                                </span>
                              </div>
                              {f.diff && (
                                <pre className="diff-block">
                                  {f.diff.split("\n").map((line, j) => {
                                    const cls = line.startsWith("+")
                                      ? "diff-add"
                                      : line.startsWith("-")
                                      ? "diff-del"
                                      : line.startsWith("@")
                                      ? "diff-hunk"
                                      : "";
                                    return (
                                      <div key={j} className={cls}>
                                        {line || " "}
                                      </div>
                                    );
                                  })}
                                </pre>
                              )}
                            </div>
                          ))}
                        </div>
                      )}
                    </div>
                  )}
                  <div className="request-actions">
                    <button
                      className="btn-decision btn-reject"
                      disabled={busy.has(req.request_id)}
                      onClick={() => handleDecide(req.request_id, "reject")}
                      aria-label="在桌面端拒绝"
                    >
                      <XIcon size={13} />
                      拒绝
                    </button>
                    <button
                      className="btn-decision btn-approve"
                      disabled={busy.has(req.request_id)}
                      onClick={() => handleDecide(req.request_id, "approve")}
                      aria-label="在桌面端允许"
                    >
                      <CheckIcon size={13} />
                      允许
                    </button>
                  </div>
                </div>
              );
            })}
          </div>
        )}
      </div>
    </div>
  );
}

function ClockSmall() {
  return (
    <svg width={12} height={12} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={1.75} strokeLinecap="round" strokeLinejoin="round">
      <circle cx="12" cy="12" r="9" />
      <polyline points="12 7 12 12 15 14" />
    </svg>
  );
}

function ChevronIcon({ expanded }: { expanded: boolean }) {
  return (
    <svg
      width={12}
      height={12}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth={2}
      strokeLinecap="round"
      strokeLinejoin="round"
      style={{
        transform: expanded ? "rotate(90deg)" : "rotate(0deg)",
        transition: "transform 180ms cubic-bezier(0.16, 1, 0.3, 1)",
      }}
    >
      <polyline points="9 18 15 12 9 6" />
    </svg>
  );
}

export default Dashboard;
