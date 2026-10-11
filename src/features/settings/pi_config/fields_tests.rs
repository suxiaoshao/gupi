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
    let set = files.resolve(Field::MaxRetries, Some(&Edit::Set(vec![json!(5)])));
    assert_eq!(
        (set.values, set.source),
        (vec![Some(json!(5))], Source::Project)
    );
    let removed = files.resolve(Field::MaxRetries, Some(&Edit::Remove));
    assert_eq!(
        (removed.values, removed.source),
        (vec![Some(json!(5))], Source::Global)
    );
    let invalid = files.resolve(Field::MaxRetries, Some(&Edit::Invalid("x".into())));
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
    assert!(files.changes(&BTreeMap::new()).is_empty());

    let removal = BTreeMap::from([(Field::Model, Edit::Remove)]);
    let changes = files.changes(&removal);
    assert_eq!(changes.len(), 1, "only the leaf present in the project");
    assert_eq!(changes[0].leaf(), &["defaultModel"][..]);
}

#[test]
fn changes_carry_their_base_and_skip_no_ops_and_invalid_text() {
    let files = project(
        json!({}),
        json!({"steeringMode":"all","compaction":{"enabled":true}}),
    );
    let drafts = BTreeMap::from([
        (Field::Steering, Edit::Set(vec![json!("all")])),
        (Field::Compaction, Edit::Set(vec![json!(false)])),
        (Field::Retry, Edit::Set(vec![json!(true)])),
        (Field::MaxRetries, Edit::Invalid("-1".into())),
        (Field::FollowUp, Edit::Remove),
    ]);
    let changes = files.changes(&drafts);
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
fn fields_are_assigned_to_pages_and_scopes() {
    let network: Vec<_> = Field::ALL
        .into_iter()
        .filter(|field| field.page() == Page::Network)
        .collect();
    assert_eq!(
        network,
        [
            Field::Proxy,
            Field::Retry,
            Field::MaxRetries,
            Field::BaseDelay,
            Field::MaxDelay
        ]
    );
    assert!(Field::Proxy.is_global_only());
    assert!(Field::ALL.iter().filter(|f| f.is_global_only()).count() == 1);
}

#[test]
fn counts_and_display_text() {
    assert_eq!(parse_count(" 12 "), Some(json!(12)));
    assert_eq!(parse_count("-1"), None);
    assert_eq!(parse_count("1.5"), None);
    assert_eq!(parse_count(""), None);
    assert_eq!(display(Some(&json!(3))), "3");
    assert_eq!(display(Some(&json!("http://p"))), "http://p");
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
    assert_eq!(supported_levels(&model(false, Value::Null)), ["off"]);
    assert_eq!(
        supported_levels(&model(true, Value::Null)),
        ["off", "minimal", "low", "medium", "high"]
    );
    assert_eq!(
        supported_levels(&model(
            true,
            json!({"minimal": null, "xhigh": "x", "max": null})
        )),
        ["off", "low", "medium", "high", "xhigh"]
    );
}

#[test]
fn supported_levels_match_pi_1_1_catalog_entries() {
    // Shapes taken from Pi 1.1.0 `get_available_models` output.
    let pro = json!({"off": null, "minimal": null, "low": null, "medium": null, "high": "high", "xhigh": null, "max": null});
    assert_eq!(supported_levels(&model(true, pro)), ["high"]);
    let luna = json!({"off": "none", "minimal": null, "low": "low", "medium": "medium", "high": "high", "xhigh": "xhigh", "max": "max"});
    assert_eq!(
        supported_levels(&model(true, luna)),
        ["off", "low", "medium", "high", "xhigh", "max"]
    );
    // A non-reasoning model only offers `off`, whatever its map says.
    assert_eq!(
        supported_levels(&model(false, json!({"off": null, "xhigh": "xhigh"}))),
        ["off"]
    );
}
