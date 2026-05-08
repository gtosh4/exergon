use bevy::{
    input::keyboard::{Key, KeyboardInput},
    prelude::*,
};

/// Marker for the focused text input entity.
#[derive(Resource, Default)]
pub struct FocusedInput(pub Option<Entity>);

/// Single-line text input component. Add to a `Text` entity.
#[derive(Component, Default)]
pub struct TextInput {
    pub value: String,
    /// Set to true for one frame when Enter is pressed.
    pub submitted: bool,
}

pub struct TextInputPlugin;

impl Plugin for TextInputPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FocusedInput>()
            .add_systems(Update, (handle_keyboard, sync_display).chain());
    }
}

fn handle_keyboard(
    focus: Res<FocusedInput>,
    mut inputs: Query<&mut TextInput>,
    mut kb: MessageReader<KeyboardInput>,
) {
    let Some(focused) = focus.0 else {
        kb.clear();
        return;
    };
    let Ok(mut input) = inputs.get_mut(focused) else {
        kb.clear();
        return;
    };
    input.submitted = false;
    for ev in kb.read() {
        if !ev.state.is_pressed() {
            continue;
        }
        match &ev.logical_key {
            Key::Enter => {
                input.submitted = true;
            }
            Key::Backspace => {
                input.value.pop();
            }
            Key::Space => {
                input.value.push(' ');
            }
            Key::Character(ch) => {
                for c in ch.chars() {
                    if !c.is_ascii_control() {
                        input.value.push(c);
                    }
                }
            }
            _ => {}
        }
    }
}

fn sync_display(mut q: Query<(&TextInput, &mut Text), Changed<TextInput>>) {
    for (input, mut text) in &mut q {
        **text = input.value.clone();
    }
}
