#![allow(missing_docs)]

use bevy_app::{App, PostUpdate};
use bevy_asset::Handle;
use bevy_color::Color;
use bevy_ecs::{
    component::Component,
    entity::Entity,
    query::{Changed, With},
    system::{Commands, Query},
};
use bevy_hierarchy::{BuildChildren as _, ChildBuild as _, Children, DespawnRecursiveExt as _};
use bevy_text::{
    Font, FontSmoothing, JustifyText, LineBreak, TextColor, TextFont, TextLayout, TextSpan,
};
use bevy_ui::widget::Text;

pub(super) fn plugin(app: &mut App) {
    app.add_systems(PostUpdate, sync_rich_text_spans);
}

fn sync_rich_text_spans(
    mut commands: Commands,
    rich_text_query: Query<(Entity, Option<&Children>, &RichText), Changed<RichText>>,
    text_span_query: Query<(), With<TextSpan>>,
) {
    for (entity, children, rich_text) in &rich_text_query {
        // Despawn old `TextSpan` children.
        for &child in children.into_iter().flatten() {
            if text_span_query.contains(child) {
                commands.entity(child).despawn_recursive();
            }
        }

        commands
            .entity(entity)
            .insert((
                Text::default(),
                TextLayout::new(rich_text.justify, rich_text.linebreak_behavior),
            ))
            .with_children(|parent| {
                // Spawn new `TextSpan` children.
                for section in &rich_text.sections {
                    parent.spawn((
                        TextSpan(section.value.clone()),
                        TextColor(section.style.color),
                        TextFont {
                            font: section.style.font.clone(),
                            font_size: section.style.font_size,
                            font_smoothing: FontSmoothing::AntiAliased,
                        },
                    ));
                }
            });
    }
}

/// A rich text string in the shape of Bevy 0.14's `Text` component.
#[derive(Component, Clone, Default, Debug)]
#[cfg_attr(feature = "bevy_reflect", derive(bevy_reflect::Reflect))]
pub struct RichText {
    pub sections: Vec<TextSection>,
    pub justify: JustifyText,
    pub linebreak_behavior: LineBreak,
}

impl RichText {
    pub fn from_section(value: impl Into<String>, style: TextStyle) -> Self {
        Self {
            sections: vec![TextSection::new(value, style)],
            ..Default::default()
        }
    }

    pub fn from_sections(sections: impl IntoIterator<Item = TextSection>) -> Self {
        Self {
            sections: sections.into_iter().collect(),
            ..Default::default()
        }
    }

    pub const fn with_justify(mut self, justify: JustifyText) -> Self {
        self.justify = justify;
        self
    }

    pub const fn with_no_wrap(mut self) -> Self {
        self.linebreak_behavior = LineBreak::NoWrap;
        self
    }
}

/// A section of `RichText` in the shape of Bevy 0.14's `TextSection`.
#[derive(Clone, Default, Debug)]
#[cfg_attr(feature = "bevy_reflect", derive(bevy_reflect::Reflect))]
pub struct TextSection {
    pub value: String,
    pub style: TextStyle,
}

impl TextSection {
    pub fn new(value: impl Into<String>, style: TextStyle) -> Self {
        Self {
            value: value.into(),
            style,
        }
    }

    pub const fn from_style(style: TextStyle) -> Self {
        Self {
            value: String::new(),
            style,
        }
    }
}

impl From<&str> for TextSection {
    fn from(value: &str) -> Self {
        Self {
            value: value.into(),
            ..Default::default()
        }
    }
}

impl From<String> for TextSection {
    fn from(value: String) -> Self {
        Self {
            value,
            ..Default::default()
        }
    }
}

/// A text style in the shape of Bevy 0.14's `TextStyle`.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "bevy_reflect", derive(bevy_reflect::Reflect))]
pub struct TextStyle {
    pub font: Handle<Font>,
    pub font_size: f32,
    pub color: Color,
}

impl Default for TextStyle {
    fn default() -> Self {
        Self {
            font: Default::default(),
            font_size: 20.0,
            color: Color::WHITE,
        }
    }
}
