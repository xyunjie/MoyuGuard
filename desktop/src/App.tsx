import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import Dashboard from "./components/Dashboard";
import AuthLog from "./components/AuthLog";
import Settings from "./components/Settings";
import { MoyuLogo, ActivityIcon, ClockIcon, SettingsIcon } from "./components/Icons";
import "./App.css";

type Tab = "dashboard" | "log" | "settings";

interface AuthRequest {
  request_id: string;
  tool_name: string;
  operation: string;
  risk_level: string;
  summary: string;
  file_count: number;
  timeout_seconds: number;
}

interface LogEntry {
  id: string;
  type: "approved" | "rejected" | "info";
  time: string;
  text: string;
}

function App() {
  const [tab, setTab] = useState<Tab>("dashboard");
  const [connectedCount, setConnectedCount] = useState(0);
  const [pendingRequests, setPendingRequests] = useState<AuthRequest[]>([]);
  const [resolvedLog, setResolvedLog] = useState<LogEntry[]>([]);

  useEffect(() => {
    const pollStatus = async () => {
      try {
        const count = await invoke<number>("get_connected_count");
        setConnectedCount(count);
        const pending = await invoke<AuthRequest[]>("get_pending_requests");
        setPendingRequests(pending);
      } catch (e) {
        console.error("Failed to poll status:", e);
      }
    };

    pollStatus();
    const interval = setInterval(pollStatus, 2000);

    const unlistenAuth = listen<AuthRequest>("auth-request", (event) => {
      setPendingRequests((prev) => [...prev, event.payload]);
    });

    const unlistenResolved = listen<string>("auth-resolved", (event) => {
      const id = event.payload;
      setPendingRequests((prev) => prev.filter((r) => r.request_id !== id));
      setResolvedLog((prev) => [
        {
          id,
          type: "approved",
          time: new Date().toLocaleTimeString("zh-CN", { hour12: false }),
          text: `请求 ${id.slice(0, 8)} 已处理`,
        },
        ...prev,
      ]);
    });

    const unlistenConnection = listen<{ connected_count: number }>(
      "connection-changed",
      (event) => {
        setConnectedCount(event.payload.connected_count);
      }
    );

    return () => {
      clearInterval(interval);
      unlistenAuth.then((f) => f());
      unlistenResolved.then((f) => f());
      unlistenConnection.then((f) => f());
    };
  }, []);

  const handleMockRequest = async () => {
    try {
      await invoke("send_mock_request");
    } catch (e) {
      console.error("Failed to send mock request:", e);
    }
  };

  return (
    <div className="app">
      <header className="app-header">
        <div className="brand">
          <div className="brand-icon">
            <MoyuLogo size={20} />
          </div>
          <h1 className="app-title">摸鱼守卫</h1>
        </div>
        <div className="status-bar">
          <div className={`status-pill ${connectedCount > 0 ? "connected" : ""}`}>
            <span className={`status-dot ${connectedCount > 0 ? "on" : ""}`} />
            {connectedCount > 0 ? `${connectedCount} 台已连接` : "等待连接"}
          </div>
          <span className="ws-port">ws://:9876</span>
        </div>
      </header>

      <nav className="tab-nav">
        <button
          className={tab === "dashboard" ? "active" : ""}
          onClick={() => setTab("dashboard")}
        >
          <ActivityIcon size={14} />
          监控面板
          {pendingRequests.length > 0 && (
            <span className="tab-count">{pendingRequests.length}</span>
          )}
        </button>
        <button
          className={tab === "log" ? "active" : ""}
          onClick={() => setTab("log")}
        >
          <ClockIcon size={14} />
          授权日志
        </button>
        <button
          className={tab === "settings" ? "active" : ""}
          onClick={() => setTab("settings")}
        >
          <SettingsIcon size={14} />
          设置
        </button>
      </nav>

      <main className="content">
        {tab === "dashboard" && (
          <Dashboard
            pendingRequests={pendingRequests}
            connectedCount={connectedCount}
            onMockRequest={handleMockRequest}
          />
        )}
        {tab === "log" && <AuthLog logs={resolvedLog} />}
        {tab === "settings" && <Settings />}
      </main>
    </div>
  );
}

export default App;
