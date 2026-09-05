//! QUIC 外网服务连接测试客户端(示例/调试工具)
//!
//! 用法(仓库根目录运行):
//! ```text
//! cargo run -p quic_service --example quic_probe -- \
//!     --addr <host:port> --token <jwt> [--uuid <uuid>] [--platform PC|MOBILE] \
//!     [--verify] [--sni <name>] [--ping-every-secs <n>] [--run-secs <n>] \
//!     [--to <uuid> --text <内容>]
//! ```
//! cargo run -p quic_service --example quic_probe -- --addr xx.xx.xxx.xx:xxx --token "ey..."
//! 连接成功后进入命令行交互:
//! - `say <目标uuid> <文本>`  发送一条单聊文本
//! - `ping`                    发送一次心跳(PING)
//! - `status`                  打印收发统计
//! - `quit` / `exit` / Ctrl-C  断开退出
//!
//! 后台持续打印服务端下发的所有 uni 流帧(文本/ACK/系统通知/踢下线等)。

use std::net::SocketAddr;
use std::process::exit;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use common::config_str::SYSTEM;
use common::utils::internal_quic_client::make_internal_client_config;
use common::utils::message_types::{
    MSG_TYPE_FILE, MSG_TYPE_FORCE_LOGOUT, MSG_TYPE_GROUP_ACK, MSG_TYPE_GROUP_FILE,
    MSG_TYPE_GROUP_IMAGE, MSG_TYPE_GROUP_NOTIFICATION, MSG_TYPE_GROUP_TEXT, MSG_TYPE_IMAGE,
    MSG_TYPE_P2P, MSG_TYPE_P2P_VIDEO_CALL, MSG_TYPE_P2P_VIDEO_CALL_ACCEPT,
    MSG_TYPE_P2P_VIDEO_CALL_END, MSG_TYPE_P2P_VIDEO_CALL_INVITE, MSG_TYPE_P2P_VIDEO_CALL_REJECT,
    MSG_TYPE_P2P_VIDEO_CONFIG, MSG_TYPE_P2P_VIDEO_DATA, MSG_TYPE_PING, MSG_TYPE_RECALL_FAILURE,
    MSG_TYPE_RECALL_SUCCESS, MSG_TYPE_SYSTEM, MSG_TYPE_TEXT, MSG_TYPE_WEBRTC_SIGNAL,
    NOTIFY_TYPE_MSG,
};
use common::utils::text_msg::{HeadMsg, TextQuicMsg, X25, generate_text_msg};
use quinn::{ClientConfig, Connection};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
use tokio::sync::mpsc;

// ---------------- 参数解析 ----------------

struct Args {
    addr: SocketAddr,
    token: String,
    uuid: String,
    platform: String,
    verify: bool,
    sni: String,
    ping_every_secs: u64,
    run_secs: u64,
    send_to: Option<String>,
    send_text: Option<String>,
}

fn usage() -> ! {
    eprintln!(
        "用法: quic_probe --addr <host:port> --token <jwt> [选项]\n\n\
         选项:\n  \
         --uuid <uuid>              覆盖 token 内解析出的 uuid(默认自动解析)\n  \
         --platform PC|MOBILE       覆盖 token 内解析出的平台(默认自动解析)\n  \
         --verify                   校验证书(默认跳过证书校验,便于连自签/本机服务端)\n  \
         --sni <name>               证书校验时使用的 SNI(默认取 --addr 的 host)\n  \
         --ping-every-secs <n>      心跳周期,默认 30,0=关闭\n  \
         --run-secs <n>             运行 n 秒后自动退出,0=不退出(默认)\n  \
         --to <uuid> --text <str>   连接成功后先发送一条单聊文本\n\n\
         交互命令: say <目标uuid> <文本> | ping | status | quit/exit"
    );
    exit(2);
}

