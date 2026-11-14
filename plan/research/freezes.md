# The long freezes of the S-125 playtest (S-136)

Status: 2026-11-15. The cause is outside the game. Tests were done on
Mikey's playtest laptop (Fedora 45, KDE Plasma 6.8.2 on Wayland,
NVIDIA GTX 1650 with driver 615.78, Intel UHD Graphics for the screen).

## The result in short

- The game code does not cause the freezes. In a long frame, the game
  waits in the present call of the graphics driver. The present is the
  step that gives a finished frame to the desktop.
- The desktop compositor, KWin, causes them. A compositor is the
  program that puts all windows on the screen. KWin's main thread
  stops for about 940 ms every few minutes. All of that time is in one
  kernel call. This also occurs when the game does not run.
- The 5 s freezes in the first minute are KWin hangs too. The system
  log has "The main thread was hanging temporarily!" from KWin at
  21:40:32, 21:41:05 and 21:41:15. The playtest has its long frames at
  the same seconds.
- "Script time, process" (15.6 ms) was not a script time. It is
  Godot's `TIME_PROCESS` monitor: the slowest process step of each
  second, with the draw and the present. The real script time is about
  0.5 ms in a frame, and the physics about 0.8 ms.

## How the cause was found

1. A script started the game and looked at the main thread 50 times
   each second. When the thread did not move for 250 ms, `gdb` printed
   its stack.
2. Each stack was the same: `ioctl` in `drmSyncobjTimelineWait`, from
   the NVIDIA driver, from the draw of Godot. The driver waits there
   until the desktop takes the frame.
3. This laptop is a hybrid. The Intel GPU controls the screen, and
   KWin draws the desktop with it. The NVIDIA GPU draws the game.
4. The same script then looked at KWin's main thread. During a game
   freeze of 946 ms, KWin was on the CPU for 936 ms without a pause.
5. With no game, KWin stopped for 932 ms and for 944 ms in two tests
   of a few minutes. The CPU time was kernel time only. No device
   event and no log line came at those moments.

## Settings that were tested (8 minutes each, the bear stands still)

| Setting | Freezes of about 0.9 s |
|---|---|
| Default (X11 through XWayland, vsync on) | 3 |
| Native Wayland (`--display-driver wayland`) | 9 |
| Vsync off, 144 fps limit | 1 |
| Mailbox vsync, 144 fps limit | 0 |
| Intel GPU (`--gpu-index 0`, 4 minutes) | 0, but 12 fps |

The counts are small, and the number of KWin stalls changes from run
to run. Do not read them as a ranking. With mailbox vsync, the game
does not wait for the desktop, so the recorder sees no long frame. The
picture on the screen still stops when KWin stops. Thus the game keeps
its default vsync.

## What is not known yet

- Which kernel call KWin makes. A user without root cannot read the
  kernel stack of KWin. `tools/.cache/kwin-stall-trace.py` (not in
  git) reads it with `sudo`.
- Suspects [unverified]: a probe of the monitor ports of the NVIDIA
  GPU, memory compaction (the laptop has 7.6 GB RAM and uses swap;
  the kernel log has NVIDIA "Out of memory" lines), or the game
  controller plugin of KWin.
- Cleared: PowerDevil's monitor probe (`ddcutil detect` takes 0.2 s
  and makes no freeze).

## What changed in the game

- The recorder has a probe for the parts of each frame (`parts`:
  draw and present, idle, physics, script, other). Each hitch event has
  them, and the report counts the hitches outside the game code. The
  next playtest shows at once if a freeze is in the game or not.
- The report gives the correct name to the two Godot monitors.
- `session.json` has the display server and the desktop session type.

## A different problem that these tests found

On the Intel GPU of this laptop (UHD Graphics, CML GT2), the game runs
at 12 fps: the GPU time is 80 ms for each frame at 1920 x 1023. This
GPU is of the same class as the speed target (R5). See the agenda.
