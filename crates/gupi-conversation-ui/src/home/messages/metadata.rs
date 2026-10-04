use super::activity::RunOutcome;
use super::activity::run_outcome;
use super::*;
use fluent_bundle::FluentArgs;
use gupi_settings::i18n::t_with_args;
use time::OffsetDateTime;
use time::UtcOffset;

pub(in crate::home) fn process_title(
    messages: &[DisplayMessage],
    active: bool,
    started_at: Option<i64>,
    cx: &App,
) -> String {
    let elapsed = elapsed_ms(messages, started_at, active, now_ms());
    let mut args = FluentArgs::new();
    args.set("duration", elapsed.map(duration_label).unwrap_or_default());
    t_with_args(cx, process_title_key(messages, active), &args)
        .trim()
        .to_owned()
}

fn process_title_key(messages: &[DisplayMessage], active: bool) -> &'static str {
    if active {
        "conversation-working-duration"
    } else {
        match run_outcome(messages) {
            RunOutcome::Complete => "conversation-processed",
            RunOutcome::Failed => "conversation-processed-failed",
            RunOutcome::Stopped => "conversation-processed-stopped",
        }
    }
}
pub(super) fn now_ms() -> i64 {
    (OffsetDateTime::now_utc().unix_timestamp_nanos() / 1_000_000) as i64
}
fn elapsed_ms(
    messages: &[DisplayMessage],
    started_at: Option<i64>,
    active: bool,
    now: i64,
) -> Option<i64> {
    let start = started_at.or_else(|| {
        messages.iter().find(|m| m.role() == "assistant")?.value["timestamp"].as_i64()
    })?;
    let end = if active {
        now
    } else {
        messages
            .iter()
            .rfind(|m| m.role() != "custom")?
            .completed_at?
    };
    end.checked_sub(start).filter(|elapsed| *elapsed >= 0)
}
fn duration_label(ms: i64) -> String {
    let seconds = ms / 1000;
    if seconds < 1 {
        "<1s".into()
    } else if seconds < 60 {
        format!("{seconds}s")
    } else if seconds < 3600 {
        format!("{}m {}s", seconds / 60, seconds % 60)
    } else {
        format!("{}h {}m", seconds / 3600, seconds % 3600 / 60)
    }
}

pub(super) fn timestamp(message: &DisplayMessage) -> Option<String> {
    let millis = if message.role() == "user" {
        message.value["timestamp"].as_i64()
    } else {
        message
            .completed_at
            .or_else(|| message.value["timestamp"].as_i64())
    }?;
    let offset = UtcOffset::current_local_offset().unwrap_or(UtcOffset::UTC);
    let date = OffsetDateTime::from_unix_timestamp_nanos(i128::from(millis) * 1_000_000)
        .ok()?
        .to_offset(offset);
    let now = OffsetDateTime::now_utc().to_offset(offset);
    let clock = format!("{:02}:{:02}", date.hour(), date.minute());
    Some(if date.date() == now.date() {
        clock
    } else {
        format!("{} {clock}", date.date())
    })
}

pub(super) fn usage_fields(message: &DisplayMessage, cx: &App) -> Vec<(String, String)> {
    let mut fields = vec![];
    for (field, label) in [
        ("model", "conversation-usage-model"),
        ("provider", "conversation-usage-provider"),
    ] {
        if let Some(value) = message.value[field].as_str().filter(|s| !s.is_empty()) {
            fields.push((t(cx, label), value.to_owned()));
        }
    }
    if let Some(usage) = message.value.get("usage").filter(|v| v.is_object()) {
        for (field, label) in [
            ("input", "conversation-usage-input"),
            ("output", "conversation-usage-output"),
            ("cacheRead", "conversation-usage-cache-read"),
            ("cacheWrite", "conversation-usage-cache-write"),
            ("totalTokens", "conversation-usage-total"),
        ] {
            if let Some(value) = usage[field].as_u64() {
                fields.push((t(cx, label), value.to_string()));
            }
        }
        if let Some(cost) = usage["cost"]["total"]
            .as_f64()
            .filter(|cost| cost.is_finite() && *cost >= 0.)
        {
            fields.push((t(cx, "conversation-usage-cost"), format!("${cost:.6}")));
        }
    }
    fields
}

#[cfg(test)]
mod tests {
    use super::duration_label;
    use super::elapsed_ms;
    use super::process_title_key;
    use gupi_conversation::history::DisplayMessage;
    #[test]
    fn process_title_tracks_the_last_attempt_instead_of_historical_errors() {
        let message = |role, reason| {
            let mut record = DisplayMessage::new(
                "m".into(),
                serde_json::json! ({ "role" : role , "stopReason" : reason }),
            );
            record.entry = None;
            record.final_answer_part = None;
            record.completed_at = None;
            record
        };
        let mut messages = vec![message("assistant", "error")];
        assert_eq!(
            process_title_key(&messages, true),
            "conversation-working-duration"
        );
        assert_eq!(
            process_title_key(&messages, false),
            "conversation-processed-failed"
        );
        messages.push(message("assistant", "pending"));
        assert_eq!(
            process_title_key(&messages, true),
            "conversation-working-duration"
        );
        messages[1] = message("assistant", "toolUse");
        messages.push(message("toolResult", "error"));
        assert_eq!(
            process_title_key(&messages, true),
            "conversation-working-duration"
        );
        messages.push(message("assistant", "stop"));
        messages.push(message("custom", "error"));
        assert_eq!(
            process_title_key(&messages, false),
            "conversation-processed"
        );
        messages[3] = message("assistant", "aborted");
        assert_eq!(
            process_title_key(&messages, false),
            "conversation-processed-stopped"
        );
        messages[3] = message("assistant", "error");
        assert_eq!(
            process_title_key(&messages, false),
            "conversation-processed-failed"
        );
    }
    #[test]
    fn duration_uses_completion_instead_of_last_request_start() {
        let message = |started, completed| {
            let mut record = DisplayMessage::new(
                "m".into(),
                serde_json::json! ({ "role" : "assistant" , "timestamp" : started }),
            );
            record.entry = None;
            record.final_answer_part = None;
            record.completed_at = completed;
            record
        };
        let messages = vec![message(1000, Some(2500)), message(3000, Some(6500))];
        assert_eq!(elapsed_ms(&messages, None, false, 0), Some(5500));
        assert_eq!(elapsed_ms(&[message(3000, None)], None, false, 0), None);
        assert_eq!(
            elapsed_ms(&[message(3000, Some(2000))], None, false, 0),
            None
        );
        // Count from Pi's user timestamp even before an assistant exists.
        assert_eq!(elapsed_ms(&[], Some(500), true, 1000), Some(500));
        // Final answer output keeps counting, and only the end freezes the value.
        assert_eq!(elapsed_ms(&messages, Some(500), true, 5000), Some(4500));
        assert_eq!(elapsed_ms(&messages, Some(500), true, 6000), Some(5500));
        assert_eq!(elapsed_ms(&messages, Some(500), false, 9000), Some(6000));
        let mut with_plugin = messages.clone();
        let mut plugin = message(8000, Some(9000));
        plugin.value["role"] = serde_json::json!("custom");
        with_plugin.push(plugin);
        assert_eq!(
            elapsed_ms(&with_plugin, Some(500), false, 10000),
            Some(6000)
        );
        assert_eq!(duration_label(62000), "1m 2s");
    }
}
