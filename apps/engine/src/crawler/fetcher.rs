// apps/engine/src/crawler/fetcher.rs

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::time::Duration;

use reqwest::header::{ETAG, IF_MODIFIED_SINCE, IF_NONE_MATCH, LAST_MODIFIED, LOCATION};
use reqwest::{Client, StatusCode, Url};

use super::CrawlerResult;
use super::state::ResourceRecord;

/// Bounds applied to every remote request made from the submitted locator set.
#[derive(Debug, Clone)]
pub struct FetchPolicy {
    pub max_response_bytes: usize,
    pub connect_timeout: Duration,
    pub request_timeout: Duration,
    pub max_redirects: usize,
    pub allow_private_networks: bool,
}

impl Default for FetchPolicy {
    fn default() -> Self {
        Self {
            max_response_bytes: 4 * 1024 * 1024,
            connect_timeout: Duration::from_secs(5),
            request_timeout: Duration::from_secs(30),
            max_redirects: 3,
            allow_private_networks: false,
        }
    }
}

impl FetchPolicy {
    /// Enable loopback access only for deterministic local integration tests.
    #[cfg(test)]
    pub fn for_tests() -> Self {
        Self {
            allow_private_networks: true,
            ..Self::default()
        }
    }
}

/// A response body and its cache validators after origin checks and size limits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FetchedResource {
    pub url: Url,
    pub status: u16,
    pub etag: Option<String>,
    pub last_modified: Option<String>,
    pub body: Vec<u8>,
}

/// A bounded HTTP client for webmaster-submitted FON locators.
#[derive(Clone)]
pub struct RemoteFetcher {
    client: Client,
    policy: FetchPolicy,
}

impl RemoteFetcher {
    /// Build a client with manual redirect handling so every hop is revalidated.
    pub fn new(policy: FetchPolicy) -> CrawlerResult<Self> {
        let client = Client::builder()
            .connect_timeout(policy.connect_timeout)
            .timeout(policy.request_timeout)
            .redirect(reqwest::redirect::Policy::none())
            .build()?;
        Ok(Self { client, policy })
    }

    /// Fetch one explicitly submitted locator with optional conditional validators.
    pub async fn fetch(
        &self,
        raw_url: &str,
        previous: Option<&ResourceRecord>,
    ) -> CrawlerResult<FetchedResource> {
        let initial = Url::parse(raw_url)?;
        let mut current = initial.clone();
        for redirect_count in 0..=self.policy.max_redirects {
            self.validate_target(&current).await?;
            let mut request = self.client.get(current.clone());
            if redirect_count == 0 {
                if let Some(previous) = previous {
                    if let Some(etag) = previous.etag.as_deref() {
                        request = request.header(IF_NONE_MATCH, etag);
                    }
                    if let Some(last_modified) = previous.last_modified.as_deref() {
                        request = request.header(IF_MODIFIED_SINCE, last_modified);
                    }
                }
            }
            let mut response = request.send().await?;
            if response.status().is_redirection() {
                if redirect_count == self.policy.max_redirects {
                    return Err("remote locator redirect limit exceeded".into());
                }
                let location = response
                    .headers()
                    .get(LOCATION)
                    .ok_or_else(|| "remote locator redirect has no location".to_string())?
                    .to_str()?;
                let next = current.join(location)?;
                if !same_origin(&initial, &next) {
                    return Err("cross-origin redirect rejected".into());
                }
                current = next;
                continue;
            }
            if response.status() == StatusCode::NOT_MODIFIED {
                return Ok(FetchedResource {
                    url: current,
                    status: response.status().as_u16(),
                    etag: header_string(&response, ETAG),
                    last_modified: header_string(&response, LAST_MODIFIED),
                    body: Vec::new(),
                });
            }
            if !response.status().is_success() {
                return Err(format!("remote locator returned status {}", response.status()).into());
            }
            if response
                .content_length()
                .is_some_and(|length| length > self.policy.max_response_bytes as u64)
            {
                return Err("remote locator response exceeds byte limit".into());
            }
            let status = response.status().as_u16();
            let etag = header_string(&response, ETAG);
            let last_modified = header_string(&response, LAST_MODIFIED);
            let mut body = Vec::new();
            while let Some(chunk) = response.chunk().await? {
                if body.len().saturating_add(chunk.len()) > self.policy.max_response_bytes {
                    return Err("remote locator response exceeds byte limit".into());
                }
                body.extend_from_slice(&chunk);
            }
            return Ok(FetchedResource {
                url: current,
                status,
                etag,
                last_modified,
                body,
            });
        }
        Err("remote locator redirect loop exhausted".into())
    }

