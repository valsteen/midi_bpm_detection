use eframe::egui::{Context, Event, Key, KeyboardShortcut, Modifiers, RawInput, ViewportCommand, ViewportId};

use crate::{GuiLifecycleOwner, create_gui_shell};

fn quit_shortcut() -> KeyboardShortcut {
    KeyboardShortcut::new(Modifiers::COMMAND, Key::Q)
}

fn press_quit_shortcut(context: &Context) -> bool {
    let output = context.run_ui(
        RawInput {
            events: vec![Event::Key {
                key: Key::Q,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: Modifiers::COMMAND,
            }],
            ..Default::default()
        },
        |_| {},
    );

    output
        .viewport_output
        .get(&ViewportId::ROOT)
        .is_some_and(|viewport| viewport.commands.contains(&ViewportCommand::Close))
}

#[test]
fn application_owned_gui_can_request_application_quit() {
    let context = Context::default();
    context.options_mut(|options| options.quit_shortcuts = vec![quit_shortcut()]);

    let (_, builder) = create_gui_shell(GuiLifecycleOwner::ApplicationRuntime);
    let _app = builder.with_config(()).build(context.clone());

    assert!(press_quit_shortcut(&context));
}

#[test]
fn parent_owned_gui_cannot_request_application_quit() {
    let context = Context::default();
    context.options_mut(|options| options.quit_shortcuts = vec![quit_shortcut()]);

    let (_, builder) = create_gui_shell(GuiLifecycleOwner::ParentRuntime);
    let _app = builder.with_config(()).build(context.clone());

    assert!(!press_quit_shortcut(&context));
}
