use super::*;

fn document(value: Value) -> Document {
    let Value::Object(root) = value else {
        panic!("object")
    };
    Document::from_object(root)
}

fn project(global: Value, project: Value) -> Files {
    Files {
        global: document(global),
        project: Some(document(project)),
    }
}

#[test]
fn project_values_inherit_leaf_by_leaf_from_global_then_defaults() {
    let files = project(
        json!({"compaction":{"enabled":false,"reserveTokens":1},"steeringMode":"all"}),
        json!({"compaction":{"reserveTokens":2}}),
    );
    let reserve = files.resolve(Field::ReserveTokens, None);
    assert_eq!(
        (reserve.values, reserve.source),
        (vec![Some(json!(2))], Source::Project)
    );
    assert!(reserve.overridden);
    let enabled = files.resolve(Field::Compaction, None);
    assert_eq!(
        (enabled.values, enabled.source),
        (vec![Some(json!(false))], Source::Global)
    );
    assert!(!enabled.overridden);
    let keep = files.resolve(Field::KeepRecentTokens, None);
    assert_eq!(
        (keep.values, keep.source),
        (vec![Some(json!(20000))], Source::Default)
    );

    let global = Files {
        global: document(json!({"steeringMode":"all"})),
        project: None,
    };
    let steering = global.resolve(Field::Steering, None);
    assert_eq!(
        (steering.values, steering.source),
        (vec![Some(json!("all"))], Source::Global)
    );
    let follow = global.resolve(Field::FollowUp, None);
    assert_eq!(follow.source, Source::Default);
}

#[test]
fn drafts_show_the_pending_value_or_the_inherited_one() {
    let files = project(
        json!({"retry":{"maxRetries":5}}),
        json!({"retry":{"maxRetries":1}}),
    );
    let set = files.resolve(Field::MaxRetries, Some(&[Slot::Set(json!(5))]));
    assert_eq!(
        (set.values, set.source),
        (vec![Some(json!(5))], Source::Project)
    );
    let removed = files.resolve(Field::MaxRetries, Some(&[Slot::Unset]));
    assert_eq!(
        (removed.values, removed.source),
        (vec![Some(json!(5))], Source::Global)
    );
    let invalid = files.resolve(Field::MaxRetries, Some(&[Slot::Invalid("x".into())]));
    assert_eq!(invalid.values, vec![Some(json!(1))]);
}

#[test]
fn a_partial_model_override_is_kept_as_read() {
    let files = project(
        json!({"defaultProvider":"anthropic","defaultModel":"a"}),
        json!({"defaultModel":"b"}),
    );
    let model = files.resolve(Field::Model, None);
    assert_eq!(
        model.values,
        vec![Some(json!("anthropic")), Some(json!("b"))]
    );
    assert_eq!(model.source, Source::Project);
    // Nothing is written unless the user edits the pair.
    assert!(
        files
            .changes(
                &Snapshot::project(files.edited()),
                &Snapshot::project(files.edited()),
                None
            )
            .is_empty()
    );

    let mut removal = Snapshot::project(files.edited());
    removal.set(Field::Model, vec![Slot::Unset, Slot::Unset]);
    let changes = files.changes(&removal, &Snapshot::project(files.edited()), None);
    assert_eq!(changes.len(), 1, "only the leaf present in the project");
    assert_eq!(changes[0].leaf(), &["defaultModel"][..]);
}

#[test]
fn changes_carry_their_base_and_skip_no_ops_and_invalid_text() {
    let files = project(
        json!({}),
        json!({"steeringMode":"all","compaction":{"enabled":true}}),
    );
    let mut current = Snapshot::project(files.edited());
    current.set(Field::Steering, vec![Slot::Set(json!("all"))]);
    current.set(Field::Compaction, vec![Slot::Set(json!(false))]);
    current.set(Field::Retry, vec![Slot::Set(json!(true))]);
    current.set(Field::MaxRetries, vec![Slot::Invalid("-1".into())]);
    current.set(Field::FollowUp, vec![Slot::Unset]);
    let changes = files.changes(&current, &Snapshot::project(files.edited()), None);
    assert_eq!(
        changes,
        vec![
            Change::new(
                &["compaction", "enabled"],
                Some(json!(true)),
                Some(json!(false))
            ),
            // Setting a value equal to the inherited default is still an override.
            Change::new(&["retry", "enabled"], None, Some(json!(true))),
        ]
    );
}

