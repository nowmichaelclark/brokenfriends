// Copied from Driftloom's core/examples/dlrender/notelist.rs at commit
// f654fdb767b77bbd5845dafe8d42d2e3a0ba8647 (MIT, Mikey). Only change: `load`
// and `play` are public, so the Godot host can use them.

//! Note lists: everything the core needs to render a loop with no
//! JavaScript, and a host that renders one (queue item 42).
//!
//! A note list is plain ASCII text, one fact per line, because it is
//! committed beside the hashes it produces and a diff of it should be
//! readable. The page's host (`js/core.js`) makes the same few calls to the
//! core whatever it plays -- `param`, `damp`, `key`, `set`, `event` and
//! `note` -- so a note
//! list is those calls, in the order the synth made them, and nothing else. `tools/golden-export.mjs` writes
//! them; this file reads them. Every number is written the way JavaScript
//! prints a double (the shortest text that reads back exactly) and read
//! back exactly, so a list is bit for bit what the host sent.
//!
//! ```text
//! dlnotes 2                    the format and its version, first line
//! # ...                        a comment; so are blank lines
//! name tide-1                  what the list is called
//! rate 48000                   the sample rate, Hz
//! quality full                 the mix: full (six combs) or lite (three)
//! frames 1536000               how many frames to render and hash
//! param ID OP VALUE TIME TAU   Core::param
//! damp CHANNEL TIME            Core::damp
//! key SEED PASS STEP ORD       Core::key: where the notes that follow sit
//! set CHANNEL BLOCK SETTING VALUE TIME
//!                              Core::set: a setting of a channel's own block
//! event CHANNEL KIND TIME VALUE
//!                              Core::event (`Event::new`)
//! route CHANNEL BLOCK SETTING FROM BUS TIME
//!                              Core::route: FROM's signal (BUS one of its
//!                              own blocks' buses, 4294967295 the whole
//!                              channel; FROM 4294967295 undoes the route)
//! retune CHANNEL TIME N1 ... N6
//!                              Core::event (`Event::retune`); "nan" past the
//!                              chord's last note
//! note VOICE CHANNEL TIME MIDI DUR VEL PARTS X0 ... X11
//!                              Core::note; "nan" for a value left out
//! ```
//!
//! Events are handed to the core in file order before the first block is
//! rendered, as the measure harness hands over a whole render. A note's
//! `TIME` is on the render's clock, in seconds.

// The casts here are of values the code has just bounded: a rate checked to
// lie between 8,000 and 384,000 Hz, a block length of at most `QUANTUM`, a
// WAV's length. Spelling each as a checked conversion would bury the format
// in error handling for numbers that cannot be out of range.
#![allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]

use std::fmt::Write as _;

use dlcore::{CHANNELS, Core, EXTRA, Key, QUANTUM, Taken};

/// One call the host made to the core.
#[derive(Clone)]
pub enum Event {
    /// `Core::param(id, op, value, time, tau)`.
    Param { id: u32, op: u32, value: f64, time: f64, tau: f64 },
    /// `Core::damp(channel, time)`.
    Damp { channel: u32, time: f64 },
    /// `Core::key(key)`: where the notes after it sit in their loop.
    Key(Key),
    /// `Core::set(channel, block, setting, value, time)`.
    Set { channel: u32, block: u32, setting: u32, value: f64, time: f64 },
    /// `Core::route(channel, block, setting, from, time)`, with `from` as
    /// `dl_route` takes it (`u32::MAX` for none, and for the whole channel).
    Route { channel: u32, block: u32, setting: u32, from: u32, bus: u32, time: f64 },
    /// `Core::event(channel, Event::new(kind, time, value))`.
    Channel { channel: u32, kind: u32, time: f64, value: f64 },
    /// `Core::event(channel, Event::retune(time, notes))`.
    Retune { channel: u32, time: f64, notes: [f64; 6] },
    /// `Core::note(...)`.
    Note {
        voice: u32,
        channel: u32,
        time: f64,
        midi: f64,
        dur: f64,
        vel: f64,
        parts: u32,
        extra: [f64; EXTRA],
    },
}

