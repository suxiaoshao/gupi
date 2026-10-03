use reqwest::Client;
use reqwest::StatusCode;
use reqwest::header;
use semver::Version;
use serde::Deserialize;
use std::time::Duration;

const LATEST: &str = "https://api.github.com/repos/suxiaoshao/gupi/releases/latest";
const MAX_RESPONSE_BYTES: usize = 1024 * 1024;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Release {
    pub version: Version,
    pub url: String,
}

#[derive(Clone, Debug, Default)]
pub struct Cache {
    etag: Option<header::HeaderValue>,
    pub release: Option<Release>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum Problem {
    #[error("release request failed")]
    Network,
    #[error("GitHub release request was rate limited")]
    RateLimited,
    #[error("invalid release response")]
    InvalidResponse,
}

impl Problem {
    pub fn key(self) -> &'static str {
        match self {
            Self::Network => "updates-error-network",
            Self::RateLimited => "updates-error-rate-limit",
            Self::InvalidResponse => "updates-error-response",
        }
    }
}

#[derive(Deserialize)]
struct PublishedRelease {
    tag_name: String,
    draft: bool,
    prerelease: bool,
}

fn parse(bytes: &[u8]) -> Result<Option<Release>, Problem> {
    let release: PublishedRelease =
        serde_json::from_slice(bytes).map_err(|_| Problem::InvalidResponse)?;
    if release.draft || release.prerelease {
        return Ok(None);
    }
    let version = Version::parse(
        release
            .tag_name
            .strip_prefix('v')
            .unwrap_or(&release.tag_name),
    )
    .map_err(|_| Problem::InvalidResponse)?;
    if !version.pre.is_empty() {
        return Ok(None);
    }
    // Construct the destination from our fixed repository, never a remote URL field.
    let mut url = url::Url::parse("https://github.com/suxiaoshao/gupi/releases/tag/").unwrap();
    url.path_segments_mut()
        .unwrap()
        .pop_if_empty()
        .push(&release.tag_name);
    Ok(Some(Release {
        version,
        url: url.into(),
    }))
}

pub fn is_newer(release: &Release, current: &Version) -> bool {
    // Build metadata does not contribute to SemVer precedence.
    release.version.cmp_precedence(current).is_gt()
}

pub async fn latest(cache: Option<Cache>) -> Result<Cache, Problem> {
    let client = Client::builder()
        .user_agent(concat!("Gupi/", env!("CARGO_PKG_VERSION")))
        .connect_timeout(Duration::from_secs(5))
        .timeout(Duration::from_secs(15))
        .https_only(true)
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|_| Problem::Network)?;
    fetch(&client, LATEST, cache).await
}

async fn fetch(client: &Client, endpoint: &str, cache: Option<Cache>) -> Result<Cache, Problem> {
    let response = request(client, endpoint, cache.as_ref())
        .send()
        .await
        .map_err(|_| Problem::Network)?;
    read_response(response, cache).await
}

fn request(client: &Client, endpoint: &str, cache: Option<&Cache>) -> reqwest::RequestBuilder {
    let mut request = client
        .get(endpoint)
        .header(header::ACCEPT, "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28");
    if let Some(etag) = cache.and_then(|cache| cache.etag.as_ref()) {
        request = request.header(header::IF_NONE_MATCH, etag.clone());
    }
    request
}

async fn read_response(
    mut response: reqwest::Response,
    cache: Option<Cache>,
) -> Result<Cache, Problem> {
    match response.status() {
        StatusCode::NOT_MODIFIED => return cache.ok_or(Problem::InvalidResponse),
        // The fixed public repository has no published Latest release yet.
        StatusCode::NOT_FOUND => return Ok(Cache::default()),
        StatusCode::FORBIDDEN | StatusCode::TOO_MANY_REQUESTS => return Err(Problem::RateLimited),
        StatusCode::OK => {}
        _ => return Err(Problem::Network),
    }
    if response
        .content_length()
        .is_some_and(|length| length > MAX_RESPONSE_BYTES as u64)
    {
        return Err(Problem::InvalidResponse);
    }
    let etag = response.headers().get(header::ETAG).cloned();
    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|_| Problem::Network)? {
        if body.len() + chunk.len() > MAX_RESPONSE_BYTES {
            return Err(Problem::InvalidResponse);
        }
        body.extend_from_slice(&chunk);
    }
    Ok(Cache {
        etag,
        release: parse(&body)?,
    })
}

#[cfg(test)]
mod tests;
