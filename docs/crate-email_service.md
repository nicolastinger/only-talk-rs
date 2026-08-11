# email_service crate

邮件发送服务库，封装多个邮件服务商（阿里云、腾讯云、AWS SES、SMTP），提供统一发送接口、重试策略、熔断、连接池、故障转移与速率限制。

## 职责

- **统一入口**：`EmailManager` / `EmailManagerBuilder`，负责发送、重试、故障转移
- **多服务商**：`EmailProvider` trait + 具体实现（Aliyun / Tencent / AwsSes / Smtp），按优先级选择，支持故障转移
- **健壮性**：指数退避/固定间隔/自适应重试策略、熔断器（CircuitBreaker）、服务商连接池（ProviderPool）、速率限制
- **邮件模型**：`Email`、`EmailAddress`、`Attachment`、`SendResult`（含按收件人的结果/错误分类）

## 依赖

- 独立于项目其他 crate（不依赖 `common` / `entity` 等）
- `reqwest`（走 HTTPS API）、`tokio`、`thiserror`、`chrono`、`sha2`/`hmac`（腾讯云/阿里云签名）

## 结构

```
email_service/src/
├── lib.rs                # 模块声明 + 类型再导出
├── config/               # EmailServiceConfig（default_provider/providers/retry/pool/rate_limit）
├── error.rs              # EmailError / EmailResult
├── manager/              # email_manager（EmailManager/Builder）、provider_pool（服务商池）
├── models/               # email、email_address、attachment、send_result
├── pool.rs               # 连接池
└── providers/            # provider（EmailProvider trait）、retry_strategy、implementations/
                          #   （aliyun / tencent / aws_ses / smtp）
```

## 关键约定

- **当前未接入业务**：没有任何 crate 依赖它用于实际发信，只有 examples（`basic_usage` / `batch_send` / `with_attachments`）。
- 在 `http_service::AppState` 中以 `Arc<EmailManager>` 占位（空配置 `EmailServiceConfig::default()` 构建），后续接入真实提供商配置后即可在业务中调用。
- 服务商实现全部通过 HTTPS API（reqwest），非标准 SMTP 协议。
- 有完整的单元测试（attachment、地址解析、重试策略、错误类型）。

## 部署形态

仅作为库，无独立可执行入口。当前是"预留能力"，尚未被业务路径调用。
