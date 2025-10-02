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
        children![
            // Demonstrate fixed placement.
            tile_fixed(Anchor::TOP_LEFT, "TOP_LEFT"),
            tile_fixed(Anchor::TOP_CENTER, "TOP_CENTER"),
            tile_fixed(Anchor::TOP_RIGHT, "TOP_RIGHT"),
            tile_fixed(Anchor::CENTER_LEFT, "CENTER_LEFT"),
            tile_fixed(Anchor::CENTER, "CENTER"),
            tile_fixed(Anchor::CENTER_RIGHT, "CENTER_RIGHT"),
            tile_fixed(Anchor::BOTTOM_LEFT, "BOTTOM_LEFT"),
            tile_fixed(Anchor::BOTTOM_CENTER, "BOTTOM_CENTER"),
            tile_fixed(Anchor::BOTTOM_RIGHT, "BOTTOM_RIGHT"),
            // Demonstrate cursor placement.
            tile(Tooltip::cursor("Tooltip::cursor(text)")),
            // Demonstrate follow cursor placement.
            tile(Tooltip::follow_cursor("Tooltip::follow_cursor(text)")),
        ],
    ));
}

fn tile_fixed(anchor: Anchor, anchor_str: &str) -> impl Bundle {
    tile(Tooltip::fixed(
        anchor,
        format!("Tooltip::fixed(Anchor::{anchor_str}, text)"),
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
