//! `report.md`: the short summary of a playtest for Claude.

use super::stats::FrameHistogram;
use super::{Event, Sample};
use serde_json::Value;
use std::fmt::Write;

/// At most this many events show in the event list of the report.
const MAX_EVENT_ROWS: usize = 300;

/// Everything that the report needs.
pub struct ReportInput<'a> {
    /// The content of `session.json`.
    pub session: &'a Value,
    pub samples: &'a [Sample],
    pub events: &'a [Event],
    /// All frame times. If it is missing (after a crash), the report
    /// uses the samples.
    pub histogram: Option<&'a FrameHistogram>,
    /// The game quit normally.
    pub quit_normally: bool,
}

fn value_text(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Null => "-".into(),
        Value::Array(items) => items.iter().map(value_text).collect::<Vec<_>>().join(", "),
        other => other.to_string(),
    }
}

fn clock(t: f64) -> String {
    let t = t.max(0.0);
    let minutes = (t / 60.0).floor();
    format!("{:02}:{:04.1}", minutes as u32, t - minutes * 60.0)
}

fn place(pos: Option<[f32; 3]>) -> String {
    pos.map_or_else(
        || "-".into(),
        |p| format!("{:.1}, {:.1}, {:.1}", p[0], p[1], p[2]),
    )
}

/// Writes the items of a JSON object as a bullet list.
fn object_list(out: &mut String, v: Option<&Value>) {
    let Some(Value::Object(map)) = v else {
        out.push_str("- (not recorded)\n");
        return;
    };
    for (key, value) in map {
        let _ = writeln!(out, "- {key}: {}", value_text(value));
    }
}

/// Makes the text of `report.md`.
pub fn make_report(r: &ReportInput) -> String {
    let mut out = String::new();
    let s = r.session;
    let last_sample = r.samples.last().map_or(0.0, |x| x.t);
    let play_time = last_sample.max(r.events.last().map_or(0.0, |e| e.t));
    let _ = writeln!(out, "# Playtest report\n");
    let _ = writeln!(out, "- build: {}", value_text(&s["build"]));
    let _ = writeln!(out, "- started: {}", value_text(&s["started"]));
    let _ = writeln!(out, "- play time: {} (min:s)", clock(play_time));
    let quit = if r.quit_normally {
        "yes"
    } else {
        "no (the game stopped without a normal quit; this report was made at the next start)"
    };
    let _ = writeln!(out, "- normal quit: {quit}");

    out.push_str("\n## Hardware\n\n");
    object_list(&mut out, s.get("hardware"));
    out.push_str("\n## Settings that change speed\n\n");
    object_list(&mut out, s.get("settings"));

    speed_section(&mut out, r);
    zone_section(&mut out, r.events);
    hitch_section(&mut out, r.events);
    mark_section(&mut out, r.events);
    event_section(&mut out, r.events);
    out
}

/// Adds the results of the tests (S-113) to the end of the report.
pub fn append_tests(report: &mut String, tests: &[String]) {
    if tests.is_empty() {
        return;
    }
    report.push_str("\n## Tests\n\n");
    report.push_str(
        "The dev menu ran these tests. The frame times are not part of the speed numbers above.\n\n",
    );
    for t in tests {
        report.push_str(t.trim_end());
        report.push_str("\n\n");
    }
}

