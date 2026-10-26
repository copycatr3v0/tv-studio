// How the preview names and colours apps. Real artwork is drawn by the TV; the preview uses a stable
// colour per package so layouts can be judged without pulling banners from the device.

export interface PreviewApp { pkg: string; label: string; color: string }

const KNOWN: Record<string, string> = {
  "com.netflix.ninja": "Netflix",
  "com.google.android.youtube.tv": "YouTube",
  "com.amazon.amazonvideo.livingroom": "Prime Video",
  "com.disney.disneyplus": "Disney+",
  "in.startv.hotstar": "Hotstar",
  "com.spotify.tv.android": "Spotify",
  "com.plexapp.android": "Plex",
  "org.xbmc.kodi": "Kodi",
  "org.videolan.vlc": "VLC",
  "com.android.vending": "Play Store",
  "com.google.android.youtube.tvmusic": "YouTube Music",
  "com.apple.atve.androidtv.appletv": "Apple TV",
  "com.wbd.hbomax": "Max",
  "com.jio.media.stb.ondemand": "JioCinema",
  "com.sonyliv": "SonyLIV",
  "com.zee5.androidtv": "ZEE5",
};

const SKIP = new Set(["com", "org", "net", "in", "tv", "android", "google", "apps", "app", "atv", "androidtv", "livingroom", "ninja", "leanback"]);

export function labelFor(pkg: string): string {
  if (KNOWN[pkg]) return KNOWN[pkg];
  const parts = pkg.split(".").filter((p) => !SKIP.has(p.toLowerCase()));
  const word = parts[0] ?? pkg.split(".").pop() ?? pkg;
  return word.charAt(0).toUpperCase() + word.slice(1);
}

export function colorFor(pkg: string): string {
  let h = 0;
  for (let i = 0; i < pkg.length; i++) h = (h * 31 + pkg.charCodeAt(i)) >>> 0;
  return `hsl(${h % 360} 55% 42%)`;
}

export const toApp = (pkg: string): PreviewApp => ({ pkg, label: labelFor(pkg), color: colorFor(pkg) });

export const SAMPLE_APPS: PreviewApp[] = [
  "com.netflix.ninja", "com.google.android.youtube.tv", "com.amazon.amazonvideo.livingroom", "com.disney.disneyplus",
  "com.spotify.tv.android", "com.plexapp.android", "org.xbmc.kodi", "org.videolan.vlc", "com.android.vending",
  "com.google.android.youtube.tvmusic", "com.apple.atve.androidtv.appletv", "com.wbd.hbomax",
].map(toApp);