fn parse_args() -> Args {
    let mut raw = std::env::args().skip(1);
    let mut addr_str: Option<String> = None;
    let mut token: Option<String> = None;
    let mut uuid_override: Option<String> = None;
    let mut platform_override: Option<String> = None;
    let mut verify = false;
    let mut sni: Option<String> = None;
    let mut ping_every_secs = 30u64;
    let mut run_secs = 0u64;
    let mut send_to: Option<String> = None;
    let mut send_text: Option<String> = None;

    let next = |raw: &mut dyn Iterator<Item = String>, name: &str| -> Option<String> {
        match raw.next() {
            Some(v) => Some(v),
            None => {
                eprintln!("缺少参数值: {name}");
                usage();
            }
        }
    };

    while let Some(arg) = raw.next() {
        match arg.as_str() {
            "--addr" => addr_str = next(&mut raw, "--addr"),
            "--token" => token = next(&mut raw, "--token"),
            "--uuid" => uuid_override = next(&mut raw, "--uuid"),
            "--platform" => platform_override = next(&mut raw, "--platform"),
            "--verify" => verify = true,
            "--sni" => sni = next(&mut raw, "--sni"),
            "--ping-every-secs" => {
                ping_every_secs = next(&mut raw, "--ping-every-secs")
                    .and_then(|v| v.parse().ok())
                    .unwrap_or_else(|| {
                        eprintln!("--ping-every-secs 需为数字");
                        usage();
                    });
            }
            "--run-secs" => {
                run_secs = next(&mut raw, "--run-secs")
                    .and_then(|v| v.parse().ok())
                    .unwrap_or_else(|| {
                        eprintln!("--run-secs 需为数字");
                        usage();
                    });
            }
            "--to" => send_to = next(&mut raw, "--to"),
            "--text" => send_text = next(&mut raw, "--text"),
            "--help" | "-h" => usage(),
            other => {
                eprintln!("未知参数: {other}");
                usage();
            }
        }
    }

    let Some(addr_str) = addr_str else {
        eprintln!("缺少 --addr");
        usage();
    };
    let Some(token) = token else {
        eprintln!("缺少 --token");
        usage();
    };

    let addr: SocketAddr = match addr_str.parse() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("--addr 解析失败 '{addr_str}': {e}");
            usage();
        }
    };

    let (token_uuid, token_sub) = match decode_jwt_payload(&token) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("--token 解析失败: {e}");
            usage();
        }
    };
    let uuid = uuid_override.unwrap_or(token_uuid);
    let platform = platform_override.unwrap_or(token_sub);
    if uuid.is_empty() {
        eprintln!("--token 缺少 uuid 声明, 请用 --uuid 指定");
        usage();
    }
    if platform.is_empty() {
        eprintln!("--token 缺少 sub 声明, 请用 --platform 指定");
        usage();
    }

    let sni = sni.unwrap_or_else(|| addr.ip().to_string());

    if send_text.is_some() && send_to.is_none() {
        eprintln!("--text 需要配合 --to 指定目标 uuid");
        usage();
    }
    if send_to.is_some() && send_text.is_none() {
        eprintln!("--to 需要配合 --text 指定发送内容");
        usage();
    }

    Args { addr, token, uuid, platform, verify, sni, ping_every_secs, run_secs, send_to, send_text }
}

/// base64url(无填充)解码
fn b64url_decode(input: &str) -> Result<Vec<u8>, String> {
    let map = |c: u8| -> Option<u8> {
        match c {
            b'A'..=b'Z' => Some(c - b'A'),
            b'a'..=b'z' => Some(c - b'a' + 26),
            b'0'..=b'9' => Some(c - b'0' + 52),
            b'-' => Some(62),
            b'_' => Some(63),
            _ => None,
        }
    };
    let mut out = Vec::with_capacity(input.len() * 3 / 4);
    let mut acc: u32 = 0;
    let mut bits = 0u32;
    for &b in input.as_bytes() {
        let Some(v) = map(b) else {
            return Err(format!("非 base64url 字符: {}", b as char));
        };
        acc = (acc << 6) | v as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
            acc &= (1 << bits) - 1;
        }
    }
    Ok(out)
}

