//! Bundle builders for themed UI primitives.
//!
//! Each helper returns `impl Bundle` so spawn calls stay short and
//! consistent. Compose with `.with_children`, or add extra components by
//! tupling with the returned bundle.
//!
//! ```ignore
//! parent.spawn((panel(), MyMarker)).with_children(|p| {
//!     p.spawn(heading("EXERGON", H::Xl));
//! });
//! ```

use bevy::prelude::*;

use crate::ui::theme::{border, font_size, palette, radius, space};

// ---------------------------------------------------------------------------
// Layout
// ---------------------------------------------------------------------------

/// Vertical flex container with `gap` between children (in pixels).
pub fn vstack(gap: f32) -> impl Bundle {
    Node {
        flex_direction: FlexDirection::Column,
        row_gap: Val::Px(gap),
        ..default()
    }
}

/// Horizontal flex container with `gap` between children (in pixels).
pub fn hstack(gap: f32) -> impl Bundle {
    Node {
        flex_direction: FlexDirection::Row,
        column_gap: Val::Px(gap),
        align_items: AlignItems::Center,
        ..default()
    }
}

/// Full-viewport centered overlay (used for menus, modals).
pub fn fullscreen_center() -> impl Bundle {
    Node {
        width: Val::Percent(100.0),
        height: Val::Percent(100.0),
        flex_direction: FlexDirection::Column,
        align_items: AlignItems::Center,
        justify_content: JustifyContent::Center,
        row_gap: Val::Px(space::XL),
        ..default()
    }
}

/// Dimmed full-viewport scrim (for pause/modal backgrounds).
pub fn scrim() -> impl Bundle {
    (fullscreen_center(), BackgroundColor(palette::OVERLAY_SCRIM))
}

/// 1px horizontal divider line.
pub fn divider() -> impl Bundle {
    (
        Node {
            height: Val::Px(border::THIN),
            width: Val::Percent(100.0),
            margin: UiRect::vertical(Val::Px(space::MD)),
            ..default()
        },
        BackgroundColor(palette::BORDER),
    )
}

// ---------------------------------------------------------------------------
// Panel surfaces
// ---------------------------------------------------------------------------

/// Style of the [`panel`] surface.
#[derive(Copy, Clone, Default)]
pub enum PanelStyle {
    /// Solid `P1` surface with thin border. Default for in-game panels.
    #[default]
    Surface,
    /// Translucent dark scrim with thin border. For overlays on the 3D world.
    Scrim,
    /// Solid `P2` surface, slightly lighter (nested sections).
    Nested,
}

/// Bordered panel surface with theme padding.
pub fn panel() -> impl Bundle {
    panel_styled(PanelStyle::Surface)
}

pub fn panel_styled(style: PanelStyle) -> impl Bundle {
    let (bg, border_color) = match style {
        PanelStyle::Surface => (palette::P1, palette::BORDER),
        PanelStyle::Scrim => (palette::OVERLAY_SCRIM, palette::BORDER),
        PanelStyle::Nested => (palette::P2, palette::BORDER),
    };
    (
        Node {
            flex_direction: FlexDirection::Column,
            border: UiRect::all(Val::Px(border::THIN)),
            padding: UiRect::all(Val::Px(space::XL)),
            border_radius: BorderRadius::all(Val::Px(radius::LG)),
            ..default()
        },
        BackgroundColor(bg),
        BorderColor::all(border_color),
    )
}

// ---------------------------------------------------------------------------
// Text
// ---------------------------------------------------------------------------

/// Heading size — picks font size from the theme scale.
#[derive(Copy, Clone)]
pub enum H {
    Xl,
    Lg,
    Md,
    Sm,
    Xs,
}

impl H {
    pub fn size(self) -> f32 {
        match self {
            H::Xl => font_size::H_XL,
            H::Lg => font_size::H_LG,
            H::Md => font_size::H_MD,
            H::Sm => font_size::H_SM,
            H::Xs => font_size::H_XS,
        }
    }
}

/// Heading text. Accent-colored by default; override with `.0.color` if needed.
pub fn heading(text: impl Into<String>, size: H) -> impl Bundle {
    (
        Text::new(text.into()),
        TextFont {
            font_size: FontSize::Px(size.size()),
            ..default()
        },
        TextColor(palette::ACCENT),
    )
}

