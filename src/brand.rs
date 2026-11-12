//! WhyDown visual identity, available without external Rust crates or files.
//!
//! The icon is derived from the WD monogram chosen for the project. Keeping
//! the tiny PNG embedded guarantees that precompiled musl binaries can expose
//! the same icon for Qt, desktop launchers and packaging without asset lookup.

pub const ICON_PNG: &[u8] = include_bytes!("../assets/whydown-icon.png");

/// Plain ASCII only: readable over SSH, serial consoles and non-UTF-8 locales.
pub const ASCII_LOGO: &str = concat!(
    "  W   W  DDDD\n",
    "  W W W  D   D   o---o\n",
    "   W W   D   D        \\---o\n",
    "   W W   DDDD       WhyDown\n",
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_icon_is_png_and_ascii_is_portable() {
        assert!(ICON_PNG.starts_with(b"\x89PNG\r\n\x1a\n"));
        assert!(ICON_PNG.len() > 1_000);
        assert!(ASCII_LOGO.is_ascii());
        assert!(ASCII_LOGO.contains("WhyDown"));
        assert!(ASCII_LOGO.contains("o---o"));
    }
}
