use bevy::prelude::*;

use crate::{GameState, inventory::InventoryOpen};

#[derive(Component)]
struct CrosshairRoot;

pub fn plugin(app: &mut App) {
    app.add_systems(OnEnter(GameState::Playing), spawn)
        .add_systems(Update, sync_visibility.run_if(in_state(GameState::Playing)));
}

fn spawn(mut commands: Commands) {
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                position_type: PositionType::Absolute,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            Pickable::IGNORE,
            DespawnOnExit(GameState::Playing),
            CrosshairRoot,
        ))
        .with_child((
            Node {
                width: Val::Px(6.0),
                height: Val::Px(6.0),
                border_radius: BorderRadius::all(Val::Percent(50.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.706, 0.706, 0.706, 0.471)),
            Pickable::IGNORE,
        ));
}

fn sync_visibility(
    inv_open: Option<Res<InventoryOpen>>,
    mut q: Query<&mut Visibility, With<CrosshairRoot>>,
) {
    let vis = if inv_open.is_some_and(|o| o.0) {
        Visibility::Hidden
    } else {
        Visibility::Inherited
    };
    for mut v in &mut q {
        *v = vis;
    }
}
