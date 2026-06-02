# OnlyTalk RS

> High-Performance Instant Messaging Server — Rust + Actix-web + QUIC + PostgreSQL + Redis

[中文版](README.md)

## Overview

OnlyTalk RS is a high-performance instant messaging (IM) backend server built with Rust. It uses Actix-web for RESTful APIs and the QUIC protocol for real-time message transmission, with support for P2P hole punching, NAT traversal, group chat broadcasting, and other advanced IM features.

## Tech Stack

| Component | Technology | Description |
|-----------|------------|-------------|
| Web Framework | Actix-web 4.9 | HTTP/HTTPS API service |
| Protocol | QUIC (quinn 0.10) | Real-time messaging, P2P connections |
| Database | PostgreSQL + rbatis 4.6 | ORM-based data persistence |
| Cache | Redis + deadpool-redis | Connection pooling, session management |
| Object Storage | AWS S3 SDK | MinIO / Aliyun OSS / AWS S3 support |
| Auth | JWT (jsonwebtoken 8.0) | User authentication |
| Logging | tracing + tracing-subscriber | Structured logging, JSON output |
| Encryption | rustls 0.21 + rcgen | TLS certificate management |
| Rust Edition | 2024 | Latest language features |

## Workspace Structure

```
only-talk-rs/
├── src/main.rs                  # Standalone mode entry (starts QUIC + HTTP together)
├── crates/
│   ├── api/                     # API service layer (integrates HTTP + QUIC routes)
│   ├── common/                  # Shared utilities: config, JWT, RSA encryption, QUIC config, tracing
│   ├── entity/                  # Data models & DB operations (rbatis), DDL scripts
│   ├── email_service/           # Email service (SMTP via reqwest, with connection pool)
│   ├── http_service/            # HTTP endpoints: user, friend, group, chat, notify, file
│   ├── quic_service/            # QUIC service: external/internal connections, NAT hole punching, P2P forwarding
│   └── s3_service/              # S3 object storage abstraction (multi-provider support)
├── config/
│   ├── app_config.toml          # App configuration (DB, Redis, QUIC, files, S3)
│   └── ssl/                     # TLS certificates (fullchain.pem + privkey.pem)
├── docs/                        # Technical documentation
├── .env.example                 # Environment variable template
├── Cargo.toml                   # Workspace configuration
├── clippy.toml                  # Clippy rules
└── rustfmt.toml                 # Code formatting rules
```

## Quick Start

### Prerequisites

- **Rust**: 1.75+ (Edition 2024)
- **PostgreSQL**: 12+
- **Redis**: 6+
- **TLS Certificates**: Self-signed or production certs (placed in `config/ssl/`)

### 1. Clone the Repository

```bash
git clone <repository-url>
cd only-talk-rs
```

### 2. Configure Environment Variables

```bash
cp .env.example .env
```

Edit `.env` with your actual values:

```env
# Database
DATABASE_URL=postgres://postgres:YOUR_PASSWORD@127.0.0.1:5432/postgres

# Redis
REDIS_URL=redis://:YOUR_PASSWORD@127.0.0.1:6379/

# S3 Object Storage (optional)
S3_ENDPOINT=http://127.0.0.1:9000
S3_ACCESS_KEY=your-access-key
S3_SECRET_KEY=your-secret-key
```

### 3. Configure the Application

Edit `config/app_config.toml` and adjust key settings:

```toml
[server]
address = "0.0.0.0:8443"        # HTTPS API listen address
log_level = "info"              # Log level: debug / info / warn / error

[quic_server]
address = "0.0.0.0:4433"        # QUIC external port

[internal_quic_server]
address = "127.0.0.1:4434"      # QUIC internal communication port

[s3]
enabled = false                 # Enable S3 storage (false uses local storage)
provider = "minio"              # minio / aliyun_oss / aws_s3
```

### 4. Place TLS Certificates

Put TLS certificates in the `config/ssl/` directory:

```
config/ssl/fullchain.pem    # Certificate chain
config/ssl/privkey.pem      # Private key (RSA / EC / PKCS8 supported)
```

For development, you can use rcgen to auto-generate self-signed certificates.

### 5. Initialize the Database

Run the DDL scripts in `crates/entity/` to create database tables.

### 6. Start the Server

```bash
# Development mode
cargo run

# With specific log level
RUST_LOG=debug cargo run

# Release mode
cargo run --release
```

After startup, the server runs:
- **HTTPS API**: `https://0.0.0.0:8443`
- **QUIC External**: `0.0.0.0:4433`
- **QUIC Internal**: `127.0.0.1:4434`

