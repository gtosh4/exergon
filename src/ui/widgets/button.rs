//! Themed button widget.
//!
//! Add a [`UiButton`] component to a node and the required Bevy UI
//! components (`Button`, `Node`, `BackgroundColor`, `BorderColor`,
//! `BorderRadius`) are auto-attached via `#[require]`. A system tracks
//! `Interaction` changes and repaints to match the variant.
//!
//! ```ignore
//! parent.spawn((UiButton::accent(), children![ button_label("LAND") ]));
//! ```

use bevy::prelude::*;

use crate::ui::theme::{border, font_size, palette, radius, space};

/// Visual variant of a [`UiButton`].
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub enum ButtonVariant {
    /// Outlined, faint background. Use for secondary actions.
    #[default]
    Default,
    /// Solid accent fill. Use for the primary action on a screen.
    Accent,
    /// Outlined like Default; flips to accent fill when `pressed=true`.
    Toggle,
    /// Borderless / no fill. Use for close buttons and icon affordances.
    Ghost,
}

/// Themed button. Required components attach the UI plumbing automatically;
/// the [`paint_buttons`] system keeps colors in sync with `Interaction`.
#[derive(Component, Copy, Clone, Debug)]
#[require(
    Button,
    Node = button_node(),
    BackgroundColor,
    BorderColor,
)]
pub struct UiButton {
    pub variant: ButtonVariant,
    /// Only meaningful for `Toggle` variant.
    pub pressed: bool,
}

impl Default for UiButton {
    fn default() -> Self {
        Self {
            variant: ButtonVariant::Default,
            pressed: false,
        }
    }
}

impl UiButton {
    pub const fn new(variant: ButtonVariant) -> Self {
        Self {
            variant,
            pressed: false,
        }
    }

    pub const fn accent() -> Self {
        Self::new(ButtonVariant::Accent)
    }

    pub const fn ghost() -> Self {
        Self::new(ButtonVariant::Ghost)
    }

    pub const fn toggle(pressed: bool) -> Self {
        Self {
            variant: ButtonVariant::Toggle,
            pressed,
        }
    }
}

fn button_node() -> Node {
    Node {
        padding: UiRect::axes(Val::Px(space::LG), Val::Px(space::SM + 1.0)),
        border: UiRect::all(Val::Px(border::THIN)),
        align_items: AlignItems::Center,
        justify_content: JustifyContent::Center,
        border_radius: BorderRadius::all(Val::Px(radius::MD)),
        ..default()
    }
}

/// Convenience: standard label bundle for use as a child of [`UiButton`].
pub fn button_label(text: impl Into<String>) -> impl Bundle {
    (
        Text::new(text.into()),
        TextFont {
            font_size: font_size::BUTTON,
            ..default()
        },
        TextColor(palette::TEXT),
    )
}

#[derive(Copy, Clone, Debug, PartialEq)]
struct Paint {
    bg: Color,
    border: Color,
}

impl Paint {
    const fn new(bg: Color, border: Color) -> Self {
        Self { bg, border }
    }
}

fn paint_for(btn: &UiButton, interaction: Interaction) -> Paint {
    match (btn.variant, interaction) {
        (ButtonVariant::Default, Interaction::None) => {
            Paint::new(Color::srgba(1.0, 1.0, 1.0, 0.05), palette::BORDER)
        }
        (ButtonVariant::Default, Interaction::Hovered) => {
            Paint::new(palette::HOVER, palette::BORDER_STRONG)
        }
        (ButtonVariant::Default, Interaction::Pressed) => {
            Paint::new(palette::PRESSED, palette::BORDER_STRONG)
        }

        (ButtonVariant::Accent, Interaction::None) => Paint::new(palette::ACCENT, Color::NONE),
        (ButtonVariant::Accent, Interaction::Hovered) => {
            Paint::new(palette::ACCENT_HOVER, Color::NONE)
        }
        (ButtonVariant::Accent, Interaction::Pressed) => {
            Paint::new(palette::ACCENT_HOVER, Color::NONE)
        }

        (ButtonVariant::Toggle, interaction) => {
            if btn.pressed {
                paint_for(
                    &UiButton::new(ButtonVariant::Accent),
                    if interaction == Interaction::None {
                        Interaction::None
                    } else {
                        Interaction::Hovered
                    },
                )
            } else {
                paint_for(&UiButton::new(ButtonVariant::Default), interaction)
            }
        }

        (ButtonVariant::Ghost, Interaction::None) => Paint::new(Color::NONE, Color::NONE),
        (ButtonVariant::Ghost, Interaction::Hovered) => Paint::new(palette::HOVER, Color::NONE),
        (ButtonVariant::Ghost, Interaction::Pressed) => Paint::new(palette::PRESSED, Color::NONE),
    }
}

