//! A demonstration of some tooltip placement options.

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
    commands.spawn((
        Node {
            display: Display::Grid,
            align_self: AlignSelf::Center,
            justify_self: JustifySelf::Center,
            row_gap: Px(8.0),
            column_gap: Px(8.0),
            grid_template_columns: RepeatedGridTrack::auto(3),
            ..default()
        },
        Transform::default(), // Required for tooltip positioning
        children![
            // Demonstrate fixed placement.
            tile_fixed("top_left", Anchor::TOP_LEFT),
            tile_fixed("top_center", Anchor::TOP_CENTER),
            tile_fixed("top_right", Anchor::TOP_RIGHT),
            tile_fixed("center_left", Anchor::CENTER_LEFT),
            tile_fixed("center", Anchor::CENTER),
            tile_fixed("center_right", Anchor::CENTER_RIGHT),
            tile_fixed("bottom_left", Anchor::BOTTOM_LEFT),
            tile_fixed("bottom_center", Anchor::BOTTOM_CENTER),
            tile_fixed("bottom_right", Anchor::BOTTOM_RIGHT),
            // Demonstrate cursor placement.
            tile(Tooltip::cursor("Tooltip::cursor(text)")),
            // Demonstrate follow cursor placement.
            tile(Tooltip::follow_cursor("Tooltip::follow_cursor(text)")),
        ],
    ));
}

fn tile_fixed(anchor_title: &str, anchor: Anchor) -> impl Bundle {
    tile(Tooltip::fixed(
        anchor,
        format!("Tooltip::fixed({anchor_title:?}, text)"),
    ))
}

fn tile(tooltip: Tooltip) -> impl Bundle {
    (
        Node {
            width: Px(64.0),
            height: Px(64.0),
            border: UiRect::all(Px(4.0)),
            ..default()
        },
        Transform::default(), // Required for tooltip positioning
        BackgroundColor(Color::WHITE),
        BorderColor::all(Color::BLACK),
        BorderRadius::all(Px(8.0)),
        tooltip,
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