## Deployment Modes

| Mode | Entry | Ports | Description |
|------|-------|-------|-------------|
| **Standalone** | `src/main.rs` | 8443 + 4433 + 4434 | QUIC + HTTP in a single process |
| **QUIC Gateway** | `crates/quic_service/src/bin/quic_server.rs` | 4433 + 4434 | QUIC connection management only |
| **API Service** | `crates/api/src/main.rs` | 8443 | HTTP REST API only |

### Standalone Mode (Default)

```bash
cargo run --release
```

QUIC and HTTP services run in the same process, suitable for small to medium deployments.

### Separated Deployment

For large-scale cluster deployments, run QUIC gateway and API service as independent processes.

## API Endpoints

### HTTP REST API

| Route Prefix | Module | Features |
|--------------|--------|----------|
| `/user` | user_service | Registration, login, profile management, password reset |
| `/friend` | friend_service | Friend requests, friend list, relationship management |
| `/group` | group_service | Group creation, member management, group message history |
| `/msg` | chat_service | Text messages, message queries, message roaming |
| `/file` | file_service | File upload/download, avatar management, S3 presigned URLs |
| `/notify` | notify_service | System notifications, push notifications |
| `/integrated` | api/controller | Integrated user services and file upload services |

### QUIC Protocol

- **External Connections**: Clients establish QUIC persistent connections on port `4433`
- **Internal Communication**: Server nodes communicate via port `4434`
- **NAT Hole Punching**: UDP ports `9562-9565` for P2P traversal

## Features

### Messaging

- Real-time message pushing via QUIC bidirectional streams
- Text messages, file messages, image previews
- Message CRC checksums for transmission integrity

### P2P Connections

- UDP NAT discovery (ports 9562-9565)
- P2P hole punching for direct connections
- Automatic fallback to server relay when P2P fails

### File Storage

- Seamless switching between local storage and S3 object storage
- Automatic image compression to WebP format
- Multipart upload for large files
- Presigned URL secure access

### Authentication & Security

- JWT Token authentication
- Argon2 password hashing
- RSA public-key encryption for sensitive data
- HTTPS (TLS 1.3) API encryption
- QUIC built-in TLS encryption

### Logging & Monitoring

- Structured logging (tracing + JSON output)
- TraceId full-link tracing
- Automatic log rotation
- Bad request auto-recording

## Configuration Reference

### app_config.toml

```toml
[database]
url = "${DATABASE_URL}"         # PostgreSQL connection string

[redis]
url = "${REDIS_URL}"            # Redis connection string

[server]
address = "0.0.0.0:8443"        # HTTPS listen address
locales = "zh-CN"               # Internationalization language
log_level = "info"              # Log level

[quic_server]
address = "0.0.0.0:4433"        # QUIC external address
cert_path = "./config/ssl/fullchain.pem"
key_path = "./config/ssl/privkey.pem"

[file_upload]
max_file_size = 20971520        # Maximum file upload size (20MB)

[s3]
enabled = true                  # Enable S3 storage
provider = "minio"              # Storage provider
endpoint = "${S3_ENDPOINT}"
access_key = "${S3_ACCESS_KEY}"
secret_key = "${S3_SECRET_KEY}"
default_bucket = "only-talk-rs"
```

### File Type Configuration

Supports images, documents, archives, audio, video and more file types. Each type supports configurable allowed extensions and MIME types.

## Development Guide

### Code Style

- `clippy::unwrap_used` is denied — use `expect("reason")` or proper error handling
- `rustfmt`: 4-space indentation, Unix newlines, grouped imports (StdExternalCrate)
- Error handling: `anyhow` for application errors, `thiserror` for domain errors

### Common Commands

```bash
# Build
cargo build --release

# Run
cargo run --release

# Clippy
cargo clippy --workspace

# Format
cargo fmt --all

# Tests
cargo test --workspace

# Benchmarks
cargo bench -p quic_service
```

### Adding New API Endpoints

1. Create a new module under `crates/http_service/src/http_service/`
2. Create a controller (handles HTTP requests)
3. Create a service (business logic)
4. Register routes in `mod.rs`

### Adding New QUIC Message Types

1. Define message types in `crates/quic_service/src/msg_service/`
2. Implement serialization/deserialization
3. Register with the message dispatcher

## Documentation

- [Deployment Guide](docs/启动部署指南.md)
- [Cluster Routing](docs/集群路由方案.md)
- [Group Chat Broadcasting](docs/群聊广播方案.md)
- [Entity Core Splitting](docs/entity-core拆分方案.md)

## License

[LICENSE](LICENSE)
