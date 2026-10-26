// Per-firmware setup guidance for the Connect wizard. Menu names differ between builds, so each guide
// says where the entry usually is; nothing here is used to make decisions about the device.

export interface Guide {
  id: string;
  label: string;
  developerOptions: string[];
  debugging: string[];
  note?: string;
}

const BUILD_7 = "Select the build number entry and press OK on the remote 7 times, until it says you are a developer.";

export const GUIDES: readonly Guide[] = [
  {
    id: "google-tv",
    label: "Google TV (Sony, TCL, Hisense, Chromecast, Philips 2022+)",
    developerOptions: ["Settings → System → About", "Scroll to “Android TV OS build”. " + BUILD_7],
    debugging: [
      "Settings → System → Developer options",
      "Turn on “USB debugging”.",
      "If you see “Wireless debugging” (Android 11+), turn it on and use the Wireless debugging tab here.",
      "If you see “Network debugging” / “ADB debugging”, turn it on and use port 5555.",
    ],
    note: "Many Google TV sets only offer Wireless debugging. It switches itself off when Wi-Fi drops.",
  },
  {
    id: "android-tv",
    label: "Android TV (Sony, Xiaomi Mi TV / Mi Box, TCL, Philips, Nokia, OnePlus)",
    developerOptions: ["Settings → Device Preferences → About", "Find “Build”. " + BUILD_7],
    debugging: [
      "Settings → Device Preferences → Developer options",
      "Turn on “USB debugging”.",
      "Turn on “Network debugging” or “ADB over network” if it is listed (port 5555).",
    ],
    note: "Philips: Settings may open its own menu first; look for “Android settings”.",
  },
  {
    id: "shield",
    label: "NVIDIA Shield",
    developerOptions: ["Settings → Device Preferences → About", "Find “Build”. " + BUILD_7],
    debugging: ["Settings → Device Preferences → Developer options", "Turn on “Network debugging”. The screen shows the address to use."],
  },
  {
    id: "other",
    label: "Other Android-based TV or projector",
    developerOptions: ["Open Settings and find “About” (often under Device Preferences or System).", "Find “Build” or “Build number”. " + BUILD_7],
    debugging: [
      "Open the new “Developer options” menu.",
      "Turn on “USB debugging”, and “Network debugging” or “Wireless debugging” if listed.",
    ],
    note: "Some projectors run network ADB on another fixed port. Use Diagnose to check it.",
  },
];

export const FIND_IP = [
  "Settings → Network & Internet (or Network) → select your Wi-Fi or Ethernet connection.",
  "Note the IP address, for example 192.168.1.50.",
  "Your PC must be on the same network. Guest Wi-Fi usually blocks this.",
];