/// 解析 JWT 中间 payload,返回 (uuid, sub/platform)
fn decode_jwt_payload(token: &str) -> Result<(String, String), String> {
    let segs: Vec<&str> = token.split('.').collect();
    if segs.len() < 2 {
        return Err("token 格式不正确(需为三段式 JWT)".to_string());
    }
    let payload = b64url_decode(segs[1])?;
    let v: serde_json::Value =
        serde_json::from_slice(&payload).map_err(|e| format!("payload 非 JSON: {e}"))?;
    let uuid = v.get("uuid").and_then(|x| x.as_str()).unwrap_or("").to_string();
    let sub = v.get("sub").and_then(|x| x.as_str()).unwrap_or("").to_string();
    Ok((uuid, sub))
}

// ---------------- TLS / QUIC 客户端配置 ----------------

/// 校验模式: 使用系统根证书
fn make_verify_client_config() -> ClientConfig {
    let mut root_store = rustls::RootCertStore::empty();
    root_store.add_trust_anchors(webpki_roots::TLS_SERVER_ROOTS.iter().map(|ta| {
        rustls::OwnedTrustAnchor::from_subject_spki_name_constraints(
            ta.subject.as_ref().to_vec(),
            ta.subject_public_key_info.as_ref().to_vec(),
            ta.name_constraints.as_ref().map(|nc| nc.as_ref().to_vec()),
        )
    }));
    let crypto = rustls::ClientConfig::builder()
        .with_safe_defaults()
        .with_root_certificates(root_store)
        .with_no_client_auth();
    let mut config = ClientConfig::new(Arc::new(crypto));
    let mut transport = quinn::TransportConfig::default();
    transport.max_idle_timeout(Some(Duration::from_secs(60).try_into().expect("超时值合法")));
    transport.max_concurrent_uni_streams(32_u8.into());
    config.transport_config(Arc::new(transport));
    config
}

// ---------------- 帧解析 ----------------

fn head_len() -> usize {
    let head = HeadMsg { version: 1, crc: 0, body_len: 0, message_type: MSG_TYPE_TEXT };
    bincode::serialize(&head).expect("序列化 HeadMsg 失败").len()
}

fn parse_frames(buf: &[u8], hl: usize) -> Vec<(HeadMsg, TextQuicMsg)> {
    let mut out = Vec::new();
    let mut i = 0usize;
    while i + hl <= buf.len() {
        let head: HeadMsg = match bincode::deserialize(&buf[i..i + hl]) {
            Ok(h) => h,
            Err(_) => break,
        };
        let start = i + hl;
        let end = start + head.body_len as usize;
        if end > buf.len() {
            break; // 半包(服务端每条 uni 流通常只含完整帧,这里直接丢弃)
        }
        let body_slice = &buf[start..end];
        if X25.checksum(body_slice) != head.crc {
            break;
        }
        match bincode::deserialize::<TextQuicMsg>(body_slice) {
            Ok(body) => out.push((head, body)),
            Err(_) => break,
        }
        i = end;
    }
    out
}

fn type_label(t: u16) -> String {
    let name = match t {
        MSG_TYPE_TEXT => "文本",
        MSG_TYPE_IMAGE => "图片",
        MSG_TYPE_FILE => "文件",
        MSG_TYPE_P2P => "P2P转发",
        MSG_TYPE_P2P_VIDEO_CALL => "P2P视频呼叫",
        MSG_TYPE_P2P_VIDEO_DATA => "P2P视频数据",
        MSG_TYPE_P2P_VIDEO_CONFIG => "P2P视频配置",
        MSG_TYPE_PING => "PING/PONG",
        MSG_TYPE_RECALL_SUCCESS => "回执成功(ACK)",
        MSG_TYPE_RECALL_FAILURE => "回执失败",
        MSG_TYPE_SYSTEM => "系统消息",
        MSG_TYPE_FORCE_LOGOUT => "强制下线",
        NOTIFY_TYPE_MSG => "通知",
        MSG_TYPE_GROUP_TEXT => "群文本",
        MSG_TYPE_GROUP_IMAGE => "群图片",
        MSG_TYPE_GROUP_FILE => "群文件",
        MSG_TYPE_GROUP_NOTIFICATION => "群通知",
        MSG_TYPE_GROUP_ACK => "群回执",
        MSG_TYPE_WEBRTC_SIGNAL => "WebRTC信令",
        MSG_TYPE_P2P_VIDEO_CALL_INVITE => "通话邀请",
        MSG_TYPE_P2P_VIDEO_CALL_ACCEPT => "通话接受",
        MSG_TYPE_P2P_VIDEO_CALL_REJECT => "通话拒绝",
        MSG_TYPE_P2P_VIDEO_CALL_END => "通话结束",
        _ => "其他",
    };
    format!("{}({})", name, t)
}

