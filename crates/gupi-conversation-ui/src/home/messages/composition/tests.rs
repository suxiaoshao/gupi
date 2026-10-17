use super::*;
use gpui_kit::component::diff::DiffFile;

fn patch(body: &str) -> String {
    format!("--- file.txt\n+++ file.txt\n{body}")
}
fn net(patches: &[&str]) -> Option<String> {
    let result =
        compose("file.txt", "/project", patches).unwrap_or_else(|_| panic!("patches: {patches:?}"));
    if let Some(source) = &result {
        let parsed = DiffFile::parse(source).unwrap();
        assert_eq!(parsed.len(), 1);
    }
    result
}
fn replacement(old: &str, new: &str) -> String {
    let old_lines = text_lines(old);
    let new_lines = text_lines(new);
    let mut result = format!(
        "--- file.txt\n+++ file.txt\n@@ -{},{} +{},{} @@\n",
        usize::from(!old_lines.is_empty()),
        old_lines.len(),
        usize::from(!new_lines.is_empty()),
        new_lines.len()
    );
    for (kind, lines) in [('-', old_lines), ('+', new_lines)] {
        for line in lines {
            result.push(kind);
            result.push_str(&line.text);
            result.push('\n');
            if !line.newline {
                result.push_str("\\ No newline at end of file\n");
            }
        }
    }
    result
}
#[test]
fn overlap_shifts_and_complete_cancellation() {
    let first = patch("@@ -2,2 +2,3 @@\n same\n-old\n+one\n+two\n");
    let second = patch("@@ -3,2 +3,1 @@\n-one\n-two\n+old\n");
    assert_eq!(net(&[&first, &second]), None);
    let insert = patch("@@ -1000000000,0 +1000000001,2 @@\n+a\n+b\n");
    let remove = patch("@@ -1000000001,2 +1000000000,0 @@\n-a\n-b\n");
    assert_eq!(net(&[&insert, &remove]), None);
}
#[test]
fn shifted_disjoint_hunks_and_inserted_lines_are_composed() {
    let first = patch("@@ -1,1 +1,2 @@\n a\n+x\n@@ -5,1 +6,1 @@\n-e\n+E\n");
    let second = patch("@@ -2,1 +2,2 @@\n-x\n+y\n+z\n@@ -6,1 +7,1 @@\n-E\n+end\n");
    let combined = net(&[&first, &second]).unwrap();
    assert_eq!(
        apply("file.txt", "", "a\nb\nc\nd\ne\nf\n", &[&combined]).unwrap(),
        "a\ny\nz\nb\nc\nd\nend\nf\n"
    );
    assert!(!combined.contains("\n b\n")); // unknown gap was not fabricated
}
#[test]
fn strict_overlap_and_repeated_lines_never_search_for_matches() {
    let first = patch("@@ -1,2 +1,2 @@\n-a\n+A\n same\n");
    let invalid = patch("@@ -2,1 +2,1 @@\n-other\n+new\n");
    assert!(compose("file.txt", "", &[&first, &invalid]).is_err());
    let wrong_position = patch("@@ -2,1 +2,1 @@\n-a\n+b\n");
    assert!(apply("file.txt", "", "a\nx\na\n", &[&wrong_position]).is_err());
}
#[test]
fn insertions_at_same_boundary_and_file_edges() {
    let one = patch("@@ -0,0 +1,1 @@\n+a\n");
    let two = patch("@@ -0,0 +1,1 @@\n+b\n");
    let three = patch("@@ -2,0 +3,1 @@\n+c\n");
    let combined = net(&[&one, &two, &three]).unwrap();
    assert_eq!(
        apply("file.txt", "", "", &[&combined]).unwrap(),
        "b\na\nc\n"
    );
    let clear = replacement("b\na\nc\n", "");
    assert_eq!(net(&[&one, &two, &three, &clear]), None);
}
#[test]
fn independent_old_and_new_eof_markers() {
    for (old, new) in [("a", "b\n"), ("a\n", "b"), ("a", "b"), ("", "b"), ("a", "")] {
        let forward = replacement(old, new);
        let combined = net(&[&forward]).unwrap();
        assert_eq!(apply("file.txt", "", old, &[&combined]).unwrap(), new);
        let backward = replacement(new, old);
        assert_eq!(net(&[&forward, &backward]), None);
    }
    let no_end = replacement("a\n", "a");
    let illegal_insert = patch("@@ -1,0 +2,1 @@\n+b\n");
    assert!(compose("file.txt", "", &[&no_end, &illegal_insert]).is_err());
    assert!(apply("file.txt", "", "a\nb\n", &[&replacement("a", "b")]).is_err());
}
#[test]
fn pi_write_protocol_preserves_bom_and_restores_each_edits_line_style() {
    let first = replacement("a\nb\nc\n", "A\nb\nc\n");
    let second = replacement("A\nb\nc\n", "A\nB\nc\n");
    assert_eq!(
        apply("file.txt", "", "\u{feff}a\r\nb\rc\n", &[&first, &second]).unwrap(),
        "\u{feff}A\r\nB\r\nc\r\n"
    );
    assert_eq!(
        apply("file.txt", "", "a\nb\r\nc\r", &[&first]).unwrap(),
        "A\nb\nc\n"
    );
    assert_eq!(
        apply("file.txt", "", "a\rb\rc\r", &[&first]).unwrap(),
        "A\nb\nc\n"
    );
    assert_eq!(
        apply("file.txt", "", "\u{feff}a\r\nb\r", &[]).unwrap(),
        "\u{feff}a\r\nb\r"
    );
    let drop_lf = replacement("a\n", "b");
    let add_lf = replacement("b", "c\nd\n");
    assert_eq!(
        apply("file.txt", "", "a\r\n", &[&drop_lf, &add_lf]).unwrap(),
        "c\nd\n"
    );
}
#[test]
fn source_cr_is_distinct_from_transport_crlf() {
    let source = patch("@@ -1,1 +1,1 @@\n-a\r\n+b\r\n");
    let result = net(&[&source]).unwrap();
    assert!(result.contains("-a\r\n+b\r\n"));
    let transport = source.replace('\n', "\r\n");
    assert_eq!(net(&[&source]), net(&[&transport]));
    let pi = replacement("a\n", "b\n").replace('\n', "\r\n");
    assert_eq!(apply("file.txt", "", "a\r\n", &[&pi]).unwrap(), "b\r\n");
}
#[test]
fn exact_text_keeps_unicode_spaces_quotes_and_bom_in_patch_content() {
    let old = "‘你好’  \n\u{feff}second\n";
    let new = "\"你好\" \n\u{feff}second\n";
    let source = replacement(old, new);
    let result = net(&[&source]).unwrap();
    assert_eq!(apply("file.txt", "", old, &[&result]).unwrap(), new);
    assert!(apply("file.txt", "", "‘你好’ \n\u{feff}second\n", &[&source]).is_err());
}
#[test]
fn lexical_paths_and_c_quoted_headers() {
    for (path, cwd, old, new) in [
        ("file.txt", "/project", "/project/file.txt", "./file.txt"),
        ("space name", "/project", "space name", "space name"),
        ("file.txt", "C:\\project", "C:/project/file.txt", "file.txt"),
        (
            "file.txt",
            "\\\\server\\share",
            "//server/share/file.txt",
            "file.txt",
        ),
        (
            "tab\tquote\"\\名字",
            "/project",
            "\"tab\\tquote\\\"\\\\\\345\\220\\215\\345\\255\\227\"",
            "\"tab\\tquote\\\"\\\\\\345\\220\\215\\345\\255\\227\"",
        ),
    ] {
        let source = format!("--- {old}\n+++ {new}\n@@ -1 +1 @@\n-a\n+b\n");
        let result = compose(path, cwd, &[&source]).unwrap().unwrap();
        assert_eq!(DiffFile::parse(&result).unwrap().len(), 1);
        assert_eq!(apply(path, cwd, "a\n", &[&result]).unwrap(), "b\n");
    }
}
#[test]
fn malformed_unsupported_and_ambiguous_inputs_fall_back() {
    let valid = patch("@@ -1 +1 @@\n-a\n+b\n");
    let cases = [
        patch("@@ -0,1 +1,1 @@\n-a\n+b\n"),
        patch("@@ -1,2 +1,1 @@\n-a\n+b\n"),
        patch("@@ -1 +2 @@\n-a\n+b\n"),
        patch("@@ -1 +1 @@\n-a\n+b\n@@ -1 +1 @@\n-b\n+c\n"),
        patch("@@ -1 +1 @@\n-a\n+b\n\\ No newline at end of file\n\\ No newline at end of file\n"),
        patch("@@ -999999999999999999999999999999 +1 @@\n-a\n+b\n"),
        valid.replace("+++ file.txt", "+++ other.txt"),
        format!("{valid}{valid}"),
        valid.trim_end_matches('\n').to_owned(),
        format!("diff --git a/other.txt b/other.txt\n{valid}"),
        format!("diff --git a/file.txt b/file.txt\nold mode 100644\nnew mode 100755\n{valid}"),
        "GIT binary patch\n".into(),
        patch("@@ -1,1 +1,1 @@\n<<<<<<< ours\n=======\n>>>>>>> theirs\n"),
    ];
    for source in cases {
        assert!(
            compose("file.txt", "", &[&source]).is_err(),
            "accepted {source:?}"
        );
    }
}
#[test]
fn canonical_output_survives_repeated_lines_and_reverted_intermediates() {
    let base = "same\nsame\na\nsame\n";
    let final_text = "same\nnew\nsame\n";
    let direct = replacement(base, final_text);
    let middle = replacement(base, "x\nsame\na\nsame\n");
    let finish = replacement("x\nsame\na\nsame\n", final_text);
    assert_eq!(net(&[&direct]), net(&[&middle, &finish]));
    let temporary = replacement(final_text, "temporary\n");
    let revert = replacement("temporary\n", final_text);
    assert_eq!(net(&[&direct]), net(&[&direct, &temporary, &revert]));
}
#[test]
fn deterministic_reference_sequences_cover_hunk_shifts_and_cancellation() {
    // Independent reference is a Vec splice. Each generated patch replaces just
    // that exact range; composition must equal the fully known reference text.
    let mut seed = 0x173912u64;
    for round in 0..80 {
        let original: Vec<String> = (0..20).map(|ix| format!("line {}\n", ix % 4)).collect();
        let mut current = original.clone();
        let mut patches = Vec::new();
        for step in 0..15 {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            let start = (seed as usize >> 8) % (current.len() + 1);
            let removed = ((seed as usize >> 16) % 4).min(current.len() - start);
            let added = (seed as usize >> 24) % 4;
            if removed + added == 0 {
                continue;
            }
            let inserted: Vec<_> = (0..added)
                .map(|ix| format!("value {}\n", (round + step + ix) % 5))
                .collect();
            let mut source = format!(
                "--- file.txt\n+++ file.txt\n@@ -{},{} +{},{} @@\n",
                start + usize::from(removed > 0),
                removed,
                start + usize::from(added > 0),
                added
            );
            for line in &current[start..start + removed] {
                source.push('-');
                source.push_str(line);
            }
            for line in &inserted {
                source.push('+');
                source.push_str(line);
            }
            current.splice(start..start + removed, inserted);
            patches.push(source);
        }
        let refs: Vec<_> = patches.iter().map(String::as_str).collect();
        let expected = current.concat();
        assert_eq!(
            apply("file.txt", "", &original.concat(), &refs).unwrap(),
            expected
        );
        let composed = net(&refs);
        let actual = composed.map_or_else(
            || original.concat(),
            |source| apply("file.txt", "", &original.concat(), &[&source]).unwrap(),
        );
        assert_eq!(actual, expected, "round {round}");
    }
}
#[test]
fn limits_fall_back_without_allocating_sparse_holes() {
    let sparse = patch("@@ -1000000000 +1000000000 @@\n-a\n+b\n");
    assert!(net(&[&sparse]).unwrap().len() < 150);
    let many = vec![sparse.as_str(); MAX_PATCHES + 1];
    assert!(compose("file.txt", "", &many).is_err());
    let oversized = "x".repeat(MAX_BYTES + 1);
    assert!(compose("file.txt", "", &[&oversized]).is_err());
}