/// Body label text. `TEXT` colored.
pub fn label(text: impl Into<String>) -> impl Bundle {
    (
        Text::new(text.into()),
        TextFont {
            font_size: FontSize::Px(font_size::LABEL),
            ..default()
        },
        TextColor(palette::TEXT),
    )
}

/// Dim label, used for captions and section sub-headings.
pub fn caption(text: impl Into<String>) -> impl Bundle {
    (
        Text::new(text.into()),
        TextFont {
            font_size: FontSize::Px(font_size::LABEL_SM),
            ..default()
        },
        TextColor(palette::DIM),
    )
}

// ---------------------------------------------------------------------------
// Tag chip
// ---------------------------------------------------------------------------

#[derive(Copy, Clone, Default)]
pub enum TagVariant {
    #[default]
    Default,
    On,
    Accent,
}

/// Small uppercase chip (like `sk-tag` in the css mock). Build the chip
/// node, then add a `tag_text` child if you want the inner label.
pub fn tag(variant: TagVariant) -> impl Bundle {
    let (bg, fg, border_color) = match variant {
        TagVariant::Default => (
            Color::srgba(1.0, 1.0, 1.0, 0.04),
            palette::DIM,
            palette::BORDER,
        ),
        TagVariant::On => (
            Color::srgba(0.541, 0.447, 0.667, 0.18),
            palette::ACCENT,
            Color::srgba(0.541, 0.447, 0.667, 0.35),
        ),
        TagVariant::Accent => (palette::ACCENT, Color::WHITE, Color::NONE),
    };
    (
        Node {
            padding: UiRect::axes(Val::Px(space::MD), Val::Px(space::XS)),
            border: UiRect::all(Val::Px(border::THIN)),
            border_radius: BorderRadius::all(Val::Px(radius::SM)),
            ..default()
        },
        BackgroundColor(bg),
        BorderColor::all(border_color),
        TagTextColor(fg),
    )
}

/// Companion text bundle for the inside of a [`tag`].
pub fn tag_text(text: impl Into<String>, color: Color) -> impl Bundle {
    (
        Text::new(text.into().to_uppercase()),
        TextFont {
            font_size: FontSize::Px(font_size::TAG),
            ..default()
        },
        TextColor(color),
    )
}

/// Carries the foreground color a [`tag`] expects its child text to use.
/// Useful when spawning children dynamically.
#[derive(Component, Copy, Clone)]
pub struct TagTextColor(pub Color);

// ---------------------------------------------------------------------------
// Progress bar
// ---------------------------------------------------------------------------

/// Bar track — spawn the fill as a child node sized by percent.
pub fn bar_track(height: f32) -> impl Bundle {
    (
        Node {
            width: Val::Percent(100.0),
            height: Val::Px(height),
            border_radius: BorderRadius::all(Val::Px(radius::MD)),
            ..default()
        },
        BackgroundColor(palette::P3),
    )
}

/// Bar fill. Spawn as a child of [`bar_track`] and update `Node::width`.
pub fn bar_fill(percent: f32, color: Color) -> impl Bundle {
    (
        Node {
            width: Val::Percent(percent.clamp(0.0, 100.0)),
            height: Val::Percent(100.0),
            border_radius: BorderRadius::all(Val::Px(radius::MD)),
            ..default()
        },
        BackgroundColor(color),
    )
}

// ---------------------------------------------------------------------------
// Slot (hotbar / inventory)
// ---------------------------------------------------------------------------

#[derive(Copy, Clone, Default)]
pub enum SlotState {
    #[default]
    Empty,
    Filled,
    Active,
}

/// Fixed-size square slot.
pub fn slot(size: f32, state: SlotState) -> impl Bundle {
    let (bg, border_color, border_w) = match state {
        SlotState::Empty => (palette::P2, palette::BORDER, border::THIN),
        SlotState::Filled => (palette::P2, palette::BORDER_STRONG, border::THIN),
        SlotState::Active => (palette::P2, palette::ACCENT, border::THIN),
    };
    (
        Node {
            width: Val::Px(size),
            height: Val::Px(size),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            border: UiRect::all(Val::Px(border_w)),
            padding: UiRect::all(Val::Px(space::MD)),
            border_radius: BorderRadius::all(Val::Px(radius::MD)),
            ..default()
        },
        BackgroundColor(bg),
        BorderColor::all(border_color),
    )
}
