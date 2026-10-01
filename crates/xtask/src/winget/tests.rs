use super::*;

fn release() -> PublishedRelease {
    PublishedRelease {
        tag_name: "v0.2.0".into(),
        draft: false,
        prerelease: false,
        published_at: Some("2026-10-01T00:00:00Z".into()),
    }
}

#[test]
fn accepts_only_published_stable_tags() {
    assert_eq!(stable_version("v0.2.0").unwrap(), Version::new(0, 2, 0));
    for tag in [
        "0.2.0",
        "v0.2",
        "v0.2.0-rc.1",
        "v0.2.0+build",
        "v00.2.0",
        "../main",
    ] {
        assert!(stable_version(tag).is_err(), "{tag}");
    }
    assert!(release().verify("v0.2.0").is_ok());
    assert!(release().verify("v0.3.0").is_err());
    for value in [
        PublishedRelease {
            draft: true,
            ..release()
        },
        PublishedRelease {
            prerelease: true,
            ..release()
        },
        PublishedRelease {
            published_at: None,
            ..release()
        },
    ] {
        assert!(value.verify("v0.2.0").is_err());
    }
}

#[test]
fn waits_for_first_inclusion_and_prevents_duplicates_or_downgrades() {
    let candidate = Version::new(0, 2, 0);
    assert!(submission_blocker(&candidate, &[], &[]).unwrap().is_some());
    let entries = vec![CommunityEntry {
        name: "0.1.0".into(),
        kind: "dir".into(),
    }];
    assert!(
        submission_blocker(&candidate, &entries, &[])
            .unwrap()
            .is_none()
    );
    for version in [Version::new(0, 1, 0), Version::new(0, 0, 9)] {
        assert!(
            submission_blocker(&version, &entries, &[])
                .unwrap()
                .is_some()
        );
    }
    let proposals = vec![Proposal {
        url: "https://github.com/microsoft/winget-pkgs/pull/123".into(),
        files: vec![ChangedFile {
            path: format!("{DIRECTORY}/0.2.0/{PACKAGE}.yaml"),
        }],
    }];
    assert!(
        submission_blocker(&candidate, &entries, &proposals)
            .unwrap()
            .unwrap()
            .contains("/pull/123")
    );
    assert!(
        submission_blocker(&Version::new(0, 20, 0), &entries, &proposals)
            .unwrap()
            .is_none()
    );
    let invalid = vec![CommunityEntry {
        name: "unknown".into(),
        kind: "dir".into(),
    }];
    assert!(submission_blocker(&candidate, &invalid, &[]).is_err());
}

fn archive(path: &Path, version: &str, names: &[&str]) {
    let compressed =
        flate2::write::GzEncoder::new(File::create(path).unwrap(), flate2::Compression::default());
    let mut archive = tar::Builder::new(compressed);
    for name in names {
        let contents = format!("PackageIdentifier: {PACKAGE}\nPackageVersion: \"{version}\"\n");
        let mut header = tar::Header::new_gnu();
        header.set_size(contents.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        archive
            .append_data(
                &mut header,
                format!("winget/{DIRECTORY}/0.2.0/{PACKAGE}{name}.yaml"),
                contents.as_bytes(),
            )
            .unwrap();
    }
    archive.into_inner().unwrap().finish().unwrap();
}

#[test]
fn reads_exact_manifest_bytes_and_rejects_mismatched_or_incomplete_archives() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("manifests.tar.gz");
    let version = Version::new(0, 2, 0);
    let names = ["", ".installer", ".locale.en-US"];
    archive(&path, "0.2.0", &names);
    let result = archived_manifests(&path, &version).unwrap();
    assert_eq!(result.len(), 3);
    assert_eq!(
        result["suxiaoshao.Gupi.yaml"],
        "PackageIdentifier: suxiaoshao.Gupi\nPackageVersion: \"0.2.0\"\n"
    );
    archive(&path, "0.3.0", &names);
    assert!(archived_manifests(&path, &version).is_err());
    archive(&path, "0.2.0", &["", ".installer"]);
    assert!(archived_manifests(&path, &version).is_err());
    archive(&path, "0.2.0", &["", ".installer", ".locale.en-US", ""]);
    assert!(archived_manifests(&path, &version).is_err());
}