#[test]
fn counts_and_display_text() {
    assert_eq!(parse_count(" 12 "), Some(json!(12)));
    assert_eq!(parse_count("-1"), None);
    assert_eq!(parse_count("1.5"), None);
    assert_eq!(parse_count(""), None);
    assert_eq!(display(None), "");
}

fn model(reasoning: bool, map: Value) -> pi_rpc::protocol::Model {
    let mut value = json!({"id":"m","name":"M","provider":"p","reasoning":reasoning});
    if !map.is_null() {
        value["thinkingLevelMap"] = map;
    }
    serde_json::from_value(value).unwrap()
}

#[test]
fn supported_levels_follow_pi_model_metadata() {
    // Include shapes from Pi's catalog: disabled levels, high-only and max.
    for (reasoning, map, expected) in [
        (false, Value::Null, vec!["off"]),
        (
            true,
            Value::Null,
            vec!["off", "minimal", "low", "medium", "high"],
        ),
        (
            true,
            json!({"minimal": null, "xhigh": "x", "max": null}),
            vec!["off", "low", "medium", "high", "xhigh"],
        ),
        (
            true,
            json!({"off": null, "minimal": null, "low": null, "medium": null, "high": "high", "xhigh": null, "max": null}),
            vec!["high"],
        ),
        (
            true,
            json!({"off": "none", "minimal": null, "low": "low", "medium": "medium", "high": "high", "xhigh": "xhigh", "max": "max"}),
            vec!["off", "low", "medium", "high", "xhigh", "max"],
        ),
        (false, json!({"off": null, "xhigh": "xhigh"}), vec!["off"]),
    ] {
        assert_eq!(supported_levels(&model(reasoning, map)), expected);
    }
}

#[test]
fn typed_projection_preserves_unreadable_and_only_validates_pending_fields() {
    let files = project(
        json!({}),
        json!({"enabledModels":[" "],"retry":{"maxRetries":"bad"},"unknown":[1]}),
    );
    let baseline = Snapshot::project(files.edited());
    assert_eq!(
        baseline.value(Field::MaxRetries),
        vec![Slot::Unreadable(json!("bad"))]
    );
    assert!(files.changes(&baseline, &baseline, None).is_empty());
    assert!(!baseline.invalid(Field::ModelScope, &baseline));
    let mut current = baseline.clone();
    current.set(Field::Compaction, vec![Slot::Set(json!(false))]);
    assert!(!current.invalid(Field::ModelScope, &baseline));
    assert_eq!(
        files
            .changes(&current, &Snapshot::project(files.edited()), None)
            .len(),
        1
    );
    current.set(Field::MaxRetries, vec![Slot::Invalid("-".into())]);
    assert!(current.invalid(Field::MaxRetries, &baseline));
    current.set(Field::ModelScope, vec![Slot::Set(json!(["a", " "]))]);
    assert!(current.invalid(Field::ModelScope, &baseline));
}

#[test]
fn snapshot_merge_preserves_newer_edits_and_adopts_unedited_disk_values() {
    let old = Snapshot::project(&document(json!({"retry":{"maxRetries":1,"baseDelayMs":2}})));
    let next = Snapshot::project(&document(json!({"retry":{"maxRetries":4,"baseDelayMs":6}})));
    let mut current = old.clone();
    current.set(Field::MaxRetries, vec![Slot::Invalid("-".into())]);
    let merged = Snapshot::merge(&current, &next, &old);
    assert_eq!(
        merged.value(Field::MaxRetries),
        vec![Slot::Invalid("-".into())]
    );
    assert_eq!(merged.value(Field::BaseDelay), vec![Slot::Set(json!(6))]);
    // A save reply uses the prepared value, preserving even a newer revert to the original value.
    let mut prepared = old.clone();
    prepared.set(Field::MaxRetries, vec![Slot::Set(json!(3))]);
    let merged = Snapshot::merge(&old, &prepared, &prepared);
    assert_eq!(
        merged.value(Field::MaxRetries),
        old.value(Field::MaxRetries)
    );
}

