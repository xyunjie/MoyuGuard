# 摸鱼守卫 MoyuGuard

> 电脑交给 AI 干活，手机替你把关，安心摸鱼。

MoyuGuard 是一个电脑端守护程序 + 手机端授权控制器，通过局域网实时通信。当 AI 工具（Claude Code、Codex 等）执行危险操作时，自动弹出授权请求到你的手机，你在手机上点允许/拒绝，电脑端收到决策后放行或拦截。

## 特性

- **实时拦截**：AI 工具的危险操作（写文件、删文件、执行命令、Git Push 等）会被拦截，等待你授权
- **手机决策**：通过 WebSocket 将请求推送到手机，手机在局域网任何位置都能审批
- **风险分级**：自动识别操作风险（Low / Medium / High / Critical），Critical 级别带脉冲告警
- **Diff 预览**：文件修改操作展示完整 diff，命令执行展示完整命令行
- **后台守护**：关窗后在系统托盘继续运行，新请求触发原生通知
- **持久化日志**：所有授权记录保存在 `~/.moyuguard/state.json`，重启后可查
- **双协议支持**：手机原生 App（Protobuf 二进制）和浏览器模拟器（JSON）都能连接

## 架构

| 项目 | 技术栈 | 职责 |
|------|--------|------|
| `desktop/` | Tauri 2 (Rust + React/TS) | WebSocket 服务 (9876)、mDNS 广播、Unix Socket Hook 服务器、Hook 安装器、系统托盘、原生通知 |
| `mobile/` | Flutter + 原生插件 | 局域网发现、配对绑定、授权决策、Diff 预览（待完成） |
| `mobile-web-sim/` | 纯 HTML/JS | Chrome 手机模拟器，开发阶段替代 Flutter |
| `proto/` | Protocol Buffers | 共享通信协议定义 |

```
┌─────────────────────────────────────────────────────────────────┐
│                      Claude Code / Codex                        │
│  (触发 Hook → moyuguard-hook.sh → Unix Socket → 桌面端)         │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                  MoyuGuard Desktop (Tauri)                      │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐          │
│  │ Hook Server  │  │ WS Server    │  │ mDNS         │          │
│  │ (Unix Socket)│  │ (:9876)      │  │ Discovery    │          │
│  └──────────────┘  └──────────────┘  └──────────────┘          │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐          │
│  │ AuthManager  │  │ Tray Menu    │  │ Notification │          │
│  │ + LogStore   │  │ (Show/Quit)  │  │ (Permission) │          │
│  └──────────────┘  └──────────────┘  └──────────────┘          │
└─────────────────────────────────────────────────────────────────┘
                              │
              ┌───────────────┼───────────────┐
              │               │               │
              ▼               ▼               ▼
     ┌─────────────┐ ┌─────────────┐ ┌─────────────┐
     │  Flutter    │ │  Chrome     │ │  Future:    │
     │  Mobile App │ │  Simulator  │ │  iOS Island │
     └─────────────┘ └─────────────┘ └─────────────┘
```

## 快速开始

### 1. 电脑端安装

```bash
# 克隆
git clone https://github.com/xyunjie/MoyuGuard.git
cd MoyuGuard/desktop

# 安装依赖
pnpm install

# 启动开发版（Vite + Tauri 窗口）
pnpm tauri dev
```

窗口启动后显示：
- WebSocket 监听 `ws://0.0.0.0:9876`
- Unix Socket `/tmp/moyuguard-$UID.sock`
- mDNS 服务 `_moyuguard._tcp.local`

### 2. 安装 Hook 拦截

在桌面窗口点 **设置** → **安装拦截**（Claude Code 或 Codex）：

- 会在 `~/.claude/settings.json` 或 `~/.codex/hooks.json` 注入 Hook 配置
- 备份原文件到 `.moyuguard.bak`，卸载时自动恢复
- 安装后 AI 工具的 `PermissionRequest` 事件会实时拦截（已在运行的会话也生效）

### 3. 手机连接

**方式 A：Chrome 模拟器（推荐开发阶段）**

```bash
cd ../mobile-web-sim
python3 -m http.server 8181
# 浏览器打开 http://localhost:8181/
# 输入 IP（如 192.168.1.x）和端口 9876，点连接
```

**方式 B：Flutter 真机（待完成）**

```bash
cd ../mobile
flutter run
```

### 4. 测试拦截

在桌面窗口点 **模拟请求**，手机会收到一张授权卡片：
- 工具名：`claude_code:Edit`
- 风险等级：MEDIUM
- 摘要：`编辑文件：src/main.rs`
- Diff：`-let db_url = "localhost";` → `+let db_url = "production.db.example.com";`

点 **允许** → 桌面窗口卡片消失，日志显示已批准。

## 使用方法

### 日常流程

