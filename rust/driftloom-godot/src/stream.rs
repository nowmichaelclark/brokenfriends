//! Music on Godot's audio thread (phones and computers).
//!
//! `DriftloomStream` is an `AudioStream`: put it in an `AudioStreamPlayer`
//! and Godot asks it for audio from its own mixing thread, so the music
//! never waits on the screen and the screen never waits on the music
//! (Driftloom's first rule). The game queues note lists from the main
//! thread; the audio thread takes the next one when the current one ends,
//! with no gap.
//!
//! The browser keeps `DriftloomPlayer` and Godot's `AudioStreamGenerator`:
//! its engine has no threads, and the generator's half-second buffer rides
//! out slow frames there.
//!
//! While playing, the audio thread allocates nothing and never waits on
//! the main thread. Starting a list means resetting the core, which takes
//! a desktop about 15 ms and a cheap phone several times that: too long
//! for the audio thread. So there are two cores. A helper thread readies
//! the next list on the one not playing, and at the hand-over the audio
//! thread only switches which core it reads (`try_lock`: if the helper
//! happens to hold the lock that instant, the switch waits a block).

use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Instant;

use dlcore::{Core, QUANTUM};
use godot::classes::native::AudioFrame;
use godot::classes::{AudioStream, AudioStreamPlayback, AudioStreamPlaybackResampled, IAudioStream, IAudioStreamPlaybackResampled};
use godot_core::meta::RawPtr;
use godot::prelude::*;

use crate::notelist::{self, List};

/// The stream's two cores (13 MB each): one playing, one being readied.
/// Separate from `DriftloomPlayer`'s, so the tests can use that one while
/// the music plays.
static CORES: [Mutex<Core>; 2] = [Mutex::new(Core::new()), Mutex::new(Core::new())];

/// A list loaded into a core, ready to play.
struct Ready {
    list: List,
    gain: f32,
    core: usize,
}

/// What the game and the audio thread share.
struct Shared {
    /// The next list, loaded and waiting for the current one to end.
    next: Mutex<Option<Ready>>,
    /// Lists queued and not yet ready (the helper thread is loading them).
    loading: AtomicUsize,
    /// The core the audio thread is playing.
    active: AtomicUsize,
    /// The lite mix (Driftloom's, for weak devices).
    lite: AtomicBool,
    /// The sample rate of the lists, Hz, as f32 bits.
    rate: AtomicU32,
    /// Nothing is playing and nothing is queued.
    idle: AtomicBool,
    /// How long rendering took and how many frames it made, for `cost_ms`.
    busy_ns: AtomicU64,
    made: AtomicU64,
}

/// Driftloom's music as an audio stream. Queue note lists with
/// `queue_notes`; they play one after another.
#[derive(GodotClass)]
#[class(init, base = AudioStream)]
pub struct DriftloomStream {
    #[init(val = Arc::new(Shared {
        next: Mutex::new(None),
        loading: AtomicUsize::new(0),
        active: AtomicUsize::new(0),
        lite: AtomicBool::new(false),
        rate: AtomicU32::new(48_000f32.to_bits()),
        idle: AtomicBool::new(true),
        busy_ns: AtomicU64::new(0),
        made: AtomicU64::new(0),
    }))]
    shared: Arc<Shared>,
    base: Base<AudioStream>,
}

#[godot_api]
impl DriftloomStream {
    /// Queue a note list to play when the current one ends (at once if
    /// nothing is playing), `gain` louder. Returns "" or what went wrong.
    /// A list already waiting is replaced. The list is readied on a helper
    /// thread, so this returns at once.
    #[func]
    fn queue_notes(&mut self, text: GString, gain: f32) -> GString {
        let list = match notelist::parse(&text.to_string()) {
            Ok(list) => list,
            Err(e) => return GString::from(e.as_str()),
        };
        self.shared.loading.fetch_add(1, Ordering::Relaxed);
        self.shared.idle.store(false, Ordering::Relaxed);
        let shared = Arc::clone(&self.shared);
        std::thread::spawn(move || shared.ready(list, gain));
        GString::new()
    }

    /// True while a list is waiting its turn (or being readied).
    #[func]
    fn has_queued(&self) -> bool {
        self.shared.loading.load(Ordering::Relaxed) > 0 || self.shared.next.lock().unwrap_or_else(PoisonError::into_inner).is_some()
    }

    /// True when the music has run out: nothing playing, nothing queued.
    #[func]
    fn is_idle(&self) -> bool {
        self.shared.idle.load(Ordering::Relaxed)
    }

    /// Driftloom's lite mix (half the reverb's work). From the next list.
    #[func]
    fn set_lite(&mut self, lite: bool) {
        self.shared.lite.store(lite, Ordering::Relaxed);
    }

    /// The audio thread's work for one sixtieth of a second of music, ms:
    /// what the music would cost a 60 fps frame if it ran on the screen's
    /// thread.
    #[func]
    fn cost_ms(&self) -> f64 {
        let made = self.shared.made.load(Ordering::Relaxed);
        if made == 0 {
            return 0.0;
        }
        let rate = f64::from(f32::from_bits(self.shared.rate.load(Ordering::Relaxed)));
        #[allow(clippy::cast_precision_loss)]
        let per_frame_ns = self.shared.busy_ns.load(Ordering::Relaxed) as f64 / made as f64;
        per_frame_ns * rate / 60.0 / 1e6
    }
}

