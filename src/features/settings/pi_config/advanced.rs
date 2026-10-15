//! Pi v1.2.0 resolution rules; no filesystem or runtime side effects.
use serde_json::Value;

pub(super) const DEFAULT_TOOLS: &[&str] = &["read", "bash", "edit", "write"];
pub(super) const KNOWN_TOOLS: &[&str] = &[
    "read",
    "bash",
    "powershell",
    "edit",
    "write",
    "grep",
    "find",
    "ls",
    "codemode",
    "tool_search",
];

fn modifier(s: &str) -> bool {
    s.starts_with(['+', '-'])
}

/// Port of settings-manager.ts mergeDefaultTools + resolveDefaultTools.
pub(super) fn startup_tools(global: Option<&Value>, project: Option<&Value>) -> Vec<String> {
    let merged = match (global, project) {
        (Some(Value::Array(base)), Some(Value::Array(overrides)))
            if overrides.iter().all(|v| v.as_str().is_some_and(modifier)) =>
        {
            Some(base.iter().chain(overrides).cloned().collect::<Vec<_>>())
        }
        (_, Some(value)) | (Some(value), None) => {
            Some(value.as_array().cloned().unwrap_or_default())
        }
        (None, None) => None,
    };
    let Some(entries) = merged else {
        return DEFAULT_TOOLS.iter().map(|s| (*s).into()).collect();
    };
    let entries: Vec<_> = entries.iter().filter_map(Value::as_str).collect();
    let mut tools: Vec<String> = entries
        .iter()
        .filter(|s| !modifier(s))
        .map(|s| (*s).into())
        .collect();
    if tools.is_empty() && !entries.is_empty() {
        tools = DEFAULT_TOOLS.iter().map(|s| (*s).into()).collect();
    }
    for entry in entries.into_iter().filter(|s| modifier(s)) {
        let name = &entry[1..];
        let index = tools.iter().position(|s| s == name);
        if entry.starts_with('+') && index.is_none() && !name.is_empty() {
            tools.push(name.into());
        } else if entry.starts_with('-')
            && let Some(index) = index
        {
            tools.remove(index);
        }
    }
    tools
}

/// Equivalent to Pi's finite, nonnegative Number conversion and floor.
pub(super) fn timeout(value: &Value) -> Option<f64> {
    let value = match value {
        Value::Number(n) => n.as_f64()?,
        Value::String(s) => {
            let s = s.trim();
            if s.eq_ignore_ascii_case("disabled") {
                return Some(0.);
            }
            if s.is_empty() {
                return None;
            }
            if let Some((radix, number)) =
                [(16, "0x"), (2, "0b"), (8, "0o")]
                    .into_iter()
                    .find_map(|(radix, prefix)| {
                        s.to_ascii_lowercase()
                            .strip_prefix(prefix)
                            .map(|n| (radix, n.to_owned()))
                    })
            {
                // Number() accepts finite radix strings beyond the u64 range.
                // Keep their original spelling in the draft; this conversion
                // determines validity and the disabled state, not stored data.
                if number.is_empty() {
                    return None;
                }
                number.chars().try_fold(0., |value, digit| {
                    Some(value * f64::from(radix) + f64::from(digit.to_digit(radix)?))
                })?
            } else {
                s.parse::<f64>().ok()?
            }
        }
        _ => return None,
    };
    (value.is_finite() && value >= 0.).then(|| value.floor())
}

pub(super) fn telemetry(value: Option<&str>) -> Option<bool> {
    value.map(|s| s == "1" || s.eq_ignore_ascii_case("true") || s.eq_ignore_ascii_case("yes"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn tools_follow_order_and_empty_project_is_a_modifier_list() {
        assert_eq!(
            startup_tools(Some(&json!(["read", "bash"])), Some(&json!([]))),
            ["read", "bash"]
        );
        assert_eq!(
            startup_tools(Some(&json!([])), Some(&json!(["+codemode"]))),
            ["read", "bash", "edit", "write", "codemode"]
        );
        assert_eq!(
            startup_tools(None, Some(&json!(["read", "+codemode", "-read", "+bash"]))),
            ["codemode", "bash"]
        );
        assert!(startup_tools(Some(&json!([])), None).is_empty());
        assert!(startup_tools(None, Some(&json!([]))).is_empty());
        assert_eq!(startup_tools(None, None), DEFAULT_TOOLS);
        assert_eq!(
            startup_tools(None, Some(&json!([" custom ", "+x", "-x"]))),
            [" custom "]
        );
    }
    #[test]
    fn timeout_and_telemetry_match_pi_coercions() {
        for (raw, expected) in [
            (json!("DISABLED"), Some(0.)),
            (json!(0), Some(0.)),
            (json!("300"), Some(300.)),
            (json!(1.5), Some(1.)),
            (json!("0x10"), Some(16.)),
            (json!("0x10000000000000000"), Some(18446744073709551616.)),
            (json!("0x"), None),
            (json!(""), None),
            (json!(-1), None),
            (json!(true), None),
            (json!("Infinity"), None),
        ] {
            assert_eq!(timeout(&raw), expected);
        }
        assert_eq!(telemetry(None), None);
        for text in ["1", "true", "TRUE", "yes", "YeS"] {
            assert_eq!(telemetry(Some(text)), Some(true));
        }
        for text in ["", "0", "false", "no", " true ", "2"] {
            assert_eq!(telemetry(Some(text)), Some(false));
        }
    }
}
