//! Driftloom's audio core inside Godot (decision 0016).
//!
//! One class, `DriftloomPlayer`: load a note list (Driftloom's `.dlnotes`
//! format), then pull audio from it a few frames at a time into an
//! `AudioStreamGenerator`. `golden_line` renders a whole list at once and
//! returns the same line `dlrender` prints, so a build can be checked
//! against Driftloom's golden hashes: same notes, same samples, inside the
//! game as in the app.
//!
//! Today the core is Driftloom's synthesizer; the part that writes the
//! notes is still JavaScript in the app (Driftloom's phase 2 moves it to
//! Rust). Until then the game plays note lists.

#[allow(dead_code)]
mod notelist;
// Not in the browser: its engine has no threads (see src/stream.rs).
#[cfg(not(target_os = "emscripten"))]
mod stream;

use std::sync::Mutex;
use std::time::Instant;

use dlcore::{Core, QUANTUM};
use godot::prelude::*;
use notelist::List;

struct DriftloomExtension;

#[gdextension]
unsafe impl ExtensionLibrary for DriftloomExtension {}

/// The core is megabytes of state, built at compile time and never on the
/// stack. One core for the whole game: its six channels are the music's
/// layers. This one is `DriftloomPlayer`'s (the browser, and the tests);
/// `DriftloomStream` has its own two (src/stream.rs).
static CORE: Mutex<Core> = Mutex::new(Core::new());

/// Plays one note list through Driftloom's core.
#[derive(GodotClass)]
#[class(init, base = RefCounted)]
pub struct DriftloomPlayer {
    list: Option<List>,
    /// The next frame the core will render.
    frame: u64,
    /// Rendered samples not yet handed to Godot, and how many are used.
    pending: Vec<f32>,
    used: usize,
    /// Start again from the top when the list ends.
    #[var]
    looping: bool,
    /// Loudness applied on the way out (the core's own level is untouched).
    #[var]
    gain: f32,
    /// Driftloom's lite mix for weak devices: half the reverb's work, the
    /// same notes. Takes effect from the next list loaded.
    #[var]
    lite: bool,
    base: Base<RefCounted>,
}

#[godot_api]
impl DriftloomPlayer {
    /// Load a note list and get ready to play it from the top. Returns ""
    /// on success, or what went wrong.
    #[func]
    fn load_notes(&mut self, text: GString) -> GString {
        let list = match notelist::parse(&text.to_string()) {
            Ok(list) => list,
            Err(e) => return GString::from(e.as_str()),
        };
        if let Err(e) = self.restart(&list) {
            return GString::from(e.as_str());
        }
        self.list = Some(list);
        if self.gain == 0.0 {
            self.gain = 1.0;
        }
        GString::new()
    }

    /// True once a list that isn't looping has played to its end.
    #[func]
    fn is_finished(&self) -> bool {
        self.list.as_ref().is_none_or(|l| self.frame >= l.frames && self.used >= self.pending.len())
    }

    /// The list's sample rate, Hz (0 before a list is loaded).
    #[func]
    fn sample_rate(&self) -> f64 {
        self.list.as_ref().map_or(0.0, |l| l.rate)
    }

    /// How long the list is, in seconds.
    #[func]
    fn length_seconds(&self) -> f64 {
        self.list.as_ref().map_or(0.0, |l| l.frames as f64 / l.rate)
    }

    /// The next `frames` frames, as stereo pairs for
    /// `AudioStreamGeneratorPlayback.push_buffer` (the core is mono today,
    /// so both sides are the same).
    #[func]
    fn render_stereo(&mut self, frames: i64) -> PackedVector2Array {
        let mut out = PackedVector2Array::new();
        let Some(list) = self.list.take() else {
            return out;
        };
        let wanted = usize::try_from(frames.max(0)).unwrap_or(0);
        out.resize(wanted);
        {
            let slice = out.as_mut_slice();
            let mut core = CORE.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            for frame in slice.iter_mut() {
                if self.used >= self.pending.len() {
                    if self.frame >= list.frames {
                        if !self.looping {
                            *frame = Vector2::ZERO;
                            continue;
                        }
                        if notelist::load(&mut core, &list).is_err() {
                            *frame = Vector2::ZERO;
                            continue;
                        }
                        if self.lite {
                            core.quality(false);
                        }
                        self.frame = 0;
                    }
                    core.process(self.frame);
                    self.pending.clear();
                    self.pending.extend_from_slice(&core.output()[0]);
                    self.used = 0;
                    self.frame += QUANTUM as u64;
                }
                let s = self.pending[self.used] * self.gain;
                self.used += 1;
                *frame = Vector2::new(s, s);
            }
        }
        self.list = Some(list);
        out
    }

    /// Render a whole note list at once and return the line Driftloom's
    /// `dlrender` prints for it (frame count and FNV-1a hashes of every
    /// signal), plus how long it took. Compare with
    /// `core/golden/*.hashes` in the Driftloom repo.
    #[func]
    fn golden_line(&mut self, text: GString) -> GString {
        let list = match notelist::parse(&text.to_string()) {
            Ok(list) => list,
            Err(e) => return GString::from(format!("error: {e}").as_str()),
        };
        let started = Instant::now();
        let mut core = CORE.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let rendered = notelist::play(&mut core, &list, None);
        let ms = started.elapsed().as_secs_f64() * 1000.0;
        drop(core);
        // The core was reused: start whatever was loaded again from the top.
        if let Some(loaded) = self.list.take() {
            let _ = self.restart(&loaded);
            self.list = Some(loaded);
        }
        match rendered {
            Ok(r) => GString::from(format!("{} ms={ms:.0}", notelist::line(&list, &r.hashes)).as_str()),
            Err(e) => GString::from(format!("error: {e}").as_str()),
        }
    }

    /// What this extension is built from.
    #[func]
    fn version(&self) -> GString {
        GString::from(concat!("driftloom-godot ", env!("CARGO_PKG_VERSION"), " (dlcore f654fdb)"))
    }
}

impl DriftloomPlayer {
    fn restart(&mut self, list: &List) -> Result<(), String> {
        let mut core = CORE.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        notelist::load(&mut core, list)?;
        if self.lite {
            core.quality(false);
        }
        self.frame = 0;
        self.pending.clear();
        self.used = 0;
        Ok(())
    }
}
