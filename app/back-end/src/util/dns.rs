// src/util/dns.rs
use hickory_resolver::TokioAsyncResolver;

/// 查询指定主机名的所有 TXT 记录列表
pub async fn lookup_txt(resolver: &TokioAsyncResolver, host: &str) -> Vec<String> {
    // 规范化 FQDN：确保以 . 结尾防止本地搜索域干扰
    let fqdn = if host.ends_with('.') {
        host.to_string()
    } else {
        format!("{}.", host)
    };

    let mut records = Vec::new();
    if let Ok(lookup) = resolver.txt_lookup(&fqdn).await {
        for txt in lookup.iter() {
            for data in txt.txt_data() {
                records.push(String::from_utf8_lossy(data).to_string());
            }
        }
    }
    records
}

/// 检查目标主机名是否包含匹配的 TXT 记录值
pub async fn check_txt_contains(
    resolver: &TokioAsyncResolver,
    host: &str,
    expected_content: &str,
) -> bool {
    let records = lookup_txt(resolver, host).await;
    records.iter().any(|r| r.contains(expected_content))
}
