use std::sync::Arc;

use common::read_global_config;
use common::state::CoreState;
use tracing::{error, info};

use crate::external::chat_node::ChatNode;
use crate::external::config::ChatNodeConfig;
use crate::external::lifecycle::ServiceLifecycle;
use crate::internal::internal_config::InternalQuicConfig;
use crate::internal::internal_quic_server::run_internal_server;
use crate::nat_ip::nat_udp_service::run_udp_server;

/// 启动 QUIC 服务（ChatNode + NAT UDP + 内部 QUIC），完全自包含
pub async fn start_server() -> anyhow::Result<Arc<ChatNode>> {
    let resolved_content = common::init_app_config()?;

    let redis_url = read_global_config!("redis", "url");
    let redis = match common::init_redis(&redis_url) {
        Ok(pool) => {
            info!("Redis 连接池就绪");
            pool
        }
        Err(e) => return Err(anyhow::anyhow!("Redis 初始化失败: {}", e)),
    };

    let db_url = read_global_config!("database", "url");
    let db = match common::init_sql_pool(&db_url).await {
        Ok(db) => {
            info!("数据库连接池就绪");
            db
        }
        Err(e) => return Err(anyhow::anyhow!("数据库初始化失败: {}", e)),
    };

    let core = CoreState { db, redis };

    let config = ChatNodeConfig::from_toml_str(&resolved_content)?;
    let internal_config = InternalQuicConfig::from_toml_str(&resolved_content)?;
    let mut node = ChatNode::new(config, core.clone());
    node.init().await?;
    let node = Arc::new(node);
    node.start().await?;

    let connections = node.connections();
    let server_index = node.config().server_index;

    // 集群：注册外部节点 + 启动 server_count 后台同步
    {
        let node_address = node.config().node_address.clone();
        if let Err(e) = common::utils::server_count_sync::register_external_node(
            &core.redis,
            server_index,
            &node_address,
        )
        .await
        {
            tracing::warn!("外部 QUIC 节点注册失败: {}", e);
        }
        common::utils::server_count_sync::start_server_count_sync(core.redis.clone());
        info!("server_count 后台同步已启动 (server_index={})", server_index);

        // 节点 key 保活与看门狗放到独立线程运行,主 runtime 卡死也不影响续期
        common::utils::server_count_sync::start_node_key_keeper(
            read_global_config!("redis", "url"),
            server_index,
            node.config().node_address.clone(),
            internal_config.node_address.clone(),
        );
        common::utils::server_count_sync::start_node_key_watchdog();
    }

    // 启动 NAT 发现 + 客户端 P2P 请求转发 UDP 服务
    run_udp_server(core.clone(), connections.clone()).await?;

    // 启动内部 QUIC 服务
    let (internal_shutdown_tx, internal_shutdown_rx) = tokio::sync::watch::channel(false);
    tokio::spawn(async move {
        let _tx = internal_shutdown_tx;
        run_internal_server(core, internal_config, connections, internal_shutdown_rx).await;
    });

    Ok(node)
}

/// 在独立的多线程 tokio runtime 上启动并运行 QUIC 服务(后台常驻线程)。
///
/// 背景: 主入口若跑在 actix 单线程 runtime 上,QUIC 的 accept 循环、各连接处理、
/// 后台任务全部挤在一个 worker 里;一旦某任务出现同步阻塞/永不返回,整个 QUIC 侧
/// (含 accept 循环)都会被冻结,表现为"新连接/新流无法接入"。这里给 QUIC 独立
/// 多 worker runtime,单条阻塞最多卡住一个 worker,不再拖死整机接收。
///
/// 返回前会阻塞等待 QUIC 初始化完成(与 HTTP 启动保持先后关系一致);
/// 初始化失败时返回 Err,调用方按启动失败处理即可。
pub fn start_server_in_dedicated_runtime() -> anyhow::Result<()> {
    let (ready_tx, ready_rx) = std::sync::mpsc::channel::<Result<(), String>>();
    let builder = std::thread::Builder::new().name("quic-main-runtime".to_string());
    let thread = builder.spawn(move || {
        let workers =
            std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4).clamp(2, 8);
        let runtime = match tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .worker_threads(workers)
            .thread_name("quic-worker")
            .build()
        {
            Ok(rt) => rt,
            Err(e) => {
                error!("创建 QUIC 独立 runtime 失败: {}", e);
                let _ = ready_tx.send(Err(format!("创建 QUIC 独立 runtime 失败: {}", e)));
                return;
            }
        };
        info!("QUIC 独立 runtime 已创建: worker_threads={}", workers);

        let result = runtime.block_on(start_server());
        // 持有 ChatNode: 其内部保存的关闭 watch::Sender 一旦被 drop,accept 循环与
        // TLS 监控会把"发送方消失"误判为收到关闭信号而立即退出,导致外部 QUIC 无法接入。
        let node = match result {
            Ok(node) => {
                info!("QUIC 服务已在独立 runtime 上启动完成");
                let _ = ready_tx.send(Ok(()));
                node
            }
            Err(e) => {
                error!("QUIC 服务启动失败: {:?}", e);
                let _ = ready_tx.send(Err(format!("QUIC 服务启动失败: {:?}", e)));
                return;
            }
        };

        // start_server 返回后,各服务任务(accept 循环/连接处理等)仍运行在 runtime 上;
        // 这里让线程常驻(同时持有 node),防止 runtime 或 node 随本线程作用域结束被 drop
        // 导致任务全部取消、关闭通道发送方消失。
        loop {
            std::thread::sleep(std::time::Duration::from_secs(3600));
            let _ = &node;
        }
    });

    let _thread = thread.map_err(|e| anyhow::anyhow!("启动 QUIC 独立 runtime 线程失败: {}", e))?;
    match ready_rx.recv() {
        Ok(Ok(())) => Ok(()),
        Ok(Err(msg)) => Err(anyhow::anyhow!("{}", msg)),
        Err(_) => Err(anyhow::anyhow!("QUIC 独立 runtime 线程异常退出")),
    }
}