fn speed_section(out: &mut String, r: &ReportInput) {
    out.push_str("\n## Speed\n\n");
    let fallback;
    let hist = match r.histogram {
        Some(h) if h.frames > 0 => Some(h),
        _ => {
            // After a crash: the samples give the average and the worst.
            fallback = r.samples.iter().fold(None::<(f64, f64, f32)>, |acc, x| {
                let (frames, ms, worst) = acc.unwrap_or((0.0, 0.0, 0.0));
                let f = x.frames.fps as f64;
                Some((
                    frames + f,
                    ms + f * x.frames.avg_ms as f64,
                    worst.max(x.frames.max_ms),
                ))
            });
            if let Some((frames, ms, worst)) = fallback {
                if ms > 0.0 {
                    let _ = writeln!(
                        out,
                        "- fps: average {:.1}, worst frame {worst:.1} ms (from the samples; \
                         the frame-time histogram was not saved)",
                        frames * 1000.0 / ms
                    );
                }
            }
            None
        }
    };
    if let Some(h) = hist {
        let _ = writeln!(
            out,
            "- fps: average {:.1}, 1% low {:.1} (the average of the slowest 1% of frames)",
            h.average_fps(),
            h.one_percent_low_fps()
        );
        let _ = writeln!(
            out,
            "- frame time: median {:.1} ms, p99 {:.1} ms, worst {:.1} ms ({} frames)",
            h.percentile_ms(0.5),
            h.percentile_ms(0.99),
            h.worst_ms,
            h.frames
        );
    }
    if r.samples.is_empty() {
        out.push_str("- no samples\n");
        return;
    }
    let n = r.samples.len() as f32;
    let avg = |f: &dyn Fn(&Sample) -> f32| r.samples.iter().map(f).sum::<f32>() / n;
    let max = |f: &dyn Fn(&Sample) -> f32| r.samples.iter().map(f).fold(0.0f32, f32::max);
    let low_seconds = r.samples.iter().filter(|x| x.frames.fps < 55.0).count();
    let _ = writeln!(
        out,
        "- seconds below 55 fps: {low_seconds} of {}",
        r.samples.len()
    );
    let _ = writeln!(
        out,
        "- render time each frame: CPU {:.2} ms, GPU {:.2} ms (averages; GPU 0 means not measured)",
        avg(&|x| x.cpu_ms),
        avg(&|x| x.gpu_ms)
    );
    let _ = writeln!(
        out,
        "- script time each frame: process {:.2} ms, physics {:.2} ms (averages)",
        avg(&|x| x.process_ms),
        avg(&|x| x.physics_ms)
    );
    let _ = writeln!(
        out,
        "- the recorder itself: {:.3} ms each frame (average, without screenshots)",
        avg(&|x| x.recorder_ms)
    );
    let _ = writeln!(
        out,
        "- draw calls: average {:.0}, most {:.0}",
        avg(&|x| x.draw_calls as f32),
        max(&|x| x.draw_calls as f32)
    );
    let _ = writeln!(
        out,
        "- primitives: average {:.0}, most {:.0}",
        avg(&|x| x.primitives as f32),
        max(&|x| x.primitives as f32)
    );
    let _ = writeln!(
        out,
        "- video memory: most {:.0} MB; static memory: most {:.0} MB",
        max(&|x| x.video_mb),
        max(&|x| x.static_mb)
    );
}

fn zone_section(out: &mut String, events: &[Event]) {
    out.push_str("\n## Zones\n\n");
    let zones: Vec<&Event> = events.iter().filter(|e| e.kind == "zone").collect();
    if zones.is_empty() {
        out.push_str("- no zone was made\n");
    }
    for e in zones {
        let _ = writeln!(out, "- {} {}", clock(e.t), e.info);
    }
}

fn hitch_section(out: &mut String, events: &[Event]) {
    let mut hitches: Vec<&Event> = events
        .iter()
        .filter(|e| e.kind == "hitch" && !e.loading && !e.own)
        .collect();
    let loading = events
        .iter()
        .filter(|e| e.kind == "hitch" && e.loading)
        .count();
    let own = events
        .iter()
        .filter(|e| e.kind == "hitch" && e.own && !e.loading)
        .count();
    let _ = writeln!(
        out,
        "\n## The 10 worst hitches\n\n{} hitches in play. Not in this list: {loading} while a \
         zone loaded, {own} after a recorder screenshot.\n",
        hitches.len()
    );
    hitches.sort_by(|a, b| b.value.unwrap_or(0.0).total_cmp(&a.value.unwrap_or(0.0)));
    if hitches.is_empty() {
        return;
    }
    out.push_str("| time | frame ms | place | info | shot |\n|---|---|---|---|---|\n");
    for e in hitches.into_iter().take(10) {
        let _ = writeln!(
            out,
            "| {} | {:.0} | {} | {} | {} |",
            clock(e.t),
            e.value.unwrap_or(0.0),
            place(e.pos),
            e.info,
            e.shot.as_deref().unwrap_or("-")
        );
    }
}

fn mark_section(out: &mut String, events: &[Event]) {
    out.push_str("\n## Marks\n\n");
    let marks: Vec<&Event> = events.iter().filter(|e| e.kind == "mark").collect();
    if marks.is_empty() {
        out.push_str("- no marks\n");
        return;
    }
    for e in marks {
        let n = e.value.unwrap_or(0.0) as u32;
        let _ = writeln!(
            out,
            "- mark {n} at {}, place {}, shot {}, frames `marks/mark-{n:02}.jsonl`. {}",
            clock(e.t),
            place(e.pos),
            e.shot.as_deref().unwrap_or("-"),
            e.info
        );
    }
}

