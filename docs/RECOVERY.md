# RECOVERY.md

## What the app records
- **Baseline** (one per TV, write-once): every package's enabled/disabled state for user 0, the default Home
  component, and the three animation scales, captured right before the first change this app ever makes.
  A later or retried run never replaces it, so a half-applied run can't become the "original".
- **Pre-change snapshot**: the same data right before every deployment or debloat run, kept with that run.
- Settings recorded: the three animation scales and Private DNS (`private_dns_mode`, `private_dns_specifier`).

## Undo one deployment
Profiles → history (or the result screen) → **Undo**: returns the TV to its state right before that deployment.
If the deployment installed the Studio launcher, Home is handed back and confirmed on screen before it is removed.

## Interrupted deployments
If the app closes, the PC sleeps or the TV reboots mid-run, the Deployment page offers **Resume**. Resume reads
the TV again and plans from what is really there, so finished steps show as already done and nothing is repeated.
During a run, a dropped connection is retried for about three minutes; each step checks the TV before acting.

## What Restore does
- Re-enables packages that were enabled in the baseline and are disabled now.
- Leaves packages alone that were already disabled before the app changed anything, or that were not installed then.
- Puts the default Home and the animation scales back.
- Reads every change back from the TV and reports what did and did not take effect.
- Never disables anything.

## When ADB no longer reaches the TV
1. Reconnect from **Connect TV**. Wireless debugging turns off when Wi-Fi drops; the port changes each time.
2. If the TV lost Wi-Fi (for example a setup-wizard network), join the network again in the TV's Settings, then reconnect.
3. If Home is black, press Home a few times; many TVs open Settings from a long press of Home or from the remote's Settings
   button. From Settings → Apps → See all apps → Show system apps, re-enable the stock launcher.
4. Safe mode (varies by maker, often holding Power on the remote or the TV's power button) starts the TV with only system apps.
5. A factory reset from the recovery menu restores everything, and erases accounts and apps.

The app never reports a restore as successful unless it could read the result back from the TV.
