//! Scope-local keyed leaves. Form occurrence identity is distinct from JSON keys.
use super::forms::{RawValue, Slot};
use gpui_form::FormSchema;
use serde_json::{Map, Value};
use std::collections::BTreeMap;

#[derive(Clone, Default, PartialEq, FormSchema)]
pub(super) enum ThinkingSlot {
    #[default]
    Unset,
    Set(ThinkingRows),
    Unreadable(RawValue),
}
#[derive(Clone, Default, PartialEq, FormSchema)]
pub(super) struct ThinkingRows {
    #[form(items)]
    pub(super) rows: Vec<ThinkingRow>,
}
#[derive(Clone, PartialEq, FormSchema)]
pub(super) struct ThinkingRow {
    pub(super) key: String,
    pub(super) level: Slot<String>,
}
#[derive(Clone, Default, PartialEq, FormSchema)]
pub(super) enum CompactionSlot {
    #[default]
    Unset,
    Set(CompactionRows),
    Unreadable(RawValue),
}
#[derive(Clone, Default, PartialEq, FormSchema)]
pub(super) struct CompactionRows {
    #[form(items)]
    pub(super) rows: Vec<CompactionRow>,
}
#[derive(Clone, PartialEq, FormSchema)]
pub(super) struct CompactionRow {
    pub(super) key: String,
    pub(super) reserve: Slot<u64>,
    pub(super) keep: Slot<u64>,
    pub(super) extra: Map<String, Value>,
    pub(super) unreadable: Option<Value>,
}

impl ThinkingSlot {
    pub(super) fn decode(value: Slot<Value>) -> Self {
        match value {
            Slot::Unset => Self::Unset,
            Slot::Set(Value::Object(map)) | Slot::Unreadable(Value::Object(map)) => {
                Self::Set(ThinkingRows {
                    rows: map
                        .into_iter()
                        .map(|(key, value)| ThinkingRow {
                            key,
                            level: Slot::decode(Slot::Set(value)),
                        })
                        .collect(),
                })
            }
            Slot::Set(value) | Slot::Unreadable(value) => Self::Unreadable(RawValue { value }),
            Slot::Invalid(_) => unreachable!("map root is not a text control"),
        }
    }
    pub(super) fn encode(&self) -> Slot<Value> {
        match self {
            Self::Unset => Slot::Unset,
            Self::Unreadable(raw) => Slot::Unreadable(raw.value.clone()),
            Self::Set(rows) => Slot::Set(Value::Object(
                rows.rows
                    .iter()
                    .filter_map(|row| row.level.encode().raw().map(|v| (row.key.clone(), v)))
                    .collect(),
            )),
        }
    }
    pub(super) fn leaves(&self) -> BTreeMap<Vec<String>, Slot<Value>> {
        match self {
            Self::Set(rows) => rows
                .rows
                .iter()
                .map(|r| {
                    (
                        vec!["modelThinkingLevels".into(), r.key.clone()],
                        r.level.encode(),
                    )
                })
                .collect(),
            Self::Unset => BTreeMap::new(),
            Self::Unreadable(_) => {
                BTreeMap::from([(vec!["modelThinkingLevels".into()], self.encode())])
            }
        }
    }
}
impl CompactionSlot {
    pub(super) fn decode(value: Slot<Value>) -> Self {
        match value {
            Slot::Unset => Self::Unset,
            Slot::Set(Value::Object(map)) | Slot::Unreadable(Value::Object(map)) => {
                Self::Set(CompactionRows {
                    rows: map
                        .into_iter()
                        .map(|(key, value)| {
                            let Value::Object(mut extra) = value else {
                                return CompactionRow {
                                    key,
                                    reserve: Slot::Unset,
                                    keep: Slot::Unset,
                                    extra: Map::new(),
                                    unreadable: Some(value),
                                };
                            };
                            let reserve = count(extra.shift_remove("reserveTokens"));
                            let keep = count(extra.shift_remove("keepRecentTokens"));
                            CompactionRow {
                                key,
                                reserve,
                                keep,
                                extra,
                                unreadable: None,
                            }
                        })
                        .collect(),
                })
            }
            Slot::Set(value) | Slot::Unreadable(value) => Self::Unreadable(RawValue { value }),
            Slot::Invalid(_) => unreachable!("map root is not a text control"),
        }
    }
    pub(super) fn encode(&self) -> Slot<Value> {
        match self {
            Self::Unset => Slot::Unset,
            Self::Unreadable(raw) => Slot::Unreadable(raw.value.clone()),
            Self::Set(rows) => Slot::Set(Value::Object(
                rows.rows
                    .iter()
                    .map(|row| {
                        if let Some(value) = &row.unreadable {
                            return (row.key.clone(), value.clone());
                        }
                        let mut object = row.extra.clone();
                        for (key, value) in [
                            ("reserveTokens", row.reserve.encode()),
                            ("keepRecentTokens", row.keep.encode()),
                        ] {
                            if let Some(value) = value.raw() {
                                object.insert(key.into(), value);
                            }
                        }
                        (row.key.clone(), Value::Object(object))
                    })
                    .collect(),
            )),
        }
    }
    pub(super) fn leaves(&self) -> BTreeMap<Vec<String>, Slot<Value>> {
        match self {
            Self::Set(rows) => rows
                .rows
                .iter()
                .flat_map(|r| {
                    let base = vec!["compaction".into(), "modelOverrides".into(), r.key.clone()];
                    if let Some(value) = &r.unreadable {
                        return vec![(base, Slot::Unreadable(value.clone()))];
                    }
                    [
                        ("reserveTokens", r.reserve.encode()),
                        ("keepRecentTokens", r.keep.encode()),
                    ]
                    .into_iter()
                    .map(|(token, value)| {
                        let mut path = base.clone();
                        path.push(token.into());
                        (path, value)
                    })
                    .collect()
                })
                .collect(),
            Self::Unset => BTreeMap::new(),
            Self::Unreadable(_) => BTreeMap::from([(
                vec!["compaction".into(), "modelOverrides".into()],
                self.encode(),
            )]),
        }
    }
}
fn count(value: Option<Value>) -> Slot<u64> {
    match value {
        None => Slot::Unset,
        Some(value)
            if value
                .as_u64()
                .is_some_and(|n| n <= super::fields::MAX_SAFE_INTEGER) =>
        {
            Slot::decode(Slot::Set(value))
        }
        Some(value) => Slot::Unreadable(value),
    }
}

