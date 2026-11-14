# The playtest recorder: references (S-123)

These notes are for the recorder (R22, `diagnostics.md`). The code is in
`rust/game/src/recorder.rs` and `rust/game/src/playtest/`.

## Sources

- **CapFrameX, "Explanation of different performance metrics"** (the
  tool has the MIT licence). It explains three kinds of "1% low":
  - The P99 frame time, given as fps.
  - The "1% low average": the mean of the slowest 1% of frames.
  - The "1% low integral": sort the slowest frames first and add them
    until the sum is 1% of the run time.
  The cost is only maths on the frame times. Tools use different
  formulas, so a report must name its formula.
  https://www.capframex.com/blog/post/Explanation%20of%20different%20performance%20metrics
- **Framedash Godot SDK** (MIT, C#/.NET, Godot 4.3+). It sends
  "heartbeats" (fps, frame time, memory, GPU time), events with a
  position for heat maps, and load times to a cloud service. It writes
  no local file. Use it for ideas only.
  https://github.com/crane-valley/framedash-godot-sdk
- **Sentry, Unreal performance metrics** (vendor blog). It samples frame
  times at a low rate and links each spike to a trace of that moment.
  A hitch in a frame without a sample is lost. A hitch guide
  (bugnet.io) says: track P95 and P99 frame times, not the average.
  https://blog.sentry.io/unreal-engine-performance-metrics

## What we use

- **Each frame counts.** The recorder adds each frame time to a
  histogram with bins of 0.1 ms. Thus no hitch is lost, and the memory
  does not grow with the play time.
- **"1% low" is the "1% low average"** (the mean of the slowest 1% of
  frames). The report names this formula. It also gives the P99 frame
  time.
- **Each second has the average, P95 and max frame time.**
- **Events with a place.** Each event has the position of the bear, as
  in Framedash.
- **A hitch is a spike.** A frame is a hitch if it is longer than
  50 ms *and* longer than 2 times the average frame of the last second.
  Without the second rule, a slow computer gives a hitch event for
  each frame (the cloud test at 3 fps gave 300 events in 100 s).

## Godot facts (checked by the research subagent in the docs and source)

- **The Godot log file.** `debug/file_logging/enable_file_logging.pc`
  is on by default, also in release builds on desktop. The log is
  `user://logs/godot.log`, and Godot keeps 5 old logs. The file is
  flushed at each error line. Normal print lines are flushed only if
  `application/run/flush_stdout_on_print` is on (off in release). Thus
  the recorder reads the errors and warnings from this file once each
  second.
- **`OS.add_logger` (Godot 4.5+)** gives `_log_error` and
  `_log_message`. They can run on other threads. The recorder does not
  use them yet, because a Rust class that Godot calls from another
  thread needs extra care in godot-rust [unverified].
- **`ZIPPacker`** uses Deflate (level 6) by default.