/// A loop to render: the header and the calls, in order.
pub struct List {
    pub name: String,
    pub rate: f64,
    pub full: bool,
    pub frames: u64,
    pub events: Vec<Event>,
}

/// The signals a render hashes, in the order the hash line gives them: what
/// the speakers get, then each channel as it went into the mix, then what
/// reached the bus compressor. The channels' order is the core's.
pub const SIGNALS: [&str; 8] = ["out", "melody", "chords", "bass", "texture", "drums", "wash", "bus"];

fn number(word: &str, what: &str, line: usize) -> Result<f64, String> {
    word.parse::<f64>()
        .map_err(|_| format!("line {line}: {what} is not a number: {word}"))
}

fn whole(word: &str, what: &str, line: usize) -> Result<u32, String> {
    word.parse::<u32>()
        .map_err(|_| format!("line {line}: {what} is not a whole number: {word}"))
}

/// Read a note list. The error says which line is wrong, in plain ASCII.
// One arm to a kind of line, so the function reads as the format does;
// splitting it would scatter the format over helpers.
#[allow(clippy::too_many_lines)]
pub fn parse(text: &str) -> Result<List, String> {
    let mut list = List {
        name: String::new(),
        rate: 0.0,
        full: true,
        frames: 0,
        events: Vec::new(),
    };
    let mut seen_header = false;
    for (n, raw) in text.lines().enumerate() {
        let line = n + 1;
        let raw = raw.trim();
        if raw.is_empty() || raw.starts_with('#') {
            continue;
        }
        let words: Vec<&str> = raw.split_whitespace().collect();
        let (kind, args) = (words[0], &words[1..]);
        if !seen_header {
            if kind != "dlnotes" || args != ["2"] {
                return Err(format!("line {line}: a note list starts with: dlnotes 2"));
            }
            seen_header = true;
            continue;
        }
        let want = |count: usize| -> Result<(), String> {
            if args.len() == count {
                Ok(())
            } else {
                Err(format!("line {line}: {kind} takes {count} values, not {}", args.len()))
            }
        };
        match kind {
            "name" => {
                want(1)?;
                list.name = args[0].to_string();
            }
            "rate" => {
                want(1)?;
                list.rate = number(args[0], "the rate", line)?;
            }
            "quality" => {
                want(1)?;
                list.full = match args[0] {
                    "full" => true,
                    "lite" => false,
                    other => return Err(format!("line {line}: quality is full or lite, not {other}")),
                };
            }
            "frames" => {
                want(1)?;
                list.frames = u64::from(whole(args[0], "the frame count", line)?);
            }
            "param" => {
                want(5)?;
                list.events.push(Event::Param {
                    id: whole(args[0], "the parameter", line)?,
                    op: whole(args[1], "the call", line)?,
                    value: number(args[2], "the value", line)?,
                    time: number(args[3], "the time", line)?,
                    tau: number(args[4], "the time constant", line)?,
                });
            }
            "damp" => {
                want(2)?;
                list.events.push(Event::Damp {
                    channel: whole(args[0], "the channel", line)?,
                    time: number(args[1], "the time", line)?,
                });
            }
            "key" => {
                want(4)?;
                list.events.push(Event::Key(Key {
                    seed: whole(args[0], "the seed", line)?,
                    pass: whole(args[1], "the pass", line)?,
                    step: whole(args[2], "the step", line)?,
                    ord: whole(args[3], "the note's place", line)?,
                }));
            }
            "set" => {
                want(5)?;
                list.events.push(Event::Set {
                    channel: whole(args[0], "the channel", line)?,
                    block: whole(args[1], "the block", line)?,
                    setting: whole(args[2], "the setting", line)?,
                    value: number(args[3], "the value", line)?,
                    time: number(args[4], "the time", line)?,
                });
            }
            "route" => {
                want(6)?;
                list.events.push(Event::Route {
                    channel: whole(args[0], "the channel", line)?,
                    block: whole(args[1], "the block", line)?,
                    setting: whole(args[2], "the setting", line)?,
                    from: whole(args[3], "the channel routed", line)?,
                    bus: whole(args[4], "the bus routed", line)?,
                    time: number(args[5], "the time", line)?,
                });
            }
            "event" => {
                want(4)?;
                list.events.push(Event::Channel {
                    channel: whole(args[0], "the channel", line)?,
                    kind: whole(args[1], "the kind", line)?,
                    time: number(args[2], "the time", line)?,
                    value: number(args[3], "the value", line)?,
                });
            }
            "retune" => {
                want(8)?;
                let mut notes = [0.0; 6];
                for (slot, word) in notes.iter_mut().zip(&args[2..]) {
                    *slot = number(word, "a note", line)?;
                }
                list.events.push(Event::Retune {
                    channel: whole(args[0], "the channel", line)?,
                    time: number(args[1], "the time", line)?,
                    notes,
                });
            }
            "note" => {
                want(7 + EXTRA)?;
                let mut extra = [0.0; EXTRA];
                for (slot, word) in extra.iter_mut().zip(&args[7..]) {
                    *slot = number(word, "an extra value", line)?;
                }
                list.events.push(Event::Note {
                    voice: whole(args[0], "the voice", line)?,
                    channel: whole(args[1], "the channel", line)?,
                    time: number(args[2], "the time", line)?,
                    midi: number(args[3], "the pitch", line)?,
                    dur: number(args[4], "the length", line)?,
                    vel: number(args[5], "the velocity", line)?,
                    parts: whole(args[6], "the parts", line)?,
                    extra,
                });
            }
            other => return Err(format!("line {line}: no such line: {other}")),
        }
    }
    if !seen_header {
        return Err("empty: a note list starts with: dlnotes 2".to_string());
    }
    if !(list.rate >= 8000.0 && list.rate <= 384_000.0) {
        return Err("the rate is missing or not between 8000 and 384000 Hz".to_string());
    }
    if list.frames == 0 {
        return Err("the frame count is missing".to_string());
    }
    Ok(list)
}