fn readable(raw: &[u8]) -> String {
    if raw.iter().all(|b| b.is_ascii_graphic() || *b == b' ') {
        String::from_utf8_lossy(raw).to_string()
    } else {
        format!("<{}字节二进制>", raw.len())
    }
}

fn print_msg(body: &TextQuicMsg) {
    println!(
        "[收] type={} from={} to={} id={} ts={} payload={}",
        type_label(body.text_type),
        body.send_user,
        body.recv_user,
        body.nano_id,
        body.timestamp,
        readable(&body.raw)
    );
}

// ---------------- 发送 ----------------

async fn send_frame(conn: &Connection, frame: &[u8]) -> Result<(), String> {
    let mut send = conn.open_uni().await.map_err(|e| format!("open_uni: {e}"))?;
    send.write_all(frame).await.map_err(|e| format!("write: {e}"))?;
    send.finish().await.map_err(|e| format!("finish: {e}"))?;
    Ok(())
}

async fn send_chat_text(conn: &Connection, from: &str, to: &str, text: &str) -> Result<(), String> {
    let frame = generate_text_msg(
        MSG_TYPE_TEXT,
        text.as_bytes().to_vec(),
        to.to_string(),
        from.to_string(),
    )
    .map_err(|e| format!("生成消息失败: {e}"))?;
    send_frame(conn, &frame).await
}

// ---------------- 交互事件 ----------------

enum Event {
    Line(String),
    Eof,
    Timeout,
    CtrlC,
}

// ---------------- 主流程 ----------------

