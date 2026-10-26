# Testing

Everything below ran on a **simulated TV** (the scriptable fake in `crates/tvs-transport/src/mock.rs`,
dressed as a TCL Google TV in `crates/tvs-demo`). No result here has been confirmed on real hardware yet.
The simulator reproduces documented firmware behaviour (from CleanYourTV's notes and Android sources). It
can't reproduce firmware it doesn't know about. Passing these tests means the logic does what it should;
it does not mean a given TV behaves the way the simulator assumes.

## How to run

```powershell
cargo test --workspace                          # 145 core tests, including the 13 end-to-end scenarios
cargo clippy --workspace --all-targets -- -D warnings
pnpm typecheck; pnpm test                       # 39 UI tests
cd launcher; .\gradlew.bat testReleaseUnitTest lintRelease   # launcher (needs the Android SDK; CI runs it)
```

CI (`.github/workflows/ci.yml`) runs all of these on every push, builds the launcher APK, checks its size
(576 KB limit) and package name, then builds the Windows installer with the APK inside.

## The simulated TV

The simulated TV is modelled on CleanYourTV's tested TCL Smart TV Pro (G09, Google TV 12) list:

- 62 packages: the 36 packages that list disables, the TCL packages it keeps, Google TV, core Android and four streaming apps.
- The setup wizard (`setupwraith`) owns the saved Wi-Fi network.
- `com.tcl.tvinput` requests `TV_INPUT_HARDWARE`, and `com.tcl.tv` handles the channel intent.
- The stock Home is Google TV's launcher.
- Benchmark data where the Studio launcher is faster and lighter. These are invented numbers, so they show the comparison works, not that it will come out this way.
- A voice assistant with recent microphone use in AppOps.

Its model and fingerprint end in "(simulated)" and "demo-keys", so it can never share a recorded baseline with a real TV.

## End-to-end scenarios (spec §15)

| Scenario | Test (`crates/tvs-demo/tests/e2e.rs`) | Simulated result |
|---|---|---|
| Full deployment of the G09 list + launcher + theme + settings, then **Revert to original** | `full_g09_deployment_then_exact_revert_to_original` | Every package, Home and setting matches the original; Studio launcher removed only after stock Home is back |
| Multiple TVs and an unauthorized one | `multiple_devices_and_an_unauthorized_one` | Only the chosen TV changes; the unauthorized TV can't be planned |
| Protected packages (core, TV input, Wi-Fi owner, Settings) | `protected_and_tv_specific_packages_are_refused` | Plan blocked with the specific reason; nothing sent |
| Setup wizard owns Wi-Fi | same | Refused, with a reason that names Wi-Fi |
| TV changes between plan and run | `the_tv_changes_between_plan_and_run` | Already-disabled package not repeated; removed package not attempted; no failure |
| Invalid configuration (unknown profile field, bad package name, bad DNS host, bad theme values) | `invalid_configuration_is_rejected_before_anything_runs` | Rejected before anything runs. **Bug found and fixed:** a bad package name used to be skipped silently |
| Firmware refuses every disable | `firmware_that_refuses_every_disable` | Run stops after the first batch; nothing claimed |
| Sticky packages (firmware re-enables them) | `sticky_packages_are_reported_not_claimed` | Reported by name as not changed; the others verified |
| TV reboots / drops Wi-Fi mid-run | `tv_reboots_mid_run_and_the_run_finishes_once` | Reconnects, re-checks, finishes; one snapshot |
| Interrupted run, then resume | `interrupted_run_resumes_from_the_tvs_real_state` | Every package disabled exactly once in total |
| Benchmark reproducibility | `benchmark_is_reproducible_and_the_verdict_is_cautious` | Same launcher twice → never "better"/"worse"; after switching → "better" with ≥5 samples; nothing changed on the TV |
| Privacy audit | `privacy_audit_on_the_simulated_tv` | Mic history found; wording never says "listening"; read-only |
| Pairing + connect (demo world) | `the_demo_world_connects_and_pairs` | Wrong code refused, right code pairs |

More focused cases (reconnect give-up, sensitive-app confirmations, wrong-TV profile, Private DNS on old
Android, drift and repair, theme delivery fallbacks, Home handover on Google TV's "Success but nothing changed"
quirk, snapshot-first ordering) live in each crate's own tests.

## Demo mode

Turn it on in Settings → Demo, or set `TVS_DEMO=1`. The real app then talks to the simulated TV instead of adb.
Every screen works: connect, inspect, debloat, themes, launcher, deployment, benchmarks, privacy, diagnostics
and Revert to original. A banner on every screen says it's simulated. The simulated TV keeps its state until the app closes.

## What only a real TV can prove (open)

Run this checklist on at least one **Google TV** and one **Android TV** (ideally one from a different brand) before claiming support:

1. Connect: legacy `adb connect` on port 5555, and Android 11+ Wireless debugging pairing; accept the prompt; reconnect after a TV restart.
2. Inspector values match the TV's own Settings → About.
3. Debloat a few low-risk apps in Safe mode, then confirm each one is gone from the TV's app list and stays gone after a reboot.
4. Restore: everything disabled comes back; nothing that was already disabled before is enabled.
5. Studio launcher: install, make Home, **press Home on the remote** and confirm the Studio launcher appears. On Google TV, check that "Success" without the screen changing is reported as not confirmed.
6. Theme: apply, change wallpaper, reset; the launcher redraws without a reboot.
7. Apply Everything with a reboot in the middle (pull the power during the debloat step): resume finishes and nothing is done twice.
8. Revert to original: back to stock Home, Studio launcher removed, apps and animation/DNS settings as before.
9. Benchmark twice with the same launcher: expect "no clear change". The simulator can't tell us how noisy real TVs are.
10. After a firmware update (if one comes): the drift check lists what came back.

Record each result in TASKS.md with the TV model and firmware. Until then, the capability matrix stays marked as simulator-only.

## Not automated yet

- Driving the desktop window itself (tauri-driver + WebDriver) only works on Windows. Planned for CI's Windows job; UI behaviour is covered by the 39 component tests in the meantime.
- The launcher's Android UI is checked by its unit tests and lint, not by an emulator run.
