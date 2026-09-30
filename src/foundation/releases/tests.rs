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
    use std::io::{Read, Write};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let endpoint = format!("http://{}/latest", listener.local_addr().unwrap());
    let server = std::thread::spawn(move || {
        for (ix, response) in [
            "HTTP/1.1 200 OK\r\nETag: \"v2\"\r\nContent-Length: 54\r\n\r\n{\"tag_name\":\"v2.0.0\",\"draft\":false,\"prerelease\":false}",
            "HTTP/1.1 304 Not Modified\r\n\r\n",
            "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n",
            "HTTP/1.1 429 Too Many Requests\r\nContent-Length: 0\r\n\r\n",
            "HTTP/1.1 500 Server Error\r\nContent-Length: 0\r\n\r\n",
            "HTTP/1.1 200 OK\r\nContent-Length: 1048577\r\n\r\n",
            "HTTP/1.1 200 OK\r\nContent-Length: 4\r\n\r\nnull",
        ].into_iter().enumerate() {
            let (mut stream, _) = listener.accept().unwrap();
            stream.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
            let mut request = Vec::new();
            let mut byte = [0];
            while !request.ends_with(b"\r\n\r\n") {
                stream.read_exact(&mut byte).unwrap();
                request.push(byte[0]);
            }
            if ix == 1 {
                assert!(String::from_utf8(request).unwrap().to_lowercase().contains("if-none-match: \"v2\""));
            }
            stream.write_all(response.as_bytes()).unwrap();
        }
    });
    let client = Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(5))
        .build()
        .unwrap();
    let cache = fetch(&client, &endpoint, None).await.unwrap();
    assert_eq!(
        cache.release.as_ref().unwrap().version,
        Version::new(2, 0, 0)
    );
    let cached = fetch(&client, &endpoint, Some(cache.clone()))
        .await
        .unwrap();
    assert_eq!(cached.release, cache.release);
    assert!(
        fetch(&client, &endpoint, Some(cache))
            .await
            .unwrap()
            .release
            .is_none()
    );
    for expected in [
        Problem::RateLimited,
        Problem::Network,
        Problem::InvalidResponse,
        Problem::InvalidResponse,
    ] {
        assert_eq!(fetch(&client, &endpoint, None).await.unwrap_err(), expected);
    }
    server.join().unwrap();
}
