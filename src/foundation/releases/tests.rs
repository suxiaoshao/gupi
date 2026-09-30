use super::*;

#[test]
fn stable_versions_use_semver_precedence_and_a_fixed_download_origin() {
    let release = parse(br#"{"tag_name":"v0.10.0","draft":false,"prerelease":false,"html_url":"https://untrusted.invalid"}"#).unwrap().unwrap();
    assert!(is_newer(&release, &Version::parse("0.9.0").unwrap()));
    assert!(!is_newer(
        &release,
        &Version::parse("0.10.0+local").unwrap()
    ));
    assert!(!is_newer(&release, &Version::parse("1.0.0").unwrap()));
    assert!(is_newer(
        &release,
        &Version::parse("0.10.0-beta.1").unwrap()
    ));
    assert_eq!(
        release.url,
        "https://github.com/suxiaoshao/gupi/releases/tag/v0.10.0"
    );
    for payload in [
        r#"{"tag_name":"v9.0.0","draft":true,"prerelease":false}"#,
        r#"{"tag_name":"v9.0.0","draft":false,"prerelease":true}"#,
        r#"{"tag_name":"v9.0.0-rc.1","draft":false,"prerelease":false}"#,
    ] {
        assert!(parse(payload.as_bytes()).unwrap().is_none());
    }
    assert_eq!(
        parse(br#"{"tag_name":"nightly","draft":false,"prerelease":false}"#),
        Err(Problem::InvalidResponse)
    );
}

#[tokio::test]
async fn conditional_requests_keep_the_release_and_report_server_failures() {
    // Exercise the production request/response paths entirely in memory: no server,
    // ports, proxy, connection reuse, or timing-dependent disconnects.
    let initial = http::Response::builder()
        .status(200)
        .header(header::ETAG, "\"v2\"")
        .body(r#"{"tag_name":"v2.0.0","draft":false,"prerelease":false}"#)
        .unwrap();
    let cache = read_response(initial.into(), None).await.unwrap();
    assert_eq!(
        cache.release.as_ref().unwrap().version,
        Version::new(2, 0, 0)
    );
    let client = Client::new();
    let conditional = request(&client, LATEST, Some(&cache)).build().unwrap();
    assert_eq!(conditional.headers()[header::IF_NONE_MATCH], "\"v2\"");
    assert_eq!(
        conditional.headers()[header::ACCEPT],
        "application/vnd.github+json"
    );
    assert_eq!(conditional.headers()["X-GitHub-Api-Version"], "2022-11-28");
    assert_eq!(conditional.url().as_str(), LATEST);
    let unconditional = request(&client, LATEST, None).build().unwrap();
    assert!(!unconditional.headers().contains_key(header::IF_NONE_MATCH));
    let cached = read_response(response(304, ""), Some(cache.clone()))
        .await
        .unwrap();
    assert_eq!(cached.release, cache.release);
    assert_eq!(cached.etag, cache.etag);
    assert!(
        read_response(response(404, ""), Some(cache))
            .await
            .unwrap()
            .release
            .is_none()
    );
    for (status, expected) in [
        (304, Problem::InvalidResponse),
        (403, Problem::RateLimited),
        (429, Problem::RateLimited),
        (500, Problem::Network),
    ] {
        assert_eq!(
            read_response(response(status, ""), None).await.unwrap_err(),
            expected
        );
    }
    assert_eq!(
        read_response(response(200, "null"), None)
            .await
            .unwrap_err(),
        Problem::InvalidResponse
    );
}

#[tokio::test]
async fn response_size_limit_accepts_the_boundary_and_rejects_larger_valid_json() {
    let mut body = br#"{"tag_name":"v2.0.0","draft":false,"prerelease":false}"#.to_vec();
    body.resize(MAX_RESPONSE_BYTES, b' ');
    assert!(
        read_response(response(200, body.clone()), None)
            .await
            .is_ok()
    );
    body.push(b' ');
    assert_eq!(
        read_response(response(200, body), None).await.unwrap_err(),
        Problem::InvalidResponse
    );
}

fn response(status: u16, body: impl Into<reqwest::Body>) -> reqwest::Response {
    http::Response::builder()
        .status(status)
        .body(body)
        .unwrap()
        .into()
}