impl ThinkingSlot {
    pub(super) fn merge(current: &Self, next: &Self, since: &Self) -> Self {
        if current == since {
            return next.clone();
        }
        if matches!(current, Self::Unreadable(_))
            || matches!(next, Self::Unreadable(_))
            || matches!(since, Self::Unreadable(_))
        {
            return if current != since {
                current.clone()
            } else {
                next.clone()
            };
        }
        let rows = |slot: &Self| match slot {
            Self::Set(rows) => rows
                .rows
                .iter()
                .map(|r| (r.key.clone(), r.level.clone()))
                .collect::<BTreeMap<_, _>>(),
            _ => BTreeMap::new(),
        };
        // Disk order is the form baseline order. Preserve it, then append pending
        // local-only rows in their current order so reconciliation cannot create
        // a dirty draft merely by sorting model keys.
        let mut keys = match next {
            Self::Set(rows) => rows
                .rows
                .iter()
                .map(|row| row.key.clone())
                .collect::<Vec<_>>(),
            _ => Vec::new(),
        };
        if let Self::Set(rows) = current {
            for row in &rows.rows {
                if !keys.contains(&row.key) {
                    keys.push(row.key.clone());
                }
            }
        }
        let (current, next, since) = (rows(current), rows(next), rows(since));
        let rows = keys
            .into_iter()
            .filter_map(|key| {
                let current = current.get(&key).unwrap_or(&Slot::Unset);
                let since = since.get(&key).unwrap_or(&Slot::Unset);
                let value = if current != since {
                    current
                } else {
                    next.get(&key).unwrap_or(&Slot::Unset)
                };
                (!matches!(value, Slot::Unset)).then(|| ThinkingRow {
                    key: key.clone(),
                    level: value.clone(),
                })
            })
            .collect::<Vec<_>>();
        if rows.is_empty() {
            Self::Unset
        } else {
            Self::Set(ThinkingRows { rows })
        }
    }
}
impl CompactionSlot {
    pub(super) fn merge(current: &Self, next: &Self, since: &Self) -> Self {
        if current == since {
            return next.clone();
        }
        if matches!(current, Self::Unreadable(_))
            || matches!(next, Self::Unreadable(_))
            || matches!(since, Self::Unreadable(_))
        {
            return if current != since {
                current.clone()
            } else {
                next.clone()
            };
        }
        let rows = |slot: &Self| match slot {
            Self::Set(rows) => rows
                .rows
                .iter()
                .map(|r| (r.key.clone(), r.clone()))
                .collect::<BTreeMap<_, _>>(),
            _ => BTreeMap::new(),
        };
        // Disk order is the form baseline order. Preserve it, then append pending
        // local-only rows in their current order so reconciliation cannot create
        // a dirty draft merely by sorting model keys.
        let mut keys = match next {
            Self::Set(rows) => rows
                .rows
                .iter()
                .map(|row| row.key.clone())
                .collect::<Vec<_>>(),
            _ => Vec::new(),
        };
        if let Self::Set(rows) = current {
            for row in &rows.rows {
                if !keys.contains(&row.key) {
                    keys.push(row.key.clone());
                }
            }
        }
        let (current, next, since) = (rows(current), rows(next), rows(since));
        let rows = keys
            .into_iter()
            .filter_map(|key| {
                let empty = CompactionRow {
                    key: key.clone(),
                    reserve: Slot::Unset,
                    keep: Slot::Unset,
                    extra: Map::new(),
                    unreadable: None,
                };
                let (c, n, s) = (
                    current.get(&key).unwrap_or(&empty),
                    next.get(&key).unwrap_or(&empty),
                    since.get(&key).unwrap_or(&empty),
                );
                if c.unreadable.is_some() || n.unreadable.is_some() || s.unreadable.is_some() {
                    let row = if c != s { c } else { n };
                    return (row != &empty).then(|| row.clone());
                }
                let row = CompactionRow {
                    key: key.clone(),
                    reserve: if c.reserve != s.reserve {
                        c.reserve.clone()
                    } else {
                        n.reserve.clone()
                    },
                    keep: if c.keep != s.keep {
                        c.keep.clone()
                    } else {
                        n.keep.clone()
                    },
                    extra: n.extra.clone(),
                    unreadable: None,
                };
                (row != empty).then_some(row)
            })
            .collect::<Vec<_>>();
        if rows.is_empty() {
            Self::Unset
        } else {
            Self::Set(CompactionRows { rows })
        }
    }
}
