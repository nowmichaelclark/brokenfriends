//! The playtest recorder: the parts without Godot (R22,
//! `plan/diagnostics.md`).
//!
//! The Godot node is `crate::recorder::BfRecorder`. It collects the
//! data. This module has the data types, the statistics, the event
//! detectors and the report. All of it has unit tests.

pub mod detect;
pub mod report;
pub mod stats;

use serde::{Deserialize, Serialize};
use stats::FrameSummary;

/// The state of the bear in one sample.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct BearSample {
    /// The feet of the bear in world space, in metres.
    pub pos: [f32; 3],
    /// The speed in metres per second.
    pub speed: f32,
    /// idle, walk, jog, jump, fall, climb, slide, mantle.
    pub state: String,
    /// The slope below the bear (or of the wall), in degrees.
    pub slope_deg: f32,
    /// The main ground type below the bear.
    pub ground: String,
    /// The grip of the wall (on a wall) or of the ground, 0 to 1.
    pub grip: f32,
    /// The stamina, 0 to 1.
    pub stamina: f32,
    /// The arms and legs that the bear has.
    pub limbs: u32,
    /// The time of one animation step of the bear, in microseconds.
    #[serde(default)]
    pub anim_us: f32,
    /// The gait from the limbs: walk, hop, arm_crawl, one_arm_drag or
    /// wriggle.
    #[serde(default)]
    pub gait: String,
    /// How far the bear is in the smoke at the zone edge, in metres (0: not in it).
    #[serde(default)]
    pub smoke_m: f32,
    /// The seconds in the smoke that count now. At the limit, the bear dies.
    #[serde(default)]
    pub smoke_s: f32,
}

/// The camera in one sample.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CameraSample {
    pub pos: [f32; 3],
    /// The direction that the camera looks, length 1.
    pub dir: [f32; 3],
    /// The name of the active camera node.
    pub name: String,
}

/// The input in one sample.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct InputSample {
    /// The move stick or keys: x right, y down (Godot axes).
    #[serde(rename = "move")]
    pub move_stick: [f32; 2],
    /// The look stick.
    pub look: [f32; 2],
    /// The names of the buttons and keys that are held.
    pub held: Vec<String>,
}

/// One line of `samples.jsonl`: one second of play.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Sample {
    /// The time since the start of the session, in seconds.
    pub t: f64,
    pub frames: FrameSummary,
    /// The CPU time of the renderer for one frame, in milliseconds.
    pub cpu_ms: f32,
    /// The GPU time for one frame, in milliseconds (0 if not measured).
    pub gpu_ms: f32,
    /// The time of `_process` and `_physics_process`, in milliseconds.
    pub process_ms: f32,
    pub physics_ms: f32,
    pub draw_calls: u32,
    pub primitives: u32,
    pub objects: u32,
    pub video_mb: f32,
    pub static_mb: f32,
    /// The sum of Godot's pipeline compilation monitors (shader
    /// compiles). A rise near a hitch points to a shader compile.
    #[serde(default)]
    pub pipeline_compiles: u32,
    /// The time that the recorder itself used in each frame (average
    /// over the second, without screenshots), in milliseconds.
    #[serde(default)]
    pub recorder_ms: f32,
    pub bear: Option<BearSample>,
    pub camera: Option<CameraSample>,
    pub input: InputSample,
}

/// One line of `events.jsonl`.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Event {
    /// The time since the start of the session, in seconds.
    pub t: f64,
    /// hitch, low_fps, low_fps_end, stuck, climb_loop, through_ground,
    /// mark, bear (a state change), zone, error, warning, recorder.
    pub kind: String,
    /// A short text for people.
    pub info: String,
    /// A number, for example the frame time of a hitch in ms.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<f64>,
    /// The feet of the bear at the event, if there is a bear.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pos: Option<[f32; 3]>,
    /// The file name of a screenshot in `shots/`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shot: Option<String>,
    /// The event came while a zone was made or loaded.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub loading: bool,
    /// The recorder caused the event (for example the frame after a
    /// screenshot).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub own: bool,
}

/// One frame in the 10 s buffer that a mark saves.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct FrameRecord {
    pub t: f64,
    pub ms: f32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bear: Option<[f32; 3]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub velocity: Option<[f32; 3]>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub state: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub camera: Option<[f32; 3]>,
    #[serde(rename = "move")]
    pub move_stick: [f32; 2],
    pub look: [f32; 2],
}

/// Reads a file of JSON lines. Lines that do not parse (for example a
/// half line after a crash) are skipped.
pub fn parse_lines<T: for<'de> Deserialize<'de>>(text: &str) -> Vec<T> {
    text.lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect()
}

