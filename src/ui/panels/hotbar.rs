use bevy::prelude::*;

use crate::{
    GameState,
    inventory::{Hotbar, ItemRegistry},
    logistics::StorageUnit,
    ui::theme::COLOR_OVERLAY_BG,
};

#[derive(Component)]
struct HotbarRoot;

/// Marks a slot border node with its slot index.
#[derive(Component)]
struct HotbarSlot(usize);

/// Marks the text label inside a slot.
#[derive(Component)]
struct HotbarSlotText(usize);

pub fn plugin(app: &mut App) {
    app.add_systems(OnEnter(GameState::Playing), spawn)
        .add_systems(Update, update.run_if(in_state(GameState::Playing)));
}

fn spawn(mut commands: Commands) {
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                position_type: PositionType::Absolute,
                bottom: Val::Px(10.0),
                justify_content: JustifyContent::Center,
                ..default()
            },
            Pickable::IGNORE,
            DespawnOnExit(GameState::Playing),
            HotbarRoot,
        ))
        .with_children(|root| {
            for i in 0..9usize {
                root.spawn((
                    Node {
                        width: Val::Px(96.0),
                        height: Val::Px(96.0),
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        border: UiRect::all(Val::Px(2.0)),
                        margin: UiRect::horizontal(Val::Px(2.0)),
                        padding: UiRect::all(Val::Px(6.0)),
                        ..default()
                    },
                    BorderColor::all(Color::srgb(0.471, 0.471, 0.471)),
                    BackgroundColor(COLOR_OVERLAY_BG),
                    Pickable::IGNORE,
                    HotbarSlot(i),
                ))
                .with_child((
                    Text::new(format!("{}", i + 1)),
                    TextFont {
                        font_size: 14.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.627, 0.627, 0.627)),
                    Pickable::IGNORE,
                    HotbarSlotText(i),
                ));
            }
        });
}

fn update(
    hotbar: Option<Res<Hotbar>>,
    item_registry: Option<Res<ItemRegistry>>,
    storage_q: Query<&StorageUnit>,
    mut slot_q: Query<(&HotbarSlot, &mut BorderColor)>,
    mut text_q: Query<(&HotbarSlotText, &mut Text, &mut TextColor)>,
) {
    let Some(hotbar) = hotbar else { return };

    for (slot, mut border) in &mut slot_q {
        let selected = slot.0 == hotbar.selected;
        *border = if selected {
            BorderColor::all(Color::srgb(1.0, 0.863, 0.196))
        } else {
            BorderColor::all(Color::srgb(0.471, 0.471, 0.471))
        };
    }

    for (slot_text, mut text, mut color) in &mut text_q {
        let i = slot_text.0;
        match hotbar.slots.get(i).and_then(|s| s.as_ref()) {
            Some(s) => {
                let name = item_registry
                    .as_ref()
                    .and_then(|r| r.get(&s.item_id))
                    .map_or(s.item_id.as_str(), |d| d.name.as_str());
                let count: u32 = storage_q
                    .iter()
                    .filter_map(|u| u.items.get(&s.item_id))
                    .sum();
                **text = format!("{}\n×{}", name, count);
                *color = TextColor(Color::WHITE);
            }
            None => {
                **text = format!("{}", i + 1);
                *color = TextColor(Color::srgb(0.627, 0.627, 0.627));
            }
        }
    }
}
