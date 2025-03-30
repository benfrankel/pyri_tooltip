#![allow(missing_docs)]

use bevy_app::{App, PostUpdate};
use bevy_asset::Handle;
use bevy_color::Color;
use bevy_ecs::{
    component::Component,
    entity::Entity,
    hierarchy::Children,
    query::{Changed, With},
    schedule::{IntoScheduleConfigs as _, SystemSet},
    system::{Commands, Query},
};
use bevy_text::{Font, JustifyText, LineBreak, TextColor, TextFont, TextLayout, TextSpan};
use bevy_ui::{UiSystem, widget::Text};

pub(super) fn plugin(app: &mut App) {
    app.configure_sets(PostUpdate, RichTextSystems.before(UiSystem::Prepare));
    app.add_systems(PostUpdate, sync_rich_text_spans.in_set(RichTextSystems));
}

/// A system set for the systems that update rich text entities in `PostUpdate`.
#[derive(SystemSet, Eq, PartialEq, Hash, Clone, Debug)]
pub struct RichTextSystems;

fn sync_rich_text_spans(
    mut commands: Commands,
    rich_text_query: Query<(Entity, Option<&Children>, &RichText), Changed<RichText>>,
    text_span_query: Query<(), With<TextSpan>>,
) {
    for (entity, children, rich_text) in &rich_text_query {
        // Update root text entity.
        commands.entity(entity).insert((
            Text::default(),
            TextLayout::new(rich_text.justify, rich_text.linebreak_behavior),
        ));

        // Update text span child entities.
        let mut section_idx = 0;
        for &child in children.into_iter().flatten() {
            // Skip children that aren't text spans.
            if !text_span_query.contains(child) {
                continue;
            }

            // Despawn text spans when there are no sections left to write.
            if section_idx == rich_text.sections.len() {
                commands.entity(child).despawn();
                continue;
            }

            // Update text spans when there are still sections left to write.
            let section = &rich_text.sections[section_idx];
            commands.entity(child).insert((
                TextSpan(section.value.clone()),
                TextColor(section.style.color),
                TextFont {
                    font: section.style.font.clone(),
                    font_size: section.style.font_size,
                    ..Default::default()
                },
            ));
            section_idx += 1;
        }

        // If all sections are written, we're done.
        if section_idx == rich_text.sections.len() {
            continue;
        }

        // Otherwise, spawn new text spans for the remaining sections.
        commands.entity(entity).with_children(|parent| {
            for section in &rich_text.sections[section_idx..] {
                parent.spawn((
                    TextSpan(section.value.clone()),
                    TextColor(section.style.color),
                    TextFont {
                        font: section.style.font.clone(),
                        font_size: section.style.font_size,
                        ..Default::default()
                    },
                ));
            }
        });
    }
}

/// A rich text string in the shape of Bevy 0.14's `Text` component.
#[derive(Component, Clone, Default, Debug)]
#[require(Text)]
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
