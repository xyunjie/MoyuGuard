import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import Dashboard from "./components/Dashboard";
import AuthLog from "./components/AuthLog";
import Settings from "./components/Settings";
import { MoyuLogo, ActivityIcon, ClockIcon, SettingsIcon } from "./components/Icons";
import "./App.css";

type Tab = "dashboard" | "log" | "settings";

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

interface PairPendingEvent {
  client_id: string;
  device_name: string;
  device_id: string;
  platform: string;
}

function PlatformIcon({ platform }: { platform: string }) {
  if (platform === "ios") return <span>📱</span>;
  if (platform === "android") return <span>🤖</span>;
  if (platform === "web") return <span>🌐</span>;
  return <span>📱</span>;
}

function PairDialog({
  request,
  onApprove,
  onReject,
}: {
  request: PairPendingEvent;
  onApprove: () => void;
  onReject: () => void;
}) {
  return (
    <div className="pair-overlay">
      <div className="pair-dialog">
        <div className="pair-dialog-icon">
          <PlatformIcon platform={request.platform} />
        </div>
        <h3 className="pair-dialog-title">配对请求</h3>
        <p className="pair-dialog-desc">
          一台新设备请求连接到 MoyuGuard
        </p>
        <div className="pair-dialog-device">
          <div className="pair-device-name">{request.device_name}</div>
          <div className="pair-device-meta">{request.platform} · {request.device_id.slice(0, 8)}…</div>
        </div>
        <p className="pair-dialog-warn">
          允许后该设备可以审批所有 AI 操作请求
        </p>
        <div className="pair-dialog-actions">
          <button className="btn-pair-reject" onClick={onReject}>
            拒绝
          </button>
          <button className="btn-pair-approve" onClick={onApprove}>
            允许配对
          </button>
        </div>
      </div>
    </div>
  );
}

function App() {
  const [tab, setTab] = useState<Tab>("dashboard");
  const [connectedCount, setConnectedCount] = useState(0);
  const [pendingRequests, setPendingRequests] = useState<AuthRequest[]>([]);
  const [pairQueue, setPairQueue] = useState<PairPendingEvent[]>([]);

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
    });

    const unlistenConnection = listen<{ connected_count: number }>(
      "connection-changed",
      (event) => {
        setConnectedCount(event.payload.connected_count);
      }
    );

    const unlistenPair = listen<PairPendingEvent>("pair-pending", (event) => {
      setPairQueue((prev) => [...prev, event.payload]);
    });

    return () => {
      clearInterval(interval);
      unlistenAuth.then((f) => f());
      unlistenResolved.then((f) => f());
      unlistenConnection.then((f) => f());
      unlistenPair.then((f) => f());
    };
  }, []);

  const handleMockRequest = async () => {
    try {
      await invoke("send_mock_request");
    } catch (e) {
      console.error("Failed to send mock request:", e);
    }
  };

  const handlePairApprove = async (req: PairPendingEvent) => {
    try {
      await invoke("approve_pair", { clientId: req.client_id });
    } catch (e) {
      console.error("Failed to approve pair:", e);
    }
    setPairQueue((prev) => prev.filter((r) => r.client_id !== req.client_id));
  };

  const handlePairReject = async (req: PairPendingEvent) => {
    try {
      await invoke("reject_pair", { clientId: req.client_id });
    } catch (e) {
      console.error("Failed to reject pair:", e);
    }
    setPairQueue((prev) => prev.filter((r) => r.client_id !== req.client_id));
  };

  const currentPair = pairQueue[0] ?? null;

  return (
    <div className="app">
      {currentPair && (
        <PairDialog
          request={currentPair}
          onApprove={() => handlePairApprove(currentPair)}
          onReject={() => handlePairReject(currentPair)}
        />
      )}

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
          className={`${tab === "settings" ? "active" : ""}`}
          onClick={() => setTab("settings")}
        >
          <SettingsIcon size={14} />
          设置
          {pairQueue.length > 0 && (
            <span className="tab-count">{pairQueue.length}</span>
          )}
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
        {tab === "log" && <AuthLog />}
        {tab === "settings" && <Settings />}
      </main>
    </div>
  );
}

export default App;