    async fn validate_target(&self, url: &Url) -> CrawlerResult<()> {
        if !matches!(url.scheme(), "http" | "https") {
            return Err("remote locator must use http or https".into());
        }
        if !url.username().is_empty() || url.password().is_some() {
            return Err("remote locator must not contain credentials".into());
        }
        let host = url
            .host_str()
            .ok_or_else(|| "remote locator must contain a host".to_string())?;
        if self.policy.allow_private_networks {
            return Ok(());
        }
        let port = url
            .port_or_known_default()
            .ok_or_else(|| "remote locator must use a known HTTP port".to_string())?;
        let addresses = tokio::net::lookup_host((host, port)).await?;
        let mut found = false;
        for address in addresses {
            found = true;
            if !is_public_ip(address.ip()) {
                return Err("remote locator resolves to a private or reserved address".into());
            }
        }
        if !found {
            return Err("remote locator host did not resolve".into());
        }
        Ok(())
    }
}

fn header_string(
    response: &reqwest::Response,
    name: reqwest::header::HeaderName,
) -> Option<String> {
    response
        .headers()
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(ToOwned::to_owned)
}

fn same_origin(left: &Url, right: &Url) -> bool {
    left.scheme() == right.scheme()
        && left.host() == right.host()
        && left.port_or_known_default() == right.port_or_known_default()
}

fn is_public_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => is_public_ipv4(ip),
        IpAddr::V6(ip) => is_public_ipv6(ip),
    }
}

fn is_public_ipv4(ip: Ipv4Addr) -> bool {
    let octets = ip.octets();
    !ip.is_private()
        && !ip.is_loopback()
        && !ip.is_link_local()
        && !ip.is_unspecified()
        && !ip.is_broadcast()
        && !matches!(octets[0], 0 | 10 | 127 | 224..=255)
        && !(octets[0] == 100 && (64..=127).contains(&octets[1]))
        && !(octets[0] == 169 && octets[1] == 254)
        && !(octets[0] == 192 && octets[1] == 0 && octets[2] == 0)
        && !(octets[0] == 192 && octets[1] == 0 && octets[2] == 2)
        && !(octets[0] == 192 && octets[1] == 88 && octets[2] == 99)
        && !(octets[0] == 198 && (18..=19).contains(&octets[1]))
        && !(octets[0] == 198 && octets[1] == 51 && octets[2] == 100)
        && !(octets[0] == 203 && octets[1] == 0 && octets[2] == 113)
}

fn is_public_ipv6(ip: Ipv6Addr) -> bool {
    let segments = ip.segments();
    !ip.is_loopback()
        && !ip.is_unspecified()
        && !ip.is_unique_local()
        && !ip.is_unicast_link_local()
        && !(segments[0] & 0xff00 == 0xff00)
        && !(segments[0] & 0xffc0 == 0xfe80)
        && !(segments[0] == 0x2001 && segments[1] == 0x0db8)
}

#[cfg(test)]
mod tests {
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

    use reqwest::Url;

    use super::{is_public_ip, same_origin};

    #[test]
    fn rejects_private_and_reserved_addresses() {
        assert!(!is_public_ip(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1))));
        assert!(!is_public_ip(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1))));
        assert!(!is_public_ip(IpAddr::V6(Ipv6Addr::LOCALHOST)));
        assert!(is_public_ip(IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1))));
    }

    #[test]
    fn redirect_origin_comparison_includes_scheme_host_and_port() {
        let origin = Url::parse("https://example.com:443/a").unwrap();
        assert!(same_origin(
            &origin,
            &Url::parse("https://example.com/b").unwrap()
        ));
        assert!(!same_origin(
            &origin,
            &Url::parse("http://example.com/a").unwrap()
        ));
        assert!(!same_origin(
            &origin,
            &Url::parse("https://other.example/a").unwrap()
        ));
    }
}