/// Repaints buttons whose [`Interaction`] or [`UiButton`] changed. Also runs
/// on insert (the `Added<UiButton>` branch) so freshly spawned buttons get
/// their resting style without needing the cursor to enter them first.
pub(super) fn paint_buttons(
    mut q: Query<
        (
            &UiButton,
            &Interaction,
            &mut BackgroundColor,
            &mut BorderColor,
        ),
        Or<(Changed<Interaction>, Changed<UiButton>, Added<UiButton>)>,
    >,
) {
    for (btn, interaction, mut bg, mut border) in &mut q {
        let p = paint_for(btn, *interaction);
        *bg = BackgroundColor(p.bg);
        *border = BorderColor::all(p.border);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_button_resting_paint() {
        let p = paint_for(&UiButton::default(), Interaction::None);
        assert_eq!(p.bg, Color::srgba(1.0, 1.0, 1.0, 0.05));
        assert_eq!(p.border, palette::BORDER);
    }

    #[test]
    fn accent_button_hover_brightens() {
        let resting = paint_for(&UiButton::accent(), Interaction::None);
        let hovered = paint_for(&UiButton::accent(), Interaction::Hovered);
        assert_ne!(resting.bg, hovered.bg);
        assert_eq!(hovered.bg, palette::ACCENT_HOVER);
    }

    #[test]
    fn toggle_when_pressed_uses_accent_paint() {
        let off = paint_for(&UiButton::toggle(false), Interaction::None);
        let on = paint_for(&UiButton::toggle(true), Interaction::None);
        assert_ne!(off.bg, on.bg);
        assert_eq!(on.bg, palette::ACCENT);
    }

    #[test]
    fn ghost_resting_is_transparent() {
        let p = paint_for(&UiButton::ghost(), Interaction::None);
        assert_eq!(p.bg, Color::NONE);
        assert_eq!(p.border, Color::NONE);
    }

    #[test]
    fn paint_system_repaints_on_interaction_change() {
        let mut app = App::new();
        app.add_systems(Update, paint_buttons);

        let id = app
            .world_mut()
            .spawn((UiButton::default(), Interaction::None))
            .id();

        app.update();
        let bg_initial = app.world().get::<BackgroundColor>(id).unwrap().0;

        app.world_mut().entity_mut(id).insert(Interaction::Hovered);
        app.update();
        let bg_hovered = app.world().get::<BackgroundColor>(id).unwrap().0;

        assert_ne!(bg_initial, bg_hovered);
    }

    #[test]
    fn paint_system_reacts_to_toggle_flip() {
        let mut app = App::new();
        app.add_systems(Update, paint_buttons);

        let id = app
            .world_mut()
            .spawn((UiButton::toggle(false), Interaction::None))
            .id();

        app.update();
        let bg_off = app.world().get::<BackgroundColor>(id).unwrap().0;

        *app.world_mut().get_mut::<UiButton>(id).unwrap() = UiButton::toggle(true);
        app.update();
        let bg_on = app.world().get::<BackgroundColor>(id).unwrap().0;

        assert_ne!(bg_off, bg_on);
        assert_eq!(bg_on, palette::ACCENT);
    }
}