#[tokio::main]
async fn main() {
    let args = parse_args();

    println!(
        "QUIC probe 启动\n  服务端: {}\n  uuid: {}\n  platform: {}\n  证书校验: {}\n  SNI: {}",
        args.addr,
        args.uuid,
        args.platform,
        if args.verify { "开" } else { "关(跳过)" },
        args.sni
    );

    let mut endpoint = quinn::Endpoint::client("0.0.0.0:0".parse().expect("绑定地址合法"))
        .expect("创建 endpoint 失败");
    if args.verify {
        endpoint.set_default_client_config(make_verify_client_config());
    } else {
        endpoint.set_default_client_config(make_internal_client_config().expect("客户端配置失败"));
    }

    let conn = match endpoint.connect(args.addr, &args.sni) {
        Ok(conn_fut) => match conn_fut.await {
            Ok(c) => c,
            Err(e) => {
                eprintln!("连接失败: {e}");
                exit(1);
            }
        },
        Err(e) => {
            eprintln!("创建连接失败: {e}");
            exit(1);
        }
    };
    println!("[连接] 成功 remote={}", conn.remote_address());

    // 双向流用于握手, 发端保持不关闭(关闭会被服务端判定离线)
    let (bidi_send, bidi_recv) = match conn.open_bi().await {
        Ok(s) => s,
        Err(e) => {
            eprintln!("open_bi 失败: {e}");
            exit(1);
        }
    };

    let hl = head_len();
    let mut first_fields = serde_json::Map::new();
    first_fields.insert("token".to_string(), serde_json::Value::String(args.token.clone()));
    first_fields.insert("uuid".to_string(), serde_json::Value::String(args.uuid.clone()));
    first_fields.insert("msg_type".to_string(), serde_json::Value::String("Text".to_string()));
    first_fields.insert(
        "text_serde_struct".to_string(),
        serde_json::Value::String("user_chat_json".to_string()),
    );
    first_fields.insert("dyn_buffer_size".to_string(), serde_json::Value::from(1024 * 100usize));
    first_fields.insert("dyn_header_size".to_string(), serde_json::Value::from(hl));
    let first_str = serde_json::to_string(&serde_json::Value::Object(first_fields))
        .expect("序列化握手消息失败");
    println!("[握手] 发送初始化: {}", first_str);
    let mut bidi_send = bidi_send;
    if let Err(e) = bidi_send.write_all(first_str.as_bytes()).await {
        eprintln!("发送初始化消息失败: {e}");
        exit(1);
    }
    if let Err(e) = bidi_send.flush().await {
        eprintln!("flush 初始化消息失败: {e}");
        exit(1);
    }

    // 保持 bidi 双向流存活: recv 空读到关闭, send 端挂起不 drop(FIN 会导致服务端判离线)
    tokio::spawn(async move {
        let bidi_send = bidi_send;
        let mut bidi_recv = bidi_recv;
        let mut buf = [0u8; 1024];
        while let Ok(Some(_)) = bidi_recv.read(&mut buf).await {}
        drop(bidi_recv);
        let _send_keep = bidi_send;
        std::future::pending::<()>().await;
    });

    let sent: Arc<AtomicU64> = Arc::new(AtomicU64::new(0));
    let received: Arc<AtomicU64> = Arc::new(AtomicU64::new(0));

    // 接收循环: uni 流 -> 整流读取 -> 逐帧解析打印
    let recv_task = {
        let conn_rx = conn.clone();
        let received = received.clone();
        tokio::spawn(async move {
            loop {
                let mut recv = match conn_rx.accept_uni().await {
                    Ok(r) => r,
                    Err(e) => {
                        eprintln!("[接收] 连接已结束: {e}");
                        break;
                    }
                };
                let mut data = Vec::new();
                let mut chunk = vec![0u8; 1024 * 8];
                loop {
                    match recv.read(&mut chunk).await {
                        Ok(Some(n)) => data.extend_from_slice(&chunk[..n]),
                        Ok(None) => break,
                        Err(e) => {
                            eprintln!("[接收] 流读取错误: {e}");
                            break;
                        }
                    }
                }
                for (_, body) in parse_frames(&data, hl) {
                    received.fetch_add(1, Ordering::Relaxed);
                    print_msg(&body);
                    if body.text_type == MSG_TYPE_FORCE_LOGOUT {
                        println!("[接收] 收到强制下线,连接即将被服务端关闭");
                    }
                }
            }
        })
    };

    // 心跳
    let heartbeat = if args.ping_every_secs > 0 {
        let conn_hb = conn.clone();
        let uuid_hb = args.uuid.clone();
        let sent_hb = sent.clone();
        let every = Duration::from_secs(args.ping_every_secs);
        Some(tokio::spawn(async move {
            let mut interval = tokio::time::interval(every);
            interval.tick().await; // 跳过首个立即 tick,与主逻辑错开
            loop {
                interval.tick().await;
                let frame = match generate_text_msg(
                    MSG_TYPE_PING,
                    b"ping".to_vec(),
                    SYSTEM.to_string(),
                    uuid_hb.clone(),
                ) {
                    Ok(f) => f,
                    Err(e) => {
                        eprintln!("生成心跳消息失败: {e}");
                        continue;
                    }
                };
                match send_frame(&conn_hb, &frame).await {
                    Ok(_) => {
                        sent_hb.fetch_add(1, Ordering::Relaxed);
                        println!("[发送] ping 心跳");
                    }
                    Err(e) => {
                        eprintln!("发送心跳失败: {e}");
                    }
                }
            }
        }))
    } else {
        None
    };

    // 连接成功后一次性发送 --to/--text
    if let (Some(to), Some(text)) = (args.send_to.clone(), args.send_text.clone()) {
        match send_chat_text(&conn, &args.uuid, &to, &text).await {
            Ok(_) => {
                sent.fetch_add(1, Ordering::Relaxed);
                println!("[发送] 单聊文本 -> {to}: {text}");
            }
            Err(e) => eprintln!("发送失败: {e}"),
        }
    }

    println!(
        "\n已连接. 可用命令: say <目标uuid> <文本> | ping | status | quit/exit\n(心跳每 {}s 自动发送{})",
        args.ping_every_secs,
        if args.ping_every_secs == 0 { "(已关闭)" } else { "" }
    );

    // 事件源
    let (tx, mut rx) = mpsc::channel::<Event>(32);
    {
        let tx = tx.clone();
        tokio::spawn(async move {
            let mut lines = tokio::io::BufReader::new(tokio::io::stdin()).lines();
            loop {
                match lines.next_line().await {
                    Ok(Some(line)) => {
                        if tx.send(Event::Line(line)).await.is_err() {
                            break;
                        }
                    }
                    Ok(None) => {
                        let _ = tx.send(Event::Eof).await;
                        break;
                    }
                    Err(_) => {
                        let _ = tx.send(Event::Eof).await;
                        break;
                    }
                }
            }
        });
    }
    if args.run_secs > 0 {
        let tx = tx.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(args.run_secs)).await;
            let _ = tx.send(Event::Timeout).await;
        });
    }
    {
        let tx = tx.clone();
        tokio::spawn(async move {
            let _ = tokio::signal::ctrl_c().await;
            let _ = tx.send(Event::CtrlC).await;
        });
    }

    let mut quit = false;
    while let Some(ev) = rx.recv().await {
        match ev {
            Event::Timeout => {
                println!("[超时] 运行 {}s 结束,退出", args.run_secs);
                quit = true;
            }
            Event::CtrlC => {
                println!("[退出] 收到 Ctrl-C");
                quit = true;
            }
            Event::Eof => {
                println!("[stdin] 输入结束,仅维持心跳直到 Ctrl-C/超时");
            }
            Event::Line(line) => {
                if !handle_command(&line, &conn, &args, &sent).await {
                    quit = true;
                }
            }
        }
        if quit {
            break;
        }
    }

    println!(
        "[退出] 关闭连接... (收={}, 发={})",
        received.load(Ordering::Relaxed),
        sent.load(Ordering::Relaxed)
    );
    conn.close(0u32.into(), b"client quit");
    if let Some(hb) = heartbeat {
        hb.abort();
    }
    recv_task.abort();
    endpoint.wait_idle().await;
    println!("[退出] 已断开");
}