/// FNV-1a, 64 bits, over bytes: the hash a render is kept by. It is not a
/// secure hash and need not be; it has to be short, the same on every
/// machine and written in a few lines in any language (the page's tools
/// have their own, `tools/golden-lib.mjs`, and a test pins both to the same
/// published answers). Samples go in as their IEEE-754 bit patterns, four
/// bytes each, least significant first, whatever the machine's byte order.
pub struct Fnv(u64);

impl Fnv {
    pub const fn new() -> Self {
        Fnv(0xcbf2_9ce4_8422_2325)
    }
    pub fn bytes(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.0 = (self.0 ^ u64::from(b)).wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    pub fn samples(&mut self, samples: &[f32]) {
        for s in samples {
            self.bytes(&s.to_bits().to_le_bytes());
        }
    }
    pub fn hex(&self) -> String {
        format!("{:016x}", self.0)
    }
}

/// What a render gives back.
pub struct Rendered {
    /// One hash per signal, in `SIGNALS` order.
    pub hashes: Vec<String>,
    /// The signal asked to be kept, `frames` samples of it.
    pub kept: Vec<f32>,
}

/// Render a list through a fresh core and hash every signal. `keep` is the
/// index in `SIGNALS` of one signal whose samples to return as well.
pub fn render(list: &List, keep: Option<usize>) -> Result<Rendered, String> {
    render_on(list, keep, false)
}

/// `render`, with the processor set to flush subnormal numbers to zero
/// (inputs and results), as Chromium sets its audio threads. The core never
/// makes a subnormal number (`dlcore::math`, "Subnormal numbers"), so this
/// must hash the same as `render`; it is how that is checked. x86-64 and
/// aarch64 only.
pub fn render_flushing(list: &List, keep: Option<usize>) -> Result<Rendered, String> {
    render_on(list, keep, true)
}

fn render_on(list: &List, keep: Option<usize>, flushing: bool) -> Result<Rendered, String> {
    // The core is megabytes of zeros: it is built on a thread whose stack
    // can hold it (a debug build copies it a few times on the way), and
    // lives on the heap.
    let stack = std::mem::size_of::<Core>() * 4 + (16 << 20);
    std::thread::scope(|scope| {
        std::thread::Builder::new()
            .stack_size(stack)
            .spawn_scoped(scope, || {
                if flushing {
                    flush_subnormals()?;
                }
                let mut core = Box::new(Core::new());
                play(&mut core, list, keep)
            })
            .map_err(|e| format!("could not start the render thread: {e}"))?
            .join()
            .map_err(|_| "the render thread panicked".to_string())?
    })
}

/// Turn on flush-to-zero and denormals-are-zero for this thread: MXCSR's
/// FTZ and DAZ bits on x86-64, FPCR's FZ bit on aarch64 (which flushes
/// both). Threads start with them off.
#[cfg(target_arch = "x86_64")]
// The same signature as on the targets where it fails.
#[allow(clippy::unnecessary_wraps)]
fn flush_subnormals() -> Result<(), String> {
    let mut csr: u32 = 0;
    // SAFETY: reads and writes this thread's SSE control register, nothing
    // else; the bits set change only how subnormal numbers round.
    unsafe {
        std::arch::asm!("stmxcsr [{}]", in(reg) &raw mut csr, options(nostack));
        csr |= (1 << 15) | (1 << 6);
        std::arch::asm!("ldmxcsr [{}]", in(reg) &raw const csr, options(nostack));
    }
    Ok(())
}

#[cfg(target_arch = "aarch64")]
// The same signature as on the targets where it fails.
#[allow(clippy::unnecessary_wraps)]
fn flush_subnormals() -> Result<(), String> {
    // SAFETY: as above, for this thread's floating-point control register.
    unsafe {
        let mut fpcr: u64;
        std::arch::asm!("mrs {}, fpcr", out(reg) fpcr, options(nomem, nostack));
        fpcr |= 1 << 24;
        std::arch::asm!("msr fpcr, {}", in(reg) fpcr, options(nomem, nostack));
    }
    Ok(())
}

#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
fn flush_subnormals() -> Result<(), String> {
    Err("--flush is for x86-64 and aarch64".to_string())
}

/// Start a core for a list and hand it every call, before the first block:
/// the host's side of a render, whichever way the samples are then taken.
pub fn load(core: &mut Core, list: &List) -> Result<(), String> {
    core.start(list.rate, 1).map_err(|e| e.message().to_string())?;
    core.quality(list.full);
    if !core.mixing() {
        return Err(format!("the core does not mix at {} Hz", list.rate));
    }
    for (n, event) in list.events.iter().enumerate() {
        match event {
            Event::Param { id, op, value, time, tau } => {
                if !core.param(*id, *op, *value, *time, *tau) {
                    return Err(format!("event {}: the core refused parameter {id}", n + 1));
                }
            }
            Event::Damp { channel, time } => core.damp(*channel, *time),
            Event::Key(key) => core.key(*key),
            Event::Set { channel, block, setting, value, time } => {
                if !core.set(*channel, *block as usize, *setting as usize, *value, *time) {
                    return Err(format!("event {}: the core refused setting {setting} of block {block}", n + 1));
                }
            }
            Event::Route { channel, block, setting, from, bus, time } => {
                let tap = (*from != u32::MAX).then_some(dlcore::Tap { channel: *from, block: (*bus != u32::MAX).then_some(*bus as usize) });
                if !core.route(*channel, *block as usize, *setting as usize, tap, *time) {
                    return Err(format!("event {}: the core refused a route into setting {setting} of block {block}", n + 1));
                }
            }
            Event::Channel { channel, kind, time, value } => {
                if !core.event(*channel, dlcore::block::Event::new(*kind, *time, *value)) {
                    return Err(format!("event {}: the core refused an event of kind {kind}", n + 1));
                }
            }
            Event::Retune { channel, time, notes } => {
                if !core.event(*channel, dlcore::block::Event::retune(*time, notes)) {
                    return Err(format!("event {}: the core refused a retune", n + 1));
                }
            }
            Event::Note { voice, channel, time, midi, dur, vel, parts, extra } => {
                let taken = core.note(*voice, *channel, *time, *midi, *dur, *vel, *parts, *extra);
                if taken != Taken::OnTime {
                    return Err(format!("event {}: the core took the note as {taken:?}, not on time", n + 1));
                }
            }
        }
    }
    Ok(())
}

pub fn play(core: &mut Core, list: &List, keep: Option<usize>) -> Result<Rendered, String> {
    load(core, list)?;
    let mut hashes: Vec<Fnv> = SIGNALS.iter().map(|_| Fnv::new()).collect();
    let mut kept = Vec::new();
    let mut frame = 0u64;
    while frame < list.frames {
        let taps = *core.process(frame);
        let mut blocks: [&[f32]; CHANNELS + 2] = [&[]; CHANNELS + 2];
        blocks[0] = &core.output()[0];
        for (slot, tap) in blocks[1..=CHANNELS].iter_mut().zip(taps.iter()) {
            *slot = &tap[..];
        }
        blocks[CHANNELS + 1] = &core.bus()[0];
        // The last block may run past the end; the end is the list's.
        let used = (list.frames - frame).min(QUANTUM as u64) as usize;
        for (i, block) in blocks.iter().enumerate() {
            let block = block.get(..used).unwrap_or(&[]);
            hashes[i].samples(block);
            if keep == Some(i) {
                kept.extend_from_slice(block);
            }
        }
        frame += QUANTUM as u64;
    }
    // A render with notes dropped or late is not a golden: it would not
    // be the same on a machine that is faster or slower.
    if core.dropped() > 0 || core.late() > 0 || core.missed() > 0 {
        return Err(format!(
            "the core dropped {}, took {} late and missed {} notes",
            core.dropped(),
            core.late(),
            core.missed()
        ));
    }
    Ok(Rendered { hashes: hashes.iter().map(Fnv::hex).collect(), kept })
}

impl List {
    /// The same list at another sample rate. A list is in seconds and Hz, so
    /// nothing in it changes but how many frames the same time is.
    #[must_use]
    // A list is minutes of frames at most, far under what a double holds.
    #[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    pub fn at_rate(&self, rate: f64) -> List {
        List {
            name: format!("{}@{}", self.name, rate as u64),
            rate,
            full: self.full,
            frames: (self.frames as f64 * rate / self.rate).round() as u64,
            events: self.events.clone(),
        }
    }
}

