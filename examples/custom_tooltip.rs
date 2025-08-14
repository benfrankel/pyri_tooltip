//! A demonstration of a custom integrated tooltip.

use bevy::prelude::*;
use bevy::ui::Val::*;
use pyri_tooltip::prelude::*;

fn main() -> AppExit {
    App::new()
        .add_plugins((DefaultPlugins, TooltipPlugin::default()))
        .add_systems(Startup, spawn_scene)
        .run()
}

fn spawn_scene(mut commands: Commands) {
    commands.spawn(Camera2d);

    let ui_tooltip = {
        let gray = Color::linear_rgba(0.2, 0.2, 0.2, 1.0);
        let light_red = Color::linear_rgba(1.0, 0.2, 0.2, 1.0);

        commands
            .spawn((
                Node {
                    width: Px(200.0),
                    height: Val::Auto,
                    border: UiRect::all(Px(4.0)),
                    display: Display::Flex,
                    flex_direction: FlexDirection::Column,
                    row_gap: Px(8.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    padding: UiRect::all(Px(8.0)),
                    ..default()
                },
                // Important to set the tooltip visibility to hidden initially.
                Visibility::Hidden,
                BackgroundColor(gray),
                BorderColor(Color::WHITE),
                BorderRadius::all(Px(8.0)),
                children![
                    (Text::new("A custom tooltip"),),
                    (
                        Node {
                            width: Px(64.0),
                            height: Px(64.0),
                            ..default()
                        },
                        BackgroundColor(light_red),
                    ),
                ],
            ))
            .id()
    };

    commands.spawn((
        Node {
            display: Display::Grid,
            align_self: AlignSelf::Center,
            justify_self: JustifySelf::Center,
            ..default()
        },
        children![(
            Node {
                width: Px(64.0),
                height: Px(64.0),
                border: UiRect::all(Px(4.0)),
                ..default()
            },
            BackgroundColor(Color::WHITE),
            BorderColor(Color::BLACK),
            BorderRadius::all(Px(8.0)),
            Tooltip::cursor(ui_tooltip).with_activation(TooltipActivation::IMMEDIATE),
        )],
    ));
}
