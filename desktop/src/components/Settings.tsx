import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  AlertIcon,
  CheckIcon,
  SettingsIcon,
  SparkleIcon,
  TerminalIcon,
  TrashIcon,
} from "./Icons";

interface HookStatus {
  claude_code: boolean;
  codex: boolean;
}

function Settings() {
  const [hookStatus, setHookStatus] = useState<HookStatus>({ claude_code: false, codex: false });
  const [loading, setLoading] = useState<string | null>(null);
  const [message, setMessage] = useState<{ text: string; type: "success" | "error" } | null>(null);

  const refreshStatus = useCallback(async () => {
    try {
      const status = await invoke<HookStatus>("get_hook_status");
      setHookStatus(status);
    } catch (e) {
      console.error("Failed to get hook status:", e);
    }
  }, []);

  useEffect(() => {
    refreshStatus();
  }, [refreshStatus]);

  const showMessage = (text: string, type: "success" | "error") => {
    setMessage({ text, type });
    setTimeout(() => setMessage(null), 4000);
  };

  const handleInstall = async (tool: string) => {
    setLoading(tool);
    try {
      const result = await invoke<string>("install_hooks", { tool });
      showMessage(result, "success");
      await refreshStatus();
    } catch (e) {
      showMessage(String(e), "error");
    } finally {
      setLoading(null);
    }
  };

  const handleUninstall = async () => {
    setLoading("uninstall");
    try {
      const result = await invoke<string>("uninstall_hooks");
      showMessage(result, "success");
      await refreshStatus();
    } catch (e) {
      showMessage(String(e), "error");
    } finally {
      setLoading(null);
    }
  };

  return (
    <div className="settings-page">
      <h2>
        <SettingsIcon size={15} />
        Hook 拦截管理
      </h2>
      <p className="settings-desc">
        安装 Hook 后，AI 工具的危险操作将通过手机审批。低风险操作（如读取文件）自动放行。
      </p>

      {message && (
        <div className={`settings-msg ${message.type}`} role="alert">
          <span className="settings-msg-icon">
            {message.type === "success" ? <CheckIcon size={14} /> : <AlertIcon size={14} />}
          </span>
          <span>{message.text}</span>
        </div>
      )}

      <div className="hook-cards">
        <div className="hook-card">
          <div className="hook-card-header">
            <div className="hook-card-title">
              <div className="hook-card-icon">
                <SparkleIcon size={18} />
              </div>
              Claude Code
            </div>
            <span
              className={`hook-status-badge ${hookStatus.claude_code ? "installed" : ""}`}
            >
              {hookStatus.claude_code && <CheckIcon size={11} />}
              {hookStatus.claude_code ? "已安装" : "未安装"}
            </span>
          </div>
          <p className="hook-card-desc">
            拦截 PreToolUse、PostToolUse、Notification、Stop、SessionStart、SessionEnd 事件
          </p>
          <code className="hook-card-path">~/.claude/settings.json</code>
          <button
            className="btn-hook"
            disabled={loading !== null}
            onClick={() => handleInstall("claude_code")}
          >
            {loading === "claude_code"
              ? "安装中..."
              : hookStatus.claude_code
              ? "重新安装"
              : "安装拦截"}
          </button>
        </div>

        <div className="hook-card">
          <div className="hook-card-header">
            <div className="hook-card-title">
              <div className="hook-card-icon">
                <TerminalIcon size={18} />
              </div>
              Codex
            </div>
            <span className={`hook-status-badge ${hookStatus.codex ? "installed" : ""}`}>
              {hookStatus.codex && <CheckIcon size={11} />}
              {hookStatus.codex ? "已安装" : "未安装"}
            </span>
          </div>
          <p className="hook-card-desc">拦截 PreToolUse、PostToolUse、Notification、Stop 事件</p>
          <code className="hook-card-path">~/.codex/hooks.json</code>
          <button
            className="btn-hook"
            disabled={loading !== null}
            onClick={() => handleInstall("codex")}
          >
            {loading === "codex" ? "安装中..." : hookStatus.codex ? "重新安装" : "安装拦截"}
          </button>
        </div>
      </div>

      {(hookStatus.claude_code || hookStatus.codex) && (
        <div className="uninstall-section">
          <button className="btn-uninstall" disabled={loading !== null} onClick={handleUninstall}>
            <TrashIcon size={14} />
            {loading === "uninstall" ? "卸载中..." : "卸载所有拦截"}
          </button>
        </div>
      )}
    </div>
  );
}

export default Settings;