fn event_section(out: &mut String, events: &[Event]) {
    out.push_str("\n## All events\n\n");
    let mut counts: Vec<(&str, usize)> = Vec::new();
    for e in events {
        match counts.iter_mut().find(|(k, _)| *k == e.kind) {
            Some((_, n)) => *n += 1,
            None => counts.push((&e.kind, 1)),
        }
    }
    let list: Vec<String> = counts.iter().map(|(k, n)| format!("{k} {n}")).collect();
    let _ = writeln!(out, "Counts: {}.\n", list.join(", "));
    if events.is_empty() {
        return;
    }
    out.push_str("| time | kind | info | place | shot |\n|---|---|---|---|---|\n");
    for e in events.iter().take(MAX_EVENT_ROWS) {
        let loading = match (e.loading, e.own) {
            (true, _) => " (loading)",
            (_, true) => " (recorder)",
            _ => "",
        };
        let _ = writeln!(
            out,
            "| {} | {} | {}{loading} | {} | {} |",
            clock(e.t),
            e.kind,
            e.info.replace('|', "/"),
            place(e.pos),
            e.shot.as_deref().unwrap_or("-")
        );
    }
    if events.len() > MAX_EVENT_ROWS {
        let _ = writeln!(
            out,
            "\n{} more events are in `events.jsonl`.",
            events.len() - MAX_EVENT_ROWS
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::playtest::stats::FrameSummary;
    use serde_json::json;

    fn event(t: f64, kind: &str, value: Option<f64>, loading: bool) -> Event {
        Event {
            t,
            kind: kind.into(),
            info: format!("{kind} info"),
            value,
            pos: Some([1.0, 2.0, 3.0]),
            shot: None,
            loading,
            own: false,
        }
    }

    #[test]
    fn report_has_the_main_parts() {
        let session = json!({
            "build": "abc1234",
            "started": "2026-11-14T15:30:02",
            "hardware": {"cpu": "Test CPU", "gpu": "Test GPU"},
            "settings": {"renderer": "mobile"},
        });
        let samples: Vec<Sample> = (1..=3)
            .map(|i| Sample {
                t: i as f64,
                frames: FrameSummary {
                    fps: if i == 2 { 40.0 } else { 60.0 },
                    avg_ms: 16.6,
                    p95_ms: 17.0,
                    max_ms: 20.0,
                },
                draw_calls: 100 * i,
                ..Default::default()
            })
            .collect();
        let mut events = vec![
            event(0.5, "hitch", Some(400.0), true),
            event(1.0, "zone", None, false),
            event(2.0, "mark", Some(1.0), false),
        ];
        for i in 0..12 {
            events.push(event(2.0 + i as f64, "hitch", Some(60.0 + i as f64), false));
        }
        let mut hist = FrameHistogram::default();
        for _ in 0..100 {
            hist.add(16.6);
        }
        let text = make_report(&ReportInput {
            session: &session,
            samples: &samples,
            events: &events,
            histogram: Some(&hist),
            quit_normally: true,
        });
        assert!(text.contains("build: abc1234"), "{text}");
        assert!(text.contains("cpu: Test CPU"));
        assert!(text.contains("1% low 60.1"), "{text}");
        assert!(text.contains("seconds below 55 fps: 1 of 3"));
        assert!(
            text.contains("12 hitches in play. Not in this list: 1 while a zone loaded, 0 after")
        );
        // The worst hitch is first, and only 10 show.
        let table = text.split("## The 10 worst hitches").nth(1).unwrap();
        let table = table.split("## Marks").next().unwrap();
        assert_eq!(table.matches("| hitch info |").count(), 10);
        assert!(table.find("| 71 |").unwrap() < table.find("| 70 |").unwrap());
        assert!(text.contains("mark 1 at 00:02.0"));
        assert!(text.contains("marks/mark-01.jsonl"));
        assert!(text.contains("Counts: hitch 13, zone 1, mark 1."));
    }

    #[test]
    fn tests_are_added_at_the_end() {
        let mut report = String::from("# Playtest report\n");
        append_tests(&mut report, &[]);
        assert_eq!(report, "# Playtest report\n");
        append_tests(&mut report, &["### Test: A\n".into(), "### Test: B".into()]);
        assert!(report.contains("## Tests"));
        assert!(report.find("### Test: A").unwrap() < report.find("### Test: B").unwrap());
    }

    #[test]
    fn report_after_a_crash() {
        let session = json!({});
        let samples = vec![Sample {
            t: 1.0,
            frames: FrameSummary {
                fps: 50.0,
                avg_ms: 20.0,
                p95_ms: 25.0,
                max_ms: 70.0,
            },
            ..Default::default()
        }];
        let text = make_report(&ReportInput {
            session: &session,
            samples: &samples,
            events: &[],
            histogram: None,
            quit_normally: false,
        });
        assert!(text.contains("normal quit: no"));
        assert!(text.contains("average 50.0, worst frame 70.0 ms"), "{text}");
        assert!(text.contains("(not recorded)"));
    }

    #[test]
    fn clock_text() {
        assert_eq!(clock(0.0), "00:00.0");
        assert_eq!(clock(75.25), "01:15.2");
    }
}
