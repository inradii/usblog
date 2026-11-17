use crate::types::{FilterOpts, UsbEvent};

/// Check if a field value matches any of the provided filter strings
fn field_matches(value: &Option<String>, filters: &[String]) -> bool {
    if filters.is_empty() {
        return true;
    }
    match value {
        Some(v) => filters
            .iter()
            .any(|f| v.to_lowercase().contains(&f.to_lowercase())),
        None => false,
    }
}

/// Apply filtering options to a list of USB events and return the filtered list
pub fn apply_filters(events: Vec<UsbEvent>, opts: &FilterOpts) -> Vec<UsbEvent> {
    let mut out: Vec<UsbEvent> = events
        .into_iter()
        .filter(|e| {
            field_matches(&e.host, &opts.host)
                && field_matches(&e.vid, &opts.vid)
                && field_matches(&e.pid, &opts.pid)
                && field_matches(&e.prod, &opts.prod)
                && field_matches(&e.manufact, &opts.manufact)
                && field_matches(&e.serial, &opts.serial)
                && field_matches(&e.port, &opts.port)
                && (opts.date.is_empty()
                    || opts.date.iter().any(|d| {
                        e.conn
                            .as_deref()
                            .map_or(false, |c| c.starts_with(d.as_str()))
                    }))
        })
        .collect();

    if let Some(n) = opts.num {
        out.truncate(n);
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(vid: &str, pid: &str) -> UsbEvent {
        UsbEvent {
            vid: Some(vid.to_string()),
            pid: Some(pid.to_string()),
            ..Default::default()
        }
    }

    #[test]
    fn filter_by_vid_keeps_matching_events() {
        let events = vec![event("1234", "abcd"), event("5678", "abcd")];
        let opts = FilterOpts {
            vid: vec!["1234".to_string()],
            ..Default::default()
        };
        let result = apply_filters(events, &opts);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].vid.as_deref(), Some("1234"));
    }

    #[test]
    fn num_limit_truncates_results() {
        let events = vec![event("1", "a"), event("2", "b"), event("3", "c")];
        let opts = FilterOpts {
            num: Some(2),
            ..Default::default()
        };
        let result = apply_filters(events, &opts);
        assert_eq!(result.len(), 2);
    }
}