#[test]
fn ordinary_git_headers_are_checked_and_sparse_insertions_stay_sparse() {
    let source = "diff --git a/file.txt b/file.txt\nindex 123abc..456def 100644\n--- a/file.txt\n+++ b/file.txt\n@@ -1 +1 @@\n-a\n+b\n";
    let result = net(&[source]).unwrap();
    assert_eq!(apply("file.txt", "", "a\n", &[&result]).unwrap(), "b\n");
    let insert = patch("@@ -10,0 +11,1 @@\n+inserted\n");
    let result = net(&[&insert]).unwrap();
    assert!(result.contains("@@ -10,0 +11,1 @@"));
    assert_eq!(result.lines().count(), 4);
}

#[test]
fn known_write_hunks_reject_out_of_bounds_and_interior_eof() {
    for source in [
        patch("@@ -3 +3 @@\n-a\n+b\n"),
        patch("@@ -3,0 +4,1 @@\n+b\n"),
        patch("@@ -1 +1 @@\n-a\n+b\n\\ No newline at end of file\n"),
        patch("@@ -1 +1 @@\n-a\n\\ No newline at end of file\n+b\n"),
    ] {
        assert!(apply("file.txt", "", "a\nx\n", &[&source]).is_err());
    }
    let interior = patch("@@ -1,0 +2,1 @@\n+b\n");
    assert!(apply("file.txt", "", "a", &[&interior]).is_err());
    let valid = patch(
        "@@ -1 +1,2 @@\n-a\n+A\n+inserted\n@@ -3 +4 @@\n-c\n+final\n\\ No newline at end of file\n",
    );
    assert_eq!(
        apply("file.txt", "", "a\nb\nc\n", &[&valid]).unwrap(),
        "A\ninserted\nb\nfinal"
    );
}
