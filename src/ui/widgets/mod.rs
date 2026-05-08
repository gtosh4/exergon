use bevy::{
    input::mouse::{MouseScrollUnit, MouseWheel},
    picking::hover::HoverMap,
    prelude::*,
};

pub struct WidgetsPlugin;

impl Plugin for WidgetsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, forward_scroll_events)
            .add_observer(on_scroll);
    }
}

/// Mark a Node with `ScrollableContent` to enable mouse-wheel scrolling.
#[derive(Component)]
pub struct ScrollableContent;

/// Internal scroll event forwarded to scrollable nodes under the cursor.
#[derive(EntityEvent, Debug)]
#[entity_event(propagate, auto_propagate)]
pub struct ScrollEvent {
    pub entity: Entity,
    pub delta: Vec2,
}

fn forward_scroll_events(
    mut wheel: MessageReader<MouseWheel>,
    hover_map: Res<HoverMap>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
) {
    for ev in wheel.read() {
        let mut delta = -Vec2::new(ev.x, ev.y);
        if ev.unit == MouseScrollUnit::Line {
            delta *= 21.0;
        }
        if keyboard.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]) {
            std::mem::swap(&mut delta.x, &mut delta.y);
        }
        for pointer_map in hover_map.values() {
            for entity in pointer_map.keys().copied() {
                commands.trigger(ScrollEvent { entity, delta });
            }
        }
    }
}

fn on_scroll(
    mut ev: On<ScrollEvent>,
    mut q: Query<(&mut ScrollPosition, &Node, &ComputedNode), With<ScrollableContent>>,
) {
    let Ok((mut pos, node, computed)) = q.get_mut(ev.entity) else {
        return;
    };
    let max = (computed.content_size() - computed.size()) * computed.inverse_scale_factor();
    let delta = &mut ev.delta;
    if node.overflow.x == OverflowAxis::Scroll && delta.x != 0.0 {
        let at_max = if delta.x > 0.0 {
            pos.x >= max.x
        } else {
            pos.x <= 0.0
        };
        if !at_max {
            pos.x += delta.x;
            delta.x = 0.0;
        }
    }
    if node.overflow.y == OverflowAxis::Scroll && delta.y != 0.0 {
        let at_max = if delta.y > 0.0 {
            pos.y >= max.y
        } else {
            pos.y <= 0.0
        };
        if !at_max {
            pos.y += delta.y;
            delta.y = 0.0;
        }
    }
    if *delta == Vec2::ZERO {
        ev.propagate(false);
    }
}
