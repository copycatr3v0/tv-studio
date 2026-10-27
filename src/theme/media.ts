/** Which pictures and videos Studio accepts, and how each one reaches the TV. */

/** Containers the TV's own player opens (H.264/H.265/VP8/VP9 inside). Mirrors the Rust and launcher lists. */
export const VIDEO_EXTS = ["mp4", "m4v", "webm", "mkv", "mov", "3gp", "ts"] as const;
export const VIDEO_ACCEPT = [...VIDEO_EXTS.map((e) => `.${e}`), "video/*"].join(",");
export const MAX_VIDEO_MB = 60;
export const isVideoName = (name: string) => new RegExp(`\\.(${VIDEO_EXTS.join("|")})$`, "i").test(name);
/** Formats the TV cannot play at all: refused with a plain reason instead of failing on the TV. */
export const UNPLAYABLE_VIDEO = /\.(avi|wmv|flv|mpg|mpeg|rm|rmvb|vob|ogv)$/i;

/** Pictures the desktop side can read itself (re-encoded to JPEG; animated GIF/WebP kept and played as live wallpapers). */
const DIRECT = /^image\/(jpeg|png|webp|gif|bmp|tiff)$/;
export const IMAGE_ACCEPT = "image/*,.heic,.heif,.avif,.jxl";

/** A picture as a data URL the theme can carry: readable types as they are, anything else the browser can show
 *  (AVIF, HEIC where supported, ICO, SVG…) converted to JPEG here. */
export async function pictureForTheme(f: File, read: (f: File) => Promise<string>): Promise<{ name: string; url: string }> {
  if (DIRECT.test(f.type)) return { name: f.name, url: await read(f) };
  let bmp: ImageBitmap;
  try { bmp = await createImageBitmap(f); }
  catch { throw new Error(`${f.name}: this computer can't read that picture type. Save it as JPG or PNG and add it again.`); }
  const scale = Math.min(1, 3840 / bmp.width, 2160 / bmp.height);
  const c = document.createElement("canvas");
  c.width = Math.max(1, Math.round(bmp.width * scale));
  c.height = Math.max(1, Math.round(bmp.height * scale));
  c.getContext("2d")!.drawImage(bmp, 0, 0, c.width, c.height);
  bmp.close();
  return { name: f.name.replace(/\.[^.]+$/, "") + ".jpg", url: c.toDataURL("image/jpeg", 0.92) };
}
