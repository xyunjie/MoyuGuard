# MoyuGuard 通信协议

## 传输层

- **协议**: WebSocket (二进制帧)
- **序列化**: Protocol Buffers 3
- **端口**: 9876 (默认)
- **服务发现**: mDNS `_moyuguard._tcp.local`

## 消息格式

所有消息使用 `Envelope` 包裹：

| 字段 | 类型 | 说明 |
|------|------|------|
| message_id | string | UUID v4 |
| timestamp | uint64 | Unix 毫秒时间戳 |
| type | MessageType | 消息类型枚举 |
| payload | oneof | 具体消息体 |

## 消息类型

| 类型 | 方向 | 说明 |
|------|------|------|
| PairRequest | 手机→电脑 | 配对请求 |
| PairResponse | 电脑→手机 | 配对响应 |
| AuthorizationRequest | 电脑→手机 | 授权请求 |
| AuthorizationResponse | 手机→电脑 | 授权决策 |
| Heartbeat | 双向 | 连接保活 (30s间隔) |
| StatusSync | 电脑→手机 | 状态同步 |

## 风险等级

| 等级 | 颜色 | 典型操作 |
|------|------|----------|
| LOW | 绿色 | 读取文件、查看状态 |
| MEDIUM | 黄色 | 修改配置文件 |
| HIGH | 橙色 | 删除文件、执行 Shell 命令 |
| CRITICAL | 红色 | git push --force、rm -rf |

## 超时策略

- 默认超时: 60 秒
- 超时后: 自动拒绝 (Decision.TIMEOUT)
- 心跳间隔: 30 秒
- 连接断开阈值: 90 秒无心跳