1. 启动电脑端（或从托盘唤醒）
2. 手机连接（Chrome 模拟器或 Flutter App）
3. 正常用 Claude Code / Codex
4. 遇到危险操作 → 手机弹窗 → 点允许/拒绝
5. 电脑端托盘图标显示待审批数量

### 卸载 Hook

在桌面窗口 **设置** → **卸载所有拦截**：
- 从 `~/.claude/settings.json` 移除 MoyuGuard 条目
- 恢复备份的原始配置（如 CodeIsland 的 Hook）
- 删除备份文件

### 日志查看

在桌面窗口点 **授权日志**：
- 显示所有已处理的请求（最新在前）
- 支持清空（不可恢复）
- 持久化文件：`~/.moyuguard/state.json`（最多 500 条）

## 技术细节

### Hook 拦截原理

1. **安装阶段**：修改 AI 工具的配置文件，注入 Hook 脚本路径
   - Claude Code：`~/.claude/settings.json` 的 `hooks` 字段
   - Codex：`~/.codex/hooks.json` + `config.toml` 启用 `hooks = true`

2. **触发阶段**：AI 工具调用危险命令前执行 Hook 脚本
   - `PermissionRequest` 事件：Claude Code 实时查配置文件，**已运行的会话也能拦截**
   - `PreToolUse` 事件：仅新会话生效（配置文件缓存）

3. **决策阶段**：Hook 脚本通过 Unix Socket 发送到桌面端 → WebSocket 推送到手机 → 用户决策 → 原路返回

### 通信协议

- **Protobuf**：原生客户端（未来 Flutter App）使用二进制协议，定义在 `proto/moyuguard.proto`
- **JSON**：浏览器客户端（mobile-web-sim）使用 JSON 协议，桌面端自动检测并转换

### 风险分级逻辑

| 条件 | 风险等级 | 行为 |
|------|---------|------|
| `Read` / `Glob` / `Grep` | Low | 自动放行，不弹窗 |
| `Bash` + 含 `rm -rf` / `sudo` / `chmod 777` | Critical | 弹窗 + 脉冲告警 |
| `Bash` 其他命令 | High | 弹窗 |
| `Edit` / `Write` 修改 `.env` / `credentials` / `.pem` | High | 弹窗 |
| `Edit` / `Write` 普通代码文件 | Medium | 弹窗 |

## 开发指南

### 构建 Release

```bash
cd desktop
pnpm tauri build
# 输出在 desktop/src-tauri/target/release/
```

### 调试 Hook

```bash
# 手动发送 Hook 事件测试桌面端
echo '{"event_name":"PermissionRequest","session_id":"test","tool_name":"Bash","tool_input":{"command":"echo test"}}' | \
  nc -U -w 30 /tmp/moyuguard-$UID.sock
```

### 目录结构

```
MoyuGuard/
├── desktop/
│   ├── src-tauri/
│   │   ├── src/
│   │   │   ├── lib.rs           # Tauri 入口 + 托盘 + 菜单
│   │   │   ├── hook_server.rs   # Unix Socket 服务端
│   │   │   ├── hook_installer.rs
│   │   │   ├── ws_server.rs     # WebSocket 服务端
│   │   │   ├── auth.rs          # 授权管理器
│   │   │   ├── log_store.rs     # 日志持久化
│   │   │   └── mdns.rs
│   │   └── icons/
│   ├── src/
│   │   ├── App.tsx
│   │   ├── components/
│   │   │   ├── Dashboard.tsx    # 卡片 + Diff 预览
│   │   │   ├── AuthLog.tsx      # 持久化日志
│   │   │   ├── Settings.tsx     # Hook 安装/卸载
│   │   │   └── Icons.tsx
│   │   └── App.css
│   └── scripts/moyuguard-hook.sh
├── mobile/                       # Flutter (待完成)
├── mobile-web-sim/index.html     # Chrome 模拟器
└── proto/moyuguard.proto
```

## 常见问题

**Q: 装完 Hook 后当前 Claude 会话没反应？**  
A: MoyuGuard 使用 `PermissionRequest` 事件，该事件 Claude 会实时查配置文件，理论上已运行会话也生效。如果没拦到，确认：
- `~/.claude/settings.json` 的 `hooks.PermissionRequest` 已注入
- 手机已连接到桌面端 WebSocket
- 托盘图标显示有待审批数量

**Q: 卸载 Hook 后 CodeIsland 也跟着没了？**  
A: MoyuGuard 卸载时会恢复备份文件。如果 CodeIsland 没回来，手动运行一次 CodeIsland 的 Reinstall Hooks。

**Q: 手机连不上？**  
A: 检查：
- 电脑防火墙是否放行 9876 端口
- 手机和电脑在同一局域网
- Chrome 模拟器用电脑 IP（不是 127.0.0.1）

**Q: 日志文件越来越大？**  
A: 自动限制 500 条，超出的 oldest 自动删除。

## License

MIT
