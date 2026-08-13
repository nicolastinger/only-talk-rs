# quic_service crate

实时通信服务层（quinn + rustls），负责客户端 QUIC 长连接、消息收发、群聊广播、NAT 打洞/P2P 转发、集群内节点间 QUIC 通信。不依赖 HTTP 层，可独立以 `quic_server` 二进制运行。

## 职责

- **外部 QUIC 服务**：客户端通过 4433 端口建立 QUIC 长连接；首包 JWT 认证并校验 token 与 uuid 一致；连接注册到本地 DashMap + Redis（TTL 7200s）
- **消息处理**：单聊消息落库 + 回 ACK + 投递（本地直发或经内部 QUIC 转发）；群聊消息独立扇出管道（存库 → Redis 查成员 → 本地投递 + 节点广播）
- **粘包/半包处理**：自研帧协议（bincode 头 + CRC 校验 + 缓冲区合并残缺包）
- **NAT 打洞**：UDP 19562-19565 端口提供 NAT 发现、P2P 请求转发（`nat_ip/`）
- **内部 QUIC 服务**：节点间 4434 端口通信，集群路由 + TTL 递减转发（`internal/`）
- **生命周期管理**：`ChatNode` 状态机（Uninitialized→Running→Stopped）、watch channel 优雅关闭、TLS 证书热更新监控（`tls_monitor`）

## 依赖

- `common`（实体模型、Redis、消息协议、JWT）、`entity`
- 不依赖 `http_service` / `s3_service` / `email_service`

## 结构

```
quic_service/src/
├── init_server.rs        # start_server()：组装 ChatNode + NAT UDP + 内部 QUIC + 集群注册
├── lib.rs
├── bin/quic_server.rs    # 独立二进制入口（拆分部署时运行）
├── external/             # 外部客户端接入：chat_node（生命周期）、quic_server（连接处理）、
│                         #   quic_client、set_server（endpoint 构建）、tls_monitor（证书热更新）、config
├── internal/             # 节点间通信：internal_quic_server、internal_router（集群路由转发）
├── msg_service/          # 消息业务：text_msg_service（帧解析）、process_msg_service（分发）、
│                         #   group_msg_service（群聊扇出）、send_msg
├── models/               # first_quic_msg（首包）、quic_connection（连接表）、text_msg
└── nat_ip/               # NAT 发现 + P2P 请求转发 UDP 服务
```

## 关键约定

- 消息序列化（bincode + CRC）必须与 `only-talk-app/src-tauri` 客户端保持同步。
- 依赖注入：入口 `start_server()` 构造 `common::CoreState { db, redis }` 一路下传；`ChatNode` 持有 `CoreState`，跨 `tokio::spawn` 处 clone 传递；服务函数以 `&CoreState` / `&RBatis` / `&Pool` 窄签名使用连接，不访问任何全局单例。
- 连接 key 形如 `平台:QUIC:SERVER:uuid:消息类型`（大写），Redis 中也以相同 key 记录所属节点。
- 高吞吐路径（粘包缓冲合并）有 O(n²) 隐患；`text_msg_service` 有 25+ 单元测试覆盖帧解析。
- 端口见 `config/app_config.toml`：4433 外部、4434 内部、19562-19565 NAT UDP。

## 部署形态

- 单体模式：由 `src/main.rs` 调用 `start_server()`
- 拆分模式：`cargo run -p quic_service --bin quic_server` 独立进程
