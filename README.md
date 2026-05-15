# 摸鱼守卫 MoyuGuard

电脑交给 AI 干活，手机替你把关，安心摸鱼。

电脑端守护程序 + 手机端授权控制器，局域网实时通信。

## 架构

| 项目 | 技术栈 | 职责 |
|------|--------|------|
| `desktop/` | Tauri 2.0 (Rust + React) | WebSocket 服务、mDNS 广播、AI 工具拦截、授权等待 |
| `mobile/` | Flutter + 原生插件 | 局域网发现、配对绑定、授权决策、Diff 预览 |
| `proto/` | Protocol Buffers | 共享通信协议定义 |

## 开发

```bash
# 电脑端
cd desktop && pnpm install && cargo tauri dev

# 手机端
cd mobile && flutter run
```

## 许可证

MIT
