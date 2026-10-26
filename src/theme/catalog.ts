// Twenty-five original starting points. Configs only use keys the launcher supports; wallpapers are this
// project's own procedural art (tools/make_wallpapers.py).
import type { ThemeConfig } from "./schema";

export interface CatalogTheme {
  id: string;
  name: string;
  blurb: string;
  config: ThemeConfig;
  /** File under public/themes/, or null for no picture. */
  wallpaper: string | null;
}

export const CATALOG: readonly CatalogTheme[] = [
  { id: "amoled-black", name: "AMOLED Black", blurb: "True black, a quiet rim highlight, no glow. Easiest on OLED panels.", wallpaper: null,
    config: { background_color: "#000000", wallpaper: false, glass_style: "rim", ambient_glow: false, dock_band: false, columns: 5 } },
  { id: "cinematic-minimal", name: "Cinematic Minimal", blurb: "The focused app's artwork fills the top shelf like a film poster.", wallpaper: "cinematic-minimal.jpg",
    config: { background_color: "#050608", wallpaper: true, cinematic_home: true, cinematic_scrim: 80, wallpaper_dim: 20 } },
  { id: "premium", name: "Premium", blurb: "Apple TV-inspired: black, a glass highlight that springs between tiles, large thin clock.", wallpaper: "premium.jpg",
    config: { background_color: "#000000", wallpaper: true, glass_style: "glass", dock_title: true, focus_overshoot: true } },
  { id: "material", name: "Material", blurb: "Material-inspired uniform tiles tinted from each app's icon, names always visible.", wallpaper: "material.jpg",
    config: { background_color: "#121212", wallpaper: true, uniform_tiles: true, always_show_names: true, wallpaper_dim: 25, glass_style: "rim" } },
  { id: "cyberpunk-neon", name: "Cyberpunk Neon", blurb: "Cyan and magenta haze, a glowing halo focus in each app's colour.", wallpaper: "cyberpunk-neon.jpg",
    config: { background_color: "#04040E", wallpaper: true, glass_style: "halo", app_glow: true, dock_glass: true } },
  { id: "retro-futuristic", name: "Retro Futuristic", blurb: "Synthwave sunset over a neon grid.", wallpaper: "retro-futuristic.jpg",
    config: { background_color: "#14042A", wallpaper: true, wallpaper_dim: 15, glass_style: "halo", dock_opacity: 70 } },
  { id: "frosted-glass", name: "Frosted Glass", blurb: "Soft colour behind a frosted dock and frosted pills.", wallpaper: "frosted-glass.jpg",
    config: { background_color: "#141826", wallpaper: true, dock_glass: true, glass_style: "glass", wallpaper_dim: 10 } },
  { id: "aurora", name: "Aurora", blurb: "Northern lights over a starry night sky.", wallpaper: "aurora.jpg",
    config: { background_color: "#070A14", wallpaper: true, dock_opacity: 70, glass_style: "rim" } },
  { id: "luxury-dark", name: "Luxury Dark", blurb: "Near-black with a warm gold light from above.", wallpaper: "luxury-dark.jpg",
    config: { background_color: "#0A0908", wallpaper: true, glass_style: "rim", header_white: true, ambient_glow: false } },
  { id: "monochrome", name: "Monochrome", blurb: "Greyscale everything: uniform tiles and a rim focus.", wallpaper: "monochrome.jpg",
    config: { background_color: "#101010", wallpaper: true, uniform_tiles: true, glass_style: "rim", ambient_glow: false } },
  { id: "gaming", name: "Gaming", blurb: "Five columns, red and green energy, app-coloured glow.", wallpaper: "gaming.jpg",
    config: { background_color: "#050406", wallpaper: true, columns: 5, app_glow: true, glass_style: "halo" } },
  { id: "family", name: "Family Friendly", blurb: "Bright pastels, big tiles, every name shown, calmer motion.", wallpaper: "family.jpg",
    config: { background_color: "#E8D8D0", wallpaper: true, columns: 4, always_show_names: true, reduced_motion: true, wallpaper_dim: 10 } },
  { id: "nature", name: "Nature", blurb: "Layered mountain ridges at golden hour.", wallpaper: "nature.jpg",
    config: { background_color: "#1A2224", wallpaper: true, dock_opacity: 70, glass_style: "rim" } },
  { id: "gradient-modern", name: "Gradient Modern", blurb: "A bold coral-to-violet-to-teal sweep.", wallpaper: "gradient-modern.jpg",
    config: { background_color: "#3A2470", wallpaper: true, dock_glass: true, wallpaper_dim: 20 } },
  { id: "sunset-dunes", name: "Sunset Dunes", blurb: "Warm desert ridges under a low sun. Soft glass highlight.", wallpaper: "sunset-dunes.jpg",
    config: { background_color: "#2A1430", wallpaper: true, glass_style: "glass", dock_opacity: 65, wallpaper_dim: 10 } },
  { id: "ocean-deep", name: "Ocean Deep", blurb: "Light shafts through deep blue water, with a cool halo focus.", wallpaper: "ocean-deep.jpg",
    config: { background_color: "#020C1C", wallpaper: true, glass_style: "halo", dock_glass: true } },
  { id: "midnight-city", name: "Midnight City", blurb: "A lit skyline at night. Neon-pink haze, rim focus, names always shown.", wallpaper: "midnight-city.jpg",
    config: { background_color: "#0E0C28", wallpaper: true, glass_style: "rim", always_show_names: true, dock_opacity: 75 } },
  { id: "forest-mist", name: "Forest Mist", blurb: "Layered pine ridges in morning fog. Calm and low contrast.", wallpaper: "forest-mist.jpg",
    config: { background_color: "#0C2620", wallpaper: true, glass_style: "rim", uniform_tiles: true, wallpaper_dim: 15 } },
  { id: "sakura", name: "Sakura", blurb: "Soft pink petals, big friendly tiles and calm motion.", wallpaper: "sakura.jpg",
    config: { background_color: "#F2C4D4", wallpaper: true, columns: 4, always_show_names: true, reduced_motion: true, dock_glass: true } },
  { id: "space-nebula", name: "Space Nebula", blurb: "Violet and blue gas clouds with a field of stars. App-coloured glow.", wallpaper: "space-nebula.jpg",
    config: { background_color: "#06040F", wallpaper: true, glass_style: "halo", app_glow: true, dock_opacity: 60 } },
  { id: "ember", name: "Ember", blurb: "Dark with a low orange glow and drifting sparks. Warm and cosy.", wallpaper: "ember.jpg",
    config: { background_color: "#0C0605", wallpaper: true, glass_style: "rim", ambient_glow: false, header_white: true } },
  { id: "arctic", name: "Arctic", blurb: "A light theme: pale ice-blue ridges and a frosted dock. Good in bright rooms.", wallpaper: "arctic.jpg",
    config: { background_color: "#B7CFE3", wallpaper: true, dock_glass: true, glass_style: "glass", always_show_names: true } },
  { id: "retro-terminal", name: "Retro Terminal", blurb: "Phosphor-green code rain on black. Uniform tiles, no frills.", wallpaper: "retro-terminal.jpg",
    config: { background_color: "#020A04", wallpaper: true, uniform_tiles: true, glass_style: "rim", ambient_glow: false, columns: 6 } },
  { id: "kids-bright", name: "Kids Bright", blurb: "Primary colours, four big tiles per row, every name shown, gentle motion.", wallpaper: "kids-bright.jpg",
    config: { background_color: "#7CC8FF", wallpaper: true, columns: 4, always_show_names: true, reduced_motion: true, uniform_tiles: true } },
  { id: "blank", name: "Custom Blank", blurb: "The launcher's built-in defaults. Start from scratch.", wallpaper: null,
    config: {} },
];