/// How a host asks for its frames, in `render_blocks`.
#[derive(Clone, Copy, Debug)]
pub enum Blocks {
    /// The same number of frames every call (the last call is what is left).
    Fixed(usize),
    /// A different number every call, from 1 to 2,000, in a fixed order.
    Random,
}

impl Blocks {
    /// `1`, `441`, ... or `random`.
    pub fn parse(word: &str) -> Result<Blocks, String> {
        if word == "random" {
            return Ok(Blocks::Random);
        }
        match word.parse::<usize>() {
            Ok(n) if n > 0 => Ok(Blocks::Fixed(n)),
            _ => Err(format!("a block size is a whole number of frames or random, not {word}")),
        }
    }
}

/// Render a list's speaker output the way a game's audio callback asks for
/// it: through `Core::render`, `blocks` frames a call. Returns the hash of
/// the samples, which must be the `out` hash of the same list rendered any
/// other way.
pub fn render_blocks(list: &List, blocks: Blocks) -> Result<String, String> {
    let stack = std::mem::size_of::<Core>() * 4 + (16 << 20);
    std::thread::scope(|scope| {
        std::thread::Builder::new()
            .stack_size(stack)
            .spawn_scoped(scope, || {
                let mut core = Box::new(Core::new());
                load(&mut core, list)?;
                let mut hash = Fnv::new();
                let mut buffer = vec![0.0f32; 2000];
                let mut state = 0x2545_f491_u32;
                let mut done = 0u64;
                while done < list.frames {
                    let want = match blocks {
                        Blocks::Fixed(n) => n,
                        Blocks::Random => {
                            state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                            1 + (state >> 8) as usize % 2000
                        }
                    };
                    let take = want.min(buffer.len()).min((list.frames - done) as usize);
                    let out = &mut buffer[..take];
                    core.render(&mut [out]).map_err(|e| e.message().to_string())?;
                    hash.samples(&buffer[..take]);
                    done += take as u64;
                }
                if core.dropped() > 0 || core.late() > 0 || core.missed() > 0 {
                    return Err("the core dropped or missed notes".to_string());
                }
                Ok(hash.hex())
            })
            .map_err(|e| format!("could not start the render thread: {e}"))?
            .join()
            .map_err(|_| "the render thread panicked".to_string())?
    })
}

