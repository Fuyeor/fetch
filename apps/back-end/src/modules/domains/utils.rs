// src/modules/domains/utils.rs
use psl::Psl;
use sha2::{Digest, Sha256};

/// 严格规范化并校验域名（PSL 检查、Punycode 转换、236 字符长度限制、拒绝子路径）
pub fn canonicalize_and_validate_domain(input: &str) -> Result<String, &'static str> {
    let trimmed = input.trim();

    //  协议检查：拒绝纯 http:// 协议
    if trimmed.starts_with("http://") {
        return Err("Only HTTPS domains are allowed");
    }

    let without_proto = trimmed.strip_prefix("https://").unwrap_or(trimmed);

    // 只允许修剪最末尾的单个根斜杠（例如允许复制粘贴的 https://fuyeor.com/）
    let without_trailing_slash = without_proto.trim_end_matches('/');

    // 核心拦截：如果还有路径斜杠、端口、Query 或 Hash，严格报错
    if without_trailing_slash.contains('/')
        || without_trailing_slash.contains(':')
        || without_trailing_slash.contains('?')
        || without_trailing_slash.contains('#')
    {
        return Err(
            "Subdirectories, paths, ports, and query parameters are not allowed; please provide a bare domain or subdomain",
        );
    }

    let lower_host = without_trailing_slash.to_lowercase();
    if lower_host.is_empty() {
        return Err("Domain cannot be empty");
    }

    // 国际化域名 (IDN) 转换为 ASCII Punycode
    let ascii_domain = idna::domain_to_ascii(&lower_host)
        .map_err(|_| "Invalid internationalized domain name (IDN)")?;

    // 长度限制：确保加上 `_fetch-challenge.` (17字符) 后不超过 DNS 253 极限
    if ascii_domain.len() > 236 {
        return Err("Domain name exceeds maximum allowed length of 236 characters");
    }

    // 基础语法结构校验
    if !ascii_domain.contains('.') || ascii_domain.starts_with('.') || ascii_domain.ends_with('.') {
        return Err("Invalid domain syntax");
    }

    // PSL 校验：禁止提交公共后缀本体（如 com.cn, github.io）
    let domain_bytes = ascii_domain.as_bytes();
    let suffix = psl::List
        .suffix(domain_bytes)
        .ok_or("Unknown or invalid public suffix (TLD)")?;

    if domain_bytes == suffix.as_bytes() {
        return Err(
            "Cannot register a public suffix (eTLD) directly; must be an eTLD+1 or subdomain",
        );
    }

    Ok(ascii_domain)
}

/// 无状态派生 32 字符 DNS / 文件 验证 Token
pub fn derive_verification_token(user_id: &str, domain: &str, salt: &str) -> String {
    let input = format!("{}:{}:{}", user_id, domain, salt);
    let hash = Sha256::digest(input.as_bytes());
    hex::encode(&hash[0..16]) // 16 字节 = 32 字符 Hex
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_domains() {
        assert_eq!(
            canonicalize_and_validate_domain("https://fuyeor.com/").unwrap(),
            "fuyeor.com"
        );
        assert_eq!(
            canonicalize_and_validate_domain("sub.fuyeor.com").unwrap(),
            "sub.fuyeor.com"
        );
        assert_eq!(
            canonicalize_and_validate_domain("a.b.c.fuyeor.com").unwrap(),
            "a.b.c.fuyeor.com"
        );
        assert_eq!(
            canonicalize_and_validate_domain("blog.github.io").unwrap(),
            "blog.github.io"
        );
    }

    #[test]
    fn test_psl_and_security_rejections() {
        // 拒绝 http
        assert!(canonicalize_and_validate_domain("http://fuyeor.com").is_err());
        // 拒绝子路径
        assert!(canonicalize_and_validate_domain("https://fuyeor.com/thought/123").is_err());
        // 拒绝子目录
        assert!(canonicalize_and_validate_domain("fuyeor.com/blog/").is_err());
        // 拒绝端口
        assert!(canonicalize_and_validate_domain("fuyeor.com:8080").is_err());
        // 拒绝公共后缀
        assert!(canonicalize_and_validate_domain("com.cn").is_err());
        assert!(canonicalize_and_validate_domain("github.io").is_err());
        assert!(canonicalize_and_validate_domain("co.uk").is_err());
        // 拒绝顶级域名
        assert!(canonicalize_and_validate_domain("com").is_err());
    }
}