/// Finds the errors and warnings in new text of the Godot log. Gives
/// (kind, message) pairs: kind is "error" or "warning". Only full lines
/// count; give the rest again with the next text.
pub fn log_problems(text: &str) -> Vec<(&'static str, String)> {
    let mut out = Vec::new();
    for line in text.lines() {
        let line = line.trim_end();
        let kind = if line.starts_with("ERROR:")
            || line.starts_with("SCRIPT ERROR:")
            || line.starts_with("USER ERROR:")
            || line.starts_with("SHADER ERROR:")
        {
            "error"
        } else if line.starts_with("WARNING:")
            || line.starts_with("USER WARNING:")
            || line.starts_with("SCRIPT WARNING:")
        {
            "warning"
        } else {
            continue;
        };
        out.push((kind, line.to_string()));
    }
    out
}

/// Gives the name of the session folder and of the zip for a start time.
/// `datetime` is in the form "2026-11-14T15:30:02".
pub fn session_name(datetime: &str) -> String {
    let clean: String = datetime
        .chars()
        .map(|c| match c {
            'T' | ' ' => '-',
            ':' => '-',
            c if c.is_ascii_alphanumeric() || c == '-' => c,
            _ => '_',
        })
        .collect();
    format!("playtest-{clean}")
}

/// Removes personal paths from a text (for example the Godot log). Each
/// `(from, to)` pair is replaced; long paths first.
pub fn scrub(text: &str, replace: &[(String, String)]) -> String {
    let mut pairs: Vec<&(String, String)> = replace.iter().filter(|(f, _)| f.len() > 1).collect();
    pairs.sort_by_key(|(f, _)| std::cmp::Reverse(f.len()));
    let mut out = text.to_string();
    for (from, to) in pairs {
        out = out.replace(from.as_str(), to);
    }
    out
}

/// Chooses which old sessions to delete so the folder holds at most
/// `limit` bytes. `files` are (name, bytes), sorted from old to new.
/// The newest file always stays.
pub fn files_to_delete(files: &[(String, u64)], limit: u64) -> Vec<String> {
    let mut total: u64 = files.iter().map(|(_, b)| b).sum();
    let mut out = Vec::new();
    for (name, bytes) in files.iter().take(files.len().saturating_sub(1)) {
        if total <= limit {
            break;
        }
        total -= bytes;
        out.push(name.clone());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn log_lines() {
        let text = "Godot Engine v4.7.1\nERROR: Bad thing.\n   at: foo (bar.cpp:1)\n\
                    WARNING: Odd thing.\nprint line\nUSER ERROR: Rust said no.\n";
        let found = log_problems(text);
        assert_eq!(found.len(), 3);
        assert_eq!(found[0], ("error", "ERROR: Bad thing.".to_string()));
        assert_eq!(found[1].0, "warning");
        assert_eq!(found[2].0, "error");
    }

    #[test]
    fn names() {
        assert_eq!(
            session_name("2026-11-14T15:30:02"),
            "playtest-2026-11-14-15-30-02"
        );
    }

    #[test]
    fn scrub_paths() {
        let text = "ERROR: /home/mikey/Games/bf/x.png and /home/mikey/.local/share/godot";
        let pairs = vec![
            ("/home/mikey/Games/bf".to_string(), "<game>".to_string()),
            ("/home/mikey".to_string(), "~".to_string()),
        ];
        assert_eq!(
            scrub(text, &pairs),
            "ERROR: <game>/x.png and ~/.local/share/godot"
        );
        // An empty path does not change the text.
        assert_eq!(scrub("abc", &[(String::new(), "x".into())]), "abc");
    }

    #[test]
    fn cleanup_keeps_the_newest() {
        let files = vec![
            ("a.zip".to_string(), 100),
            ("b.zip".to_string(), 100),
            ("c.zip".to_string(), 100),
        ];
        assert_eq!(files_to_delete(&files, 250), vec!["a.zip".to_string()]);
        assert!(files_to_delete(&files, 300).is_empty());
        // The newest stays, also if it is too large.
        assert_eq!(files_to_delete(&files, 10).len(), 2);
    }

    #[test]
    fn lines_survive_a_crash() {
        let e = Event {
            t: 1.5,
            kind: "hitch".into(),
            info: "frame 80 ms".into(),
            value: Some(80.0),
            ..Default::default()
        };
        let mut text = serde_json::to_string(&e).unwrap();
        text.push('\n');
        text.push_str("{\"t\": 2.0, \"kind\""); // a half line
        let back: Vec<Event> = parse_lines(&text);
        assert_eq!(back, vec![e]);
    }
}
