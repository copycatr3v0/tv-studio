//! The Studio launcher side: theme schema and validation, wallpaper intake, theme delivery, and
//! install / Home handover / uninstall of the launcher APK.

pub mod assets;
pub mod install;
pub mod revert;
pub mod schema;
pub mod theme;

#[cfg(test)]
mod tests;
