import { CheckIcon, ClockIcon, InfoIcon, XIcon } from "./Icons";

interface LogEntry {
  id: string;
  type: "approved" | "rejected" | "info";
  time: string;
  text: string;
}

interface AuthLogProps {
  logs: LogEntry[];
}

function AuthLog({ logs }: AuthLogProps) {
  return (
    <div className="auth-log">
      <h2>
        <ClockIcon size={15} />
        授权日志
      </h2>
      {logs.length === 0 ? (
        <div className="empty-state" role="status">
          <div className="empty-icon">
            <ClockIcon size={24} />
          </div>
          <div className="empty-title">暂无授权记录</div>
          <div className="empty-hint">已处理的授权请求会按时间倒序显示在这里</div>
        </div>
      ) : (
        <div className="log-list">
          {logs.map((log) => (
            <div key={log.id} className={`log-entry ${log.type}`}>
              <div className="log-icon">
                {log.type === "approved" && <CheckIcon size={13} />}
                {log.type === "rejected" && <XIcon size={13} />}
                {log.type === "info" && <InfoIcon size={13} />}
              </div>
              <span className="log-time">{log.time}</span>
              <span className="log-text">{log.text}</span>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

export default AuthLog;
