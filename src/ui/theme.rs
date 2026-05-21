//! UI theme tokens. Mirrors `ui_mock/theme.css`.
//!
//! Use [`palette`], [`font_size`], [`space`] for new code. Old `COLOR_*`
//! constants remain as aliases pointing at the new palette.

use bevy::prelude::*;

/// Surface, text, and semantic colors.
pub mod palette {
    use bevy::prelude::Color;

    // surfaces
    pub const BG: Color = Color::srgb(0.102, 0.114, 0.129); // #1a1d21
    pub const P1: Color = Color::srgb(0.133, 0.149, 0.169); // #22262b
    pub const P2: Color = Color::srgb(0.165, 0.184, 0.212); // #2a2f36
    pub const P3: Color = Color::srgb(0.200, 0.224, 0.251); // #333940

    // text
    pub const TEXT: Color = Color::srgb(0.784, 0.804, 0.831); // #c8cdd4
    pub const DIM: Color = Color::srgb(0.353, 0.388, 0.439); // #5a6370

    // semantic
    pub const ACCENT: Color = Color::srgb(0.541, 0.447, 0.667); // #8a72aa
    pub const OK: Color = Color::srgb(0.290, 0.620, 0.416); // #4a9e6a
    pub const WARN: Color = Color::srgb(0.784, 0.706, 0.251); // #c8b440
    pub const ERR: Color = Color::srgb(0.722, 0.290, 0.290); // #b84a4a

    // rarity
    pub const RARITY_COMMON: Color = Color::srgb(0.478, 0.522, 0.565);
    pub const RARITY_UNCOMMON: Color = Color::srgb(0.290, 0.620, 0.416);
    pub const RARITY_RARE: Color = Color::srgb(0.290, 0.510, 0.784);
    pub const RARITY_EPIC: Color = Color::srgb(0.706, 0.314, 0.784);
    pub const RARITY_LEGENDARY: Color = Color::srgb(0.824, 0.667, 0.118);

    // misc
    pub const BORDER: Color = Color::srgba(1.0, 1.0, 1.0, 0.07);
    pub const BORDER_STRONG: Color = Color::srgba(1.0, 1.0, 1.0, 0.12);
    pub const OVERLAY_SCRIM: Color = Color::srgba(0.0, 0.0, 0.0, 0.627);
    pub const PANEL_SCRIM: Color = Color::srgba(0.0, 0.0, 0.0, 0.706);

    // interaction tints (layered over surface)
    pub const HOVER: Color = Color::srgba(1.0, 1.0, 1.0, 0.09);
    pub const PRESSED: Color = Color::srgba(1.0, 1.0, 1.0, 0.15);
    pub const ACCENT_HOVER: Color = Color::srgb(0.616, 0.522, 0.741); // #9d85bd
}

/// Font sizes (pixels).
pub mod font_size {
    pub const H_XL: f32 = 36.0; // titles, modal headings
    pub const H_LG: f32 = 18.0; // panel headings
    pub const H_MD: f32 = 16.0; // subheadings
    pub const H_SM: f32 = 13.0; // small headings, section titles
    pub const H_XS: f32 = 11.0; // tiny headings

    pub const LABEL: f32 = 12.0; // body labels
    pub const LABEL_SM: f32 = 10.0;

    pub const MONO: f32 = 11.0;
    pub const MONO_SM: f32 = 10.0;
    pub const MONO_XS: f32 = 9.0;

    pub const BUTTON: f32 = 14.0;
    pub const TAG: f32 = 9.0;
}

/// Spacing scale (pixels).
pub mod space {
    pub const XS: f32 = 2.0;
    pub const SM: f32 = 4.0;
    pub const MD: f32 = 8.0;
    pub const LG: f32 = 12.0;
    pub const XL: f32 = 16.0;
    pub const XXL: f32 = 24.0;
    pub const XXXL: f32 = 32.0;
}

/// Border widths (pixels).
pub mod border {
    pub const THIN: f32 = 1.0;
    pub const THICK: f32 = 2.0;
}

/// Default corner radius (pixels). Bevy UI uses [`BorderRadius`] separately.
pub mod radius {
    pub const SM: f32 = 2.0;
    pub const MD: f32 = 3.0;
    pub const LG: f32 = 4.0;
}

// ---------------------------------------------------------------------------
// Legacy aliases. Existing panels reference these — keep working until
// migrated. Prefer `palette::*` in new code.
// ---------------------------------------------------------------------------

pub const COLOR_GOLD: Color = Color::srgb(0.784, 0.659, 0.188);
pub const COLOR_DIM: Color = palette::DIM;
pub const COLOR_GREEN: Color = palette::OK;
pub const COLOR_OVERLAY_BG: Color = palette::OVERLAY_SCRIM;
pub const COLOR_PANEL_BG: Color = palette::PANEL_SCRIM;
