import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  AlertIcon,
  CheckIcon,
  PlugIcon,
  SettingsIcon,
  SparkleIcon,
  TerminalIcon,
  TrashIcon,
  ZapIcon,
} from "./Icons";

interface HookStatus {
  claude_code: boolean;
  codex: boolean;
}

interface AppConfig {
  ws_port: number;
}

interface TrustedClient {
  device_id: string;
  device_name: string;
  platform: string;
  paired_at: number;
}

const DEFAULT_PORT = 9876;

function PlatformLabel({ platform }: { platform: string }) {
  const icons: Record<string, string> = { ios: "📱 iOS", android: "🤖 Android", web: "🌐 Web", mobile: "📱 Mobile" };
  return <span>{icons[platform] ?? `📱 ${platform}`}</span>;
}

function Settings() {
  const [hookStatus, setHookStatus] = useState<HookStatus>({ claude_code: false, codex: false });
  const [loading, setLoading] = useState<string | null>(null);
  const [message, setMessage] = useState<{ text: string; type: "success" | "error" } | null>(null);

  const [wsPort, setWsPort] = useState<number>(DEFAULT_PORT);
  const [portInput, setPortInput] = useState<string>(String(DEFAULT_PORT));
  const [autostart, setAutostart] = useState<boolean>(false);
  const [trustedClients, setTrustedClients] = useState<TrustedClient[]>([]);

  const refreshStatus = useCallback(async () => {
    try {
      const status = await invoke<HookStatus>("get_hook_status");
      setHookStatus(status);
    } catch (e) {
      console.error("Failed to get hook status:", e);
    }
    try {
      const cfg = await invoke<AppConfig>("get_app_config");
      setWsPort(cfg.ws_port);
      setPortInput(String(cfg.ws_port));
    } catch (e) {
      console.error("Failed to get app config:", e);
    }
    try {
      const enabled = await invoke<boolean>("get_autostart_enabled");
      setAutostart(enabled);
    } catch (e) {
      console.error("Failed to get autostart status:", e);
    }
    try {
      const clients = await invoke<TrustedClient[]>("get_trusted_clients");
      setTrustedClients(clients);
    } catch (e) {
      console.error("Failed to get trusted clients:", e);
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

  const handleSavePort = async () => {
    const port = parseInt(portInput, 10);
    if (Number.isNaN(port) || port < 1024 || port > 65535) {
      showMessage("端口必须是 1024 - 65535 的数字", "error");
      setPortInput(String(wsPort));
      return;
    }
    if (port === wsPort) return;
    setLoading("port");
    try {
      await invoke("save_app_config", { config: { ws_port: port } });
      setWsPort(port);
      showMessage(`端口已保存为 ${port}，下次启动生效`, "success");
    } catch (e) {
      showMessage(String(e), "error");
      setPortInput(String(wsPort));
    } finally {
      setLoading(null);
    }
  };

  const handleAutostartToggle = async () => {
    const target = !autostart;
    setLoading("autostart");
    try {
      await invoke("set_autostart_enabled", { enabled: target });
      setAutostart(target);
      showMessage(target ? "已设为开机自启" : "已取消开机自启", "success");
    } catch (e) {
      showMessage(String(e), "error");
    } finally {
      setLoading(null);
    }
  };

  const handleRemoveTrusted = async (deviceId: string, deviceName: string) => {
    try {
      await invoke("remove_trusted_client", { deviceId });
      setTrustedClients((prev) => prev.filter((c) => c.device_id !== deviceId));
      showMessage(`已移除设备：${deviceName}`, "success");
    } catch (e) {
      showMessage(String(e), "error");
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
            <span className={`hook-status-badge ${hookStatus.claude_code ? "installed" : ""}`}>
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

      <h2 style={{ marginTop: 32 }}>
        <ZapIcon size={15} />
        应用设置
      </h2>
      <p className="settings-desc">应用级配置。端口修改需要重启 MoyuGuard 才生效。</p>

      <div className="config-row">
        <div className="config-row-label">
          <div className="config-row-title">
            <PlugIcon size={14} />
            WebSocket 端口
          </div>
          <div className="config-row-hint">
            手机端连接的端口，当前生效：<code>{wsPort}</code>
          </div>
        </div>
        <div className="config-row-control">
          <input
            type="number"
            className="port-input"
            min={1024}
            max={65535}
            value={portInput}
            onChange={(e) => setPortInput(e.target.value)}
            disabled={loading !== null}
          />
          <button
            className="btn-mock"
            disabled={loading !== null || parseInt(portInput, 10) === wsPort}
            onClick={handleSavePort}
          >
            {loading === "port" ? "保存中..." : "保存"}
          </button>
        </div>
      </div>

      <div className="config-row">
        <div className="config-row-label">
          <div className="config-row-title">
            <SparkleIcon size={14} />
            开机自启
          </div>
          <div className="config-row-hint">登录系统后自动启动 MoyuGuard 守护进程</div>
        </div>
        <div className="config-row-control">
          <button
            className={`toggle ${autostart ? "on" : ""}`}
            disabled={loading !== null}
            onClick={handleAutostartToggle}
            aria-pressed={autostart}
            aria-label="切换开机自启"
          >
            <span className="toggle-thumb" />
          </button>
        </div>
      </div>

      <h2 style={{ marginTop: 32 }}>
        <CheckIcon size={15} />
        已信任设备
      </h2>
      <p className="settings-desc">
        以下设备已完成配对，可以直接连接并审批操作。移除后需重新配对。
      </p>

      {trustedClients.length === 0 ? (
        <div className="trusted-empty">暂无已信任设备</div>
      ) : (
        <div className="trusted-list">
          {trustedClients.map((c) => (
            <div key={c.device_id} className="trusted-item">
              <div className="trusted-item-info">
                <div className="trusted-item-name">{c.device_name}</div>
                <div className="trusted-item-meta">
                  <PlatformLabel platform={c.platform} />
                  <span> · {new Date(c.paired_at).toLocaleDateString()}</span>
                </div>
              </div>
              <button
                className="btn-trusted-remove"
                onClick={() => handleRemoveTrusted(c.device_id, c.device_name)}
                title="移除信任"
              >
                <TrashIcon size={13} />
              </button>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

export default Settings;