/// The hash line for a render: the list's name, its length and each
/// signal's hash.
pub fn line(list: &List, hashes: &[String]) -> String {
    let mut line = format!("{} frames={}", list.name, list.frames);
    for (signal, hash) in SIGNALS.iter().zip(hashes) {
        let _ = write!(line, " {signal}={hash}");
    }
    line
}

/// A mono 32-bit float WAV: the format every tool reads, and exact (a
/// 16-bit file would round the samples a comparison is after).
pub fn wav(samples: &[f32], rate: f64) -> Vec<u8> {
    wav_channels(samples, rate, 1)
}

/// The same with `channels` channels, `samples` interleaved: a stereo
/// block's two sides (`dlrender --block NAME/PROBE:stereo`).
pub fn wav_channels(samples: &[f32], rate: f64, channels: u16) -> Vec<u8> {
    let data = samples.len() * 4;
    let rate = rate as u32;
    let frame = 4 * u32::from(channels);
    let mut out = Vec::with_capacity(44 + data);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data as u32).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&3u16.to_le_bytes()); // IEEE float
    out.extend_from_slice(&channels.to_le_bytes());
    out.extend_from_slice(&rate.to_le_bytes());
    out.extend_from_slice(&(rate * frame).to_le_bytes());
    out.extend_from_slice(&(frame as u16).to_le_bytes());
    out.extend_from_slice(&32u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&(data as u32).to_le_bytes());
    for s in samples {
        out.extend_from_slice(&s.to_bits().to_le_bytes());
    }
    out
}

/// The samples of a WAV that `wav` wrote (and the rate), or why it is not
/// one.
pub fn read_wav(bytes: &[u8]) -> Result<(Vec<f32>, u32), String> {
    let at = |from: usize, len: usize| bytes.get(from..from + len).ok_or_else(|| "a truncated WAV".to_string());
    if at(0, 4)? != b"RIFF" || at(8, 8)? != b"WAVEfmt " || at(36, 4)? != b"data" {
        return Err("not a WAV written by dlrender".to_string());
    }
    let word = |from: usize| -> Result<u32, String> {
        Ok(u32::from_le_bytes(at(from, 4)?.try_into().map_err(|_| "a truncated WAV".to_string())?))
    };
    let (rate, data) = (word(24)?, word(40)? as usize);
    let samples = at(44, data)?
        .as_chunks::<4>()
        .0
        .iter()
        .map(|bytes| f32::from_bits(u32::from_le_bytes(*bytes)))
        .collect();
    Ok((samples, rate))
}
