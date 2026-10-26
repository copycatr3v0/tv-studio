# BENCHMARKS.md

The app measures the Home screen of the connected TV, whichever launcher answers Home, so a run before
customizing and a run after can be compared on the **same TV**. Method adapted from CleanYourTV's
`tools/bench.py` and `docs/workflows/perf-ab.md` (MIT, © 2026 Gathin).

| Metric | How | Notes |
|---|---|---|
| Remote key response | Reset `dumpsys gfxinfo <launcher>`, inject a D-pad key, read `framestats`; first frame that *completes* after the key | Key time from the input log when the TV logs injected keys, else from the reset (slightly pessimistic). Keys with no frame within 150 ms caused no visual reaction and are left out. |
| Focus movement finished | Last frame of the continuous run after the key (a gap > 50 ms ends it) | Includes animation length: shorter animations improve it without the TV being faster. |
| Launcher cold start | Settings in front → `am force-stop` → `am start -W` the HOME activity; `WaitTime` | 5 runs (3 in quick mode). |
| Return to Home | `logcat -c` → HOME key → last `am/wm_on_resume_called` of the launcher after the key (epoch log column for both) | 5 runs. |
| Janky frames | `Janky frames / Total frames rendered` over the key test | One aggregate per run. |
| Idle CPU | `/proc/stat` and `/proc/<pid>/stat` deltas over 10 s with Home in front | % of one core for the launcher, whole-device % separately. |
| Launcher memory | `dumpsys meminfo <launcher>` TOTAL PSS | One reading. |

## Verdicts
- Every metric is lower-is-better.
- **Better / Worse** only when both runs have ≥ 5 samples and their interquartile ranges don't overlap.
- Single readings (jank %, idle CPU, PSS) need a margin larger than their noise (2 %, 0.5 %, 3 %, 5 MB).
- Otherwise: **No clear change** or **Not enough data**. The app never promises a TV will be faster.
- Comparing different launchers compares the Home-screen experience, and the result says so.

## What the benchmark changes
Only which app is in front, the gfxinfo counters and the main log buffer (cleared before each return-to-Home
sample). No packages, settings or Home choices are touched.
