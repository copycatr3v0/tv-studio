# RECOVERY.md

## What the app records
- **Baseline** (one per TV, write-once): every package's enabled/disabled state for user 0, the default Home
  component, and the three animation scales, captured right before the first change this app ever makes.
  A later or retried run never replaces it, so a half-applied run can't become the "original".
- **Pre-change snapshot**: the same data before every run, kept for history.

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