#[test]
fn safe_counts_raw_timeouts_and_argv_are_preserved() {
    assert_eq!(
        parse_count("9007199254740991"),
        Some(json!(9007199254740991u64))
    );
    assert_eq!(parse_count("9007199254740992"), None);
    for raw in [
        json!("disabled"),
        json!("DISABLED"),
        json!("300"),
        json!(1.5),
        json!(0),
    ] {
        let doc = document(json!({"httpIdleTimeoutMs":raw,"unknown":true}));
        let baseline = Snapshot::project(&doc);
        let mut current = baseline.clone();
        current.set(Field::Transport, vec![Slot::Set(json!("sse"))]);
        let files = Files {
            global: doc,
            project: None,
        };
        let changes = files.changes(&current, &baseline, Some(Page::Network));
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].leaf(), &["transport"]);
        assert_eq!(current.value(Field::HttpTimeout), vec![Slot::Set(raw)]);
    }
    let doc = document(json!({"npmCommand":[" node ",""," --flag "]}));
    let baseline = Snapshot::project(&doc);
    assert_eq!(
        baseline.value(Field::NpmCommand),
        vec![Slot::Set(json!([" node ", "", " --flag "]))]
    );
    let mut current = baseline.clone();
    current.set(Field::NpmCommand, vec![Slot::Set(json!(["", "arg"]))]);
    assert!(current.invalid(Field::NpmCommand, &baseline));
    assert!(!baseline.invalid(Field::NpmCommand, &baseline));
}

#[test]
fn mapped_changes_are_token_leaves_and_reconcile_independently() {
    let doc = document(
        json!({"compaction":{"modelOverrides":{"p/m":{"reserveTokens":100,"keepRecentTokens":200,"unknown":true}}},"modelThinkingLevels":{"p/m":"high","missing/model":false}}),
    );
    let baseline = Snapshot::project(&doc);
    let mut current = baseline.clone();
    current.set(
        Field::ModelOverrides,
        vec![Slot::Set(
            json!({"p/m":{"reserveTokens":300,"keepRecentTokens":200,"unknown":true}}),
        )],
    );
    current.set(
        Field::ModelThinking,
        vec![Slot::Set(json!({"p/m":"low","missing/model":false}))],
    );
    let files = Files {
        global: doc,
        project: None,
    };
    let changes = files.changes(&current, &baseline, Some(Page::Conversation));
    assert_eq!(changes.len(), 2);
    assert!(
        changes
            .iter()
            .any(|c| c.leaf() == ["compaction", "modelOverrides", "p/m", "reserveTokens"])
    );
    assert!(
        changes
            .iter()
            .any(|c| c.leaf() == ["modelThinkingLevels", "p/m"])
    );
    let next = Snapshot::project(&document(
        json!({"compaction":{"modelOverrides":{"p/m":{"reserveTokens":150,"keepRecentTokens":250,"unknown":true},"p/new":{"reserveTokens":50}}},"modelThinkingLevels":{"p/m":"medium","missing/model":false,"p/new":"off"}}),
    ));
    let merged = Snapshot::merge(&current, &next, &baseline);
    assert_eq!(
        merged.value(Field::ModelOverrides),
        vec![Slot::Set(
            json!({"p/m":{"reserveTokens":300,"keepRecentTokens":250,"unknown":true},"p/new":{"reserveTokens":50}})
        )]
    );
    assert_eq!(
        merged.value(Field::ModelThinking),
        vec![Slot::Set(
            json!({"p/m":"low","missing/model":false,"p/new":"off"})
        )]
    );
}

#[test]
fn global_only_fields_never_generate_project_writes() {
    let files = project(json!({}), json!({"unknown":true}));
    let baseline = Snapshot::project(files.edited());
    let mut current = baseline.clone();
    for field in [Field::Proxy, Field::CacheWarming, Field::ProjectTrust] {
        current.set(field, vec![Slot::Set(json!("always"))]);
    }
    assert!(files.changes(&current, &baseline, None).is_empty());
}

#[test]
fn resource_arrays_keep_unsupported_values_and_one_conflict_leaf_each() {
    for field in Field::RESOURCE_PATHS {
        let key = field.leaves()[0][0];
        for value in [
            json!(null),
            json!(false),
            json!(["keep", 4]),
            json!({"keep":true}),
        ] {
            let files = Files {
                global: document(json!({key:value,"unknown":9})),
                project: None,
            };
            let baseline = Snapshot::project(files.edited());
            assert_eq!(baseline.value(field), vec![Slot::Unreadable(value.clone())]);
            assert!(files.changes(&baseline, &baseline, None).is_empty());
        }
        let files = Files {
            global: document(json!({key:["a","a","!x","+x","-x","builtin:mcp"]})),
            project: None,
        };
        let baseline = Snapshot::project(files.edited());
        let mut current = baseline.clone();
        current.set(field, vec![Slot::Set(json!(["a", "!x"]))]);
        let changes = files.changes(&current, &baseline, None);
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].leaf(), &[key]);
    }
}
