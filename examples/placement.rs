//! A demonstration of some tooltip placement options.

use bevy::prelude::*;
use bevy::sprite::Anchor;
use bevy::ui::Val::*;
use pyri_tooltip::prelude::*;

fn main() {
    let mut app = App::new();
    app.add_plugins((DefaultPlugins, TooltipPlugin::default()));
    app.add_systems(Startup, spawn_scene);
    app.add_systems(Update, highlight_hovered_tile);
    app.run();
}

fn spawn_scene(mut commands: Commands) {
    commands.spawn(Camera2d::default());
    commands
        .spawn(Node {
            display: Display::Grid,
            align_self: AlignSelf::Center,
            justify_self: JustifySelf::Center,
            row_gap: Px(8.0),
            column_gap: Px(8.0),
            grid_template_columns: RepeatedGridTrack::auto(3),
            ..default()
        })
        .with_children(|parent| {
            let tile = (
                Node {
                    width: Px(64.0),
                    height: Px(64.0),
                    border: UiRect::all(Px(4.0)),
                    ..default()
                },
                BackgroundColor(Color::WHITE),
                BorderColor(Color::BLACK),
                BorderRadius::all(Px(8.0)),
            );

            // Demonstrate fixed placement.
            for anchor in [
                Anchor::TopLeft,
                Anchor::TopCenter,
                Anchor::TopRight,
                Anchor::CenterLeft,
                Anchor::Center,
                Anchor::CenterRight,
                Anchor::BottomLeft,
                Anchor::BottomCenter,
                Anchor::BottomRight,
            ] {
                parent.spawn((
                    tile.clone(),
                    Tooltip::fixed(anchor, format!("Tooltip::fixed({:?}, text)", anchor)),
                ));
            }

            // Demonstrate cursor placement.
            parent.spawn((tile.clone(), Tooltip::cursor("Tooltip::cursor(text)")));
        });
}

fn highlight_hovered_tile(mut tile_query: Query<(&Interaction, &mut BackgroundColor)>) {
    for (interaction, mut background_color) in &mut tile_query {
        background_color.0 = match interaction {
            Interaction::None => Color::NONE,
            _ => Color::WHITE,
        }
    }
}