async fn handle_command(line: &str, conn: &Connection, args: &Args, sent: &Arc<AtomicU64>) -> bool {
    let line = line.trim();
    if line.is_empty() {
        return true;
    }
    let mut parts = line.splitn(3, ' ');
    match parts.next() {
        Some("quit") | Some("exit") | Some("q") => {
            println!("[命令] 退出");
            return false;
        }
        Some("ping") => match generate_text_msg(
            MSG_TYPE_PING,
            b"ping".to_vec(),
            SYSTEM.to_string(),
            args.uuid.clone(),
        ) {
            Ok(frame) => match send_frame(conn, &frame).await {
                Ok(_) => {
                    sent.fetch_add(1, Ordering::Relaxed);
                    println!("[发送] ping 心跳");
                }
                Err(e) => eprintln!("发送 ping 失败: {e}"),
            },
            Err(e) => eprintln!("生成 ping 消息失败: {e}"),
        },
        Some("status") => {
            println!(
                "[状态] 已发帧={} 本地uuid={} 平台={}",
                sent.load(Ordering::Relaxed),
                args.uuid,
                args.platform
            );
        }
        Some("say") => {
            let to = match parts.next() {
                Some(t) => t,
                None => {
                    println!("用法: say <目标uuid> <文本>");
                    return true;
                }
            };
            let text = parts.next().unwrap_or("");
            if text.is_empty() {
                println!("用法: say <目标uuid> <文本>");
                return true;
            }
            match send_chat_text(conn, &args.uuid, to, text).await {
                Ok(_) => {
                    sent.fetch_add(1, Ordering::Relaxed);
                    println!("[发送] 单聊文本 -> {to}: {text}");
                }
                Err(e) => eprintln!("发送失败: {e}"),
            }
        }
        Some("help") | Some("h") => {
            println!("命令: say <目标uuid> <文本> | ping | status | quit/exit");
        }
        Some(other) => println!("未知命令: {other} (help 查看帮助)"),
        None => {}
    }
    true
}
