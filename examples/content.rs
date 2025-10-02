//! A demonstration of some tooltip content options.

use bevy::color::palettes::tailwind::*;
use bevy::prelude::*;
use bevy::sprite::Anchor;
use bevy::ui::Val::*;
use pyri_tooltip::prelude::*;

fn main() -> AppExit {
    App::new()
        .add_plugins((DefaultPlugins, TooltipPlugin::default()))
        .add_systems(Startup, spawn_scene)
        .add_systems(Update, highlight_hovered_tile)
        .run()
}

fn spawn_scene(mut commands: Commands) {
    commands.spawn(Camera2d);

    let custom_content = commands
        .spawn((
            Node {
                align_items: AlignItems::Center,
                padding: UiRect::all(Px(8.0)),
                border: UiRect::all(Px(4.0)),
                flex_direction: FlexDirection::Column,
                row_gap: Px(8.0),
                ..default()
            },
            Transform::default(), // Required for tooltip positioning
            BackgroundColor(GRAY_700.into()),
            BorderColor::all(Color::WHITE),
            BorderRadius::all(Px(8.0)),
            // Hide the tooltip content initially.
            Visibility::Hidden,
            // Display the tooltip content in front of its target.
            ZIndex(1),
            children![
                Text::new("TooltipContent::Custom(entity)"),
                (
                    Node {
                        width: Px(64.0),
                        height: Px(64.0),
                        ..default()
                    },
                    BackgroundColor(RED_600.into()),
                ),
            ],
        ))
        .id();

    commands.spawn((
        Node {
            align_self: AlignSelf::Center,
            justify_self: JustifySelf::Center,
            column_gap: Px(8.0),
            ..default()
        },
        Transform::default(), // Required for tooltip positioning
        children![tile("TooltipContent::Primary(text)"), tile(custom_content)],
    ));
}

fn tile(content: impl Into<TooltipContent>) -> impl Bundle {
    (
        Node {
            width: Px(64.0),
            height: Px(64.0),
            border: UiRect::all(Px(4.0)),
            ..default()
        },
        BackgroundColor(Color::WHITE),
        BorderColor::all(Color::BLACK),
        BorderRadius::all(Px(8.0)),
        Transform::default(), // Required for tooltip positioning
        Tooltip::fixed(Anchor::TOP_CENTER, content),
    )
}

fn highlight_hovered_tile(mut tile_query: Query<(&Interaction, &mut BackgroundColor)>) {
    for (interaction, mut background_color) in &mut tile_query {
        background_color.0 = match interaction {
            Interaction::None => Color::NONE,
            _ => Color::WHITE,
        }
    }
}