impl Shared {
    /// Load a list into the core that isn't playing and leave it in `next`.
    /// Holding `next` the whole time keeps the audio thread from switching
    /// cores meanwhile.
    fn ready(&self, list: List, gain: f32) {
        let mut next = self.next.lock().unwrap_or_else(PoisonError::into_inner);
        let spare = 1 - self.active.load(Ordering::Acquire);
        let mut core = CORES[spare].lock().unwrap_or_else(PoisonError::into_inner);
        if notelist::load(&mut core, &list).is_ok() {
            if self.lite.load(Ordering::Relaxed) {
                core.quality(false);
            }
            *next = Some(Ready { list, gain, core: spare });
        }
        drop(core);
        drop(next);
        self.loading.fetch_sub(1, Ordering::Relaxed);
    }
}

#[godot_api]
impl IAudioStream for DriftloomStream {
    fn instantiate_playback(&self) -> Option<Gd<AudioStreamPlayback>> {
        let playback = Gd::from_init_fn(|base| DriftloomPlayback {
            shared: Arc::clone(&self.shared),
            list: None,
            gain: 1.0,
            frame: 0,
            pending: Vec::with_capacity(QUANTUM),
            used: 0,
            active: false,
            base,
        });
        Some(playback.upcast())
    }

    fn get_stream_name(&self) -> GString {
        GString::from("Driftloom")
    }
}

/// One playing `DriftloomStream`. Godot calls it from the audio thread.
#[derive(GodotClass)]
#[class(no_init, base = AudioStreamPlaybackResampled)]
pub struct DriftloomPlayback {
    shared: Arc<Shared>,
    list: Option<List>,
    gain: f32,
    frame: u64,
    pending: Vec<f32>,
    used: usize,
    active: bool,
    base: Base<AudioStreamPlaybackResampled>,
}

impl DriftloomPlayback {
    /// Switch to the next list's core, if one is ready and the helper
    /// isn't holding it this instant.
    fn take_next(&mut self) -> bool {
        let Ok(mut next) = self.shared.next.try_lock() else {
            return false;
        };
        let Some(ready) = next.take() else {
            return false;
        };
        self.shared.active.store(ready.core, Ordering::Release);
        drop(next);
        #[allow(clippy::cast_possible_truncation)]
        self.shared.rate.store((ready.list.rate as f32).to_bits(), Ordering::Relaxed);
        // The old list is dropped here, on the audio thread: freeing is
        // cheap next to the render, and it happens once a loop.
        self.list = Some(ready.list);
        self.gain = ready.gain;
        self.frame = 0;
        self.pending.clear();
        self.used = 0;
        true
    }
}

#[godot_api]
impl IAudioStreamPlaybackResampled for DriftloomPlayback {
    unsafe fn mix_resampled_rawptr(&mut self, dst_buffer: RawPtr<*mut AudioFrame>, frame_count: i32) -> i32 {
        let count = usize::try_from(frame_count).unwrap_or(0);
        // SAFETY: Godot hands a buffer of `frame_count` frames to fill.
        let out = unsafe { std::slice::from_raw_parts_mut(dst_buffer.ptr(), count) };
        let started = Instant::now();
        for frame in out.iter_mut() {
            if self.used >= self.pending.len() {
                let ended = self.list.as_ref().is_none_or(|l| self.frame >= l.frames);
                if ended && !self.take_next() {
                    // Nothing more to play: silence until something is queued.
                    if self.shared.loading.load(Ordering::Relaxed) == 0 {
                        self.shared.idle.store(true, Ordering::Relaxed);
                    }
                    frame.left = 0.0;
                    frame.right = 0.0;
                    continue;
                }
                // Only this thread locks the playing core (the helper
                // readies the other), so this never waits.
                let mut core = CORES[self.shared.active.load(Ordering::Acquire)].lock().unwrap_or_else(PoisonError::into_inner);
                core.process(self.frame);
                self.pending.clear();
                self.pending.extend_from_slice(&core.output()[0]);
                self.used = 0;
                drop(core);
                self.frame += QUANTUM as u64;
            }
            let s = self.pending[self.used] * self.gain;
            self.used += 1;
            frame.left = s;
            frame.right = s;
        }
        #[allow(clippy::cast_possible_truncation)]
        self.shared.busy_ns.fetch_add(started.elapsed().as_nanos() as u64, Ordering::Relaxed);
        self.shared.made.fetch_add(count as u64, Ordering::Relaxed);
        frame_count
    }

    fn get_stream_sampling_rate(&self) -> f32 {
        f32::from_bits(self.shared.rate.load(Ordering::Relaxed))
    }

    fn start(&mut self, _from_pos: f64) {
        self.active = true;
        self.base_mut().begin_resample();
    }

    fn stop(&mut self) {
        self.active = false;
    }

    fn is_playing(&self) -> bool {
        self.active
    }

    fn get_playback_position(&self) -> f64 {
        #[allow(clippy::cast_precision_loss)]
        let frames = self.frame as f64;
        frames / f64::from(f32::from_bits(self.shared.rate.load(Ordering::Relaxed)))
    }
}

