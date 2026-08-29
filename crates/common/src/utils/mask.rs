/// 连接信息脱敏工具：在日志中隐藏客户端 IP，避免泄露连接信息。
///
/// 支持 `host:port` / 纯 host 形式：
/// - IPv4: `1.2.3.4:4433` -> `1.2.3.x:4433`（隐藏最后一个八位组，保留端口）
/// - IPv6: `[::1]:4433` -> `[xxxx:xxxx:xxxx:xxxx]:4433`（隐藏地址，保留端口）
///
/// 端口号为整数时原样保留；仅含 IP 时返回脱敏后的 IP。
pub fn mask_addr(input: &str) -> String {
    match input.rsplit_once(':') {
        Some((host, port)) if port.chars().all(|c| c.is_ascii_digit()) => {
            format!("{}:{}", mask_host(host), port)
        }
        _ => mask_host(input),
    }
}

/// 脱敏 host 部分（不含端口）
fn mask_host(host: &str) -> String {
    let inner = host.trim_start_matches('[').trim_end_matches(']');

    if inner.contains(':') {
        // IPv6 地址：整体隐藏
        "[xxxx:xxxx:xxxx:xxxx]".to_string()
    } else {
        let parts: Vec<&str> = inner.split('.').collect();
        if parts.len() == 4 {
            // IPv4：隐藏最后一个八位组
            format!("{}.{}.{}.x", parts[0], parts[1], parts[2])
        } else {
            "x.x.x.x".to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mask_ipv4_with_port() {
        assert_eq!(mask_addr("192.168.1.10:4433"), "192.168.1.x:4433");
    }

    #[test]
    fn test_mask_ipv4_bare() {
        assert_eq!(mask_addr("192.168.1.10"), "192.168.1.x");
    }

    #[test]
    fn test_mask_ipv6_with_port() {
        assert_eq!(mask_addr("[2001:db8::1]:4434"), "[xxxx:xxxx:xxxx:xxxx]:4434");
    }

    #[test]
    fn test_mask_bad_addr() {
        assert_eq!(mask_addr("not-an-address"), "x.x.x.x");
    }
}
