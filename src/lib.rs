//! Powerful tooltips for Bevy.
//!
//! # Getting started
//!
//! Import the [prelude] to bring common types into scope:
//!
//! ```
//! use pyri_tooltip::prelude::*;
//! ```
//!
//! Add [`TooltipPlugin`] to set up the tooltip system:
//!
//! ```
//! # use bevy::prelude::*;
//! # use pyri_tooltip::prelude::*;
//! # fn plugin(app: &mut App) {
//! app.add_plugins(TooltipPlugin::default());
//! # }
//! ```
//!
//! Spawn a UI node with the [`Tooltip`] component:
//!
//! ```
//! # use bevy::prelude::*;
//! # use pyri_tooltip::prelude::*;
//! # fn system(mut commands: Commands) {
//! commands.spawn(Tooltip::cursor("Hello, world!"));
//! # }
//! ```
//!
//! # Advanced
//!
//! To customize the behavior and appearance of a tooltip, see [`Tooltip`].
//!
//! To replace the default primary tooltip, see [`TooltipPlugin`] and [`TooltipSettings`].

#![no_std]
// Support configuring Bevy lints within code.
#![cfg_attr(bevy_lint, feature(register_tool), register_tool(bevy))]

extern crate alloc;

mod context;
mod placement;
mod rich_text;

/// Re-exports for commonly used types.
///
/// # Usage
///
/// ```
/// use pyri_tooltip::prelude::*;
/// ```
pub mod prelude {
    pub use super::{
        Tooltip, TooltipActivation, TooltipContent, TooltipPlacement, TooltipPlugin,
        TooltipSettings, TooltipSystems, TooltipTransfer,
        rich_text::{RichText, TextSection, TextStyle},
    };
}

use alloc::{
    string::{String, ToString as _},
    vec::Vec,
};

use bevy_app::{Plugin, PostUpdate, PreUpdate};
use bevy_camera::visibility::Visibility;
use bevy_color::Color;
#[cfg(feature = "bevy_reflect")]
use bevy_ecs::reflect::{ReflectComponent, ReflectResource};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    entity_disabling::Disabled,
    hierarchy::{ChildOf, Children},
    name::Name,
    query::With,
    resource::Resource,
    schedule::{IntoScheduleConfigs as _, SystemSet, common_conditions::resource_changed},
    system::{Commands, Query, Res},
    world::World,
};
use bevy_sprite::Anchor;
use bevy_text::Justify;
use bevy_transform::TransformSystems;
use bevy_ui::{
    BackgroundColor, GlobalZIndex, Interaction, Node, PositionType, UiRect, UiSystems, Val,
};

pub use placement::TooltipPlacement;
pub use rich_text::{RichText, RichTextSystems, TextSection, TextStyle};

/// A [`Plugin`] that sets up the tooltip widget system.
///
/// Use the [`TooltipSettings`] resource to make changes while the app is already running.
pub struct TooltipPlugin {
    /// Set a custom entity for [`TooltipSettings::container`], or spawn the default container
    /// entity if `None`.
    ///
    /// This entity should include all of the required components of [`Node`], with
    /// [`Visibility::Hidden`] and [`Node::position_type`] set to [`PositionType::Absolute`].
    pub container: Entity,
    /// Set a custom entity for [`TooltipSettings::text`], or spawn the default text entity if
    /// `None`.
    ///
    /// This entity should include all of the required components of [`Node`], along with a
    /// [`RichText`] component, and be a child of [`Self::container`].
    pub text: Entity,
    /// Whether or not the tooltip system should initially be enabled.
    pub enabled: bool,
}

impl Plugin for TooltipPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        let settings =
            TooltipSettings::new(app.world_mut(), self.container, self.text, self.enabled);
        app.insert_resource(settings);

        app.configure_sets(
            PreUpdate,
            (
                UiSystems::Focus,
                TooltipSystems::Content.run_if(tooltips_enabled),
            )
                .chain(),
        );
        app.configure_sets(
            PostUpdate,
            (
                TransformSystems::Propagate,
                TooltipSystems::Placement.run_if(tooltips_enabled),
            )
                .chain(),
        );
        app.add_systems(
            PreUpdate,
            sync_tooltip_settings
                .run_if(resource_changed::<TooltipSettings>)
                .before(TooltipSystems::Content),
        );
        app.add_plugins((context::plugin, placement::plugin, rich_text::plugin));
    }
}

impl Default for TooltipPlugin {
    fn default() -> Self {
        Self {
            container: Entity::PLACEHOLDER,
            text: Entity::PLACEHOLDER,
            enabled: true,
        }
    }
}

/// A [`Resource`] containing the [`Entity`] IDs of the global primary tooltip.
///
/// See [`TooltipPlugin`] to set up a custom primary tooltip.
#[derive(Resource, Copy, Clone, Debug)]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(bevy_reflect::Reflect),
    reflect(Resource)
)]
pub struct TooltipSettings {
    /// The [`Entity`] ID of the UI node to be used as the primary tooltip.
    pub container: Entity,
    /// The [`Entity`] ID of the UI node to be used as the primary tooltip's text.
    pub text: Entity,
    /// Whether or not tooltips will be displayed.
    pub enabled: bool,
}

impl TooltipSettings {
    fn new(world: &mut World, container: Entity, text: Entity, enabled: bool) -> Self {
        let container = if container != Entity::PLACEHOLDER {
            container
        } else {
            world
                .spawn((
                    Name::new("PrimaryTooltip"),
                    Node {
                        position_type: PositionType::Absolute,
                        padding: UiRect::all(Val::Px(8.0)),
                        ..Default::default()
                    },
                    BackgroundColor(Color::srgba(0.106, 0.118, 0.122, 0.9)),
                    Visibility::Hidden,
                    GlobalZIndex(999),
                ))
                .id()
        };

        let text = if text != Entity::PLACEHOLDER {
            text
        } else {
            world
                .spawn((
                    Name::new("Text"),
                    Node::default(),
                    RichText::default(),
                    ChildOf(container),
                ))
                .id()
        };

        Self {
            container,
            text,
            enabled,
        }
    }
}

fn sync_tooltip_settings(mut commands: Commands, settings: Res<TooltipSettings>) {
    if settings.enabled {
        commands
            .entity(settings.container)
            .remove_recursive::<Children, Disabled>();
    } else {
        commands
            .entity(settings.container)
            .insert_recursive::<Children>(Disabled);
    }
}

fn tooltips_enabled(
    settings: Res<TooltipSettings>,
    disabled_query: Query<(), With<Disabled>>,
) -> bool {
    settings.enabled && !disabled_query.contains(settings.container)
}

// TODO: Animation, wedge (like a speech bubble), easier content customization / icons.
/// A [`Component`] that specifies a tooltip to be displayed on hover.
#[derive(Component, Clone, Debug)]
#[require(Node, Interaction)]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(bevy_reflect::Reflect),
    reflect(Component)
)]
pub struct Tooltip {
    /// The tooltip content to display.
    pub content: TooltipContent,
    /// How the tooltip will be positioned.
    pub placement: TooltipPlacement,
    /// The conditions for activating the tooltip.
    pub activation: TooltipActivation,
    /// The conditions for dismissing the tooltip.
    pub dismissal: TooltipDismissal,
    /// The conditions for skipping the next tooltip's activation delay.
    pub transfer: TooltipTransfer,
}

impl Tooltip {
    /// Create a new fixed `Tooltip`.
    pub fn fixed(placement: Anchor, content: impl Into<TooltipContent>) -> Self {
        Self {
            content: content.into(),
            placement: placement.into(),
            activation: TooltipActivation::IMMEDIATE,
            dismissal: TooltipDismissal::NONE,
            transfer: TooltipTransfer::SHORT,
        }
    }

    /// Create a new cursor `Tooltip`.
    pub fn cursor(content: impl Into<TooltipContent>) -> Self {
        Self {
            content: content.into(),
            placement: TooltipPlacement::CURSOR,
            activation: TooltipActivation::IDLE,
            dismissal: TooltipDismissal::ON_CLICK,
            transfer: TooltipTransfer::NONE,
        }
    }

    /// Create a new follow cursor `Tooltip`.
    pub fn follow_cursor(content: impl Into<TooltipContent>) -> Self {
        Self {
            content: content.into(),
            placement: TooltipPlacement::FOLLOW_CURSOR,
            activation: TooltipActivation::IMMEDIATE,
            dismissal: TooltipDismissal::NONE,
            transfer: TooltipTransfer::NONE,
        }
    }

    /// Change the text justification.
    ///
    /// NOTE: This does nothing for custom tooltips.
    pub fn with_justify(mut self, justify: Justify) -> Self {
        // TODO: Warn otherwise?
        if let TooltipContent::Primary(text) = &mut self.content {
            text.justify = justify;
        }
        self
    }

    /// Set a custom [`TooltipPlacement`].
    pub fn with_placement(mut self, placement: impl Into<TooltipPlacement>) -> Self {
        self.placement = placement.into();
        self
    }

    /// Set a custom [`TooltipActivation`].
    pub fn with_activation(mut self, activation: impl Into<TooltipActivation>) -> Self {
        self.activation = activation.into();
        self
    }

    /// Set a custom [`TooltipDismissal`].
    pub fn with_dismissal(mut self, dismissal: impl Into<TooltipDismissal>) -> Self {
        self.dismissal = dismissal.into();
        self
    }

    /// Set a custom [`TooltipTransfer`].
    pub fn with_transfer(mut self, transfer: impl Into<TooltipTransfer>) -> Self {
        self.transfer = transfer.into();
        self
    }
}

/// Tooltip content to be displayed.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "bevy_reflect", derive(bevy_reflect::Reflect))]
pub enum TooltipContent {
    /// Display the primary tooltip with custom [`RichText`].
    Primary(RichText),
    /// Display a fully custom entity as the tooltip.
    Custom(Entity),
}

impl From<&str> for TooltipContent {
    fn from(value: &str) -> Self {
        Self::Primary(RichText::from_section(
            value.to_string(),
            TextStyle::default(),
        ))
    }
}

impl From<String> for TooltipContent {
    fn from(value: String) -> Self {
        Self::Primary(RichText::from_section(value, TextStyle::default()))
    }
}

impl From<TextSection> for TooltipContent {
    fn from(value: TextSection) -> Self {
        Self::Primary(RichText::from_section(value.value, value.style))
    }
}

impl From<Vec<TextSection>> for TooltipContent {
    fn from(value: Vec<TextSection>) -> Self {
        Self::Primary(RichText::from_sections(value))
    }
}

impl From<RichText> for TooltipContent {
    fn from(value: RichText) -> Self {
        Self::Primary(value)
    }
}

impl From<Entity> for TooltipContent {
    fn from(value: Entity) -> Self {
        Self::Custom(value)
    }
}

/// Tooltip activation conditions.
///
/// Defaults to [`Self::IMMEDIATE`].
#[derive(Copy, Clone, Debug)]
#[cfg_attr(feature = "bevy_reflect", derive(bevy_reflect::Reflect))]
pub struct TooltipActivation {
    /// The hover duration before the tooltip will activate (in milliseconds).
    pub delay: u16,
    /// Whether to reset the activation delay timer whenever the cursor moves.
    pub reset_delay_on_cursor_move: bool,
}

impl TooltipActivation {
    /// Show tooltip immediately on hover.
    pub const IMMEDIATE: Self = Self {
        delay: 0,
        reset_delay_on_cursor_move: false,
    };

    /// Show tooltip after a short while.
    pub const SHORT_DELAY: Self = Self {
        delay: 200,
        reset_delay_on_cursor_move: false,
    };

    /// Show tooltip after a while.
    pub const DELAY: Self = Self {
        delay: 400,
        reset_delay_on_cursor_move: false,
    };

    /// Show tooltip after a long while.
    pub const LONG_DELAY: Self = Self {
        delay: 600,
        reset_delay_on_cursor_move: false,
    };

    /// Show tooltip after the cursor stays idle for a short while.
    pub const SHORT_IDLE: Self = Self {
        delay: 200,
        reset_delay_on_cursor_move: true,
    };

    /// Show tooltip after the cursor stays idle for a while.
    pub const IDLE: Self = Self {
        delay: 400,
        reset_delay_on_cursor_move: true,
    };

    /// Show tooltip after the cursor stays idle for a long while.
    pub const LONG_IDLE: Self = Self {
        delay: 600,
        reset_delay_on_cursor_move: true,
    };
}

impl From<u16> for TooltipActivation {
    fn from(value: u16) -> Self {
        Self {
            delay: value,
            reset_delay_on_cursor_move: false,
        }
    }
}

impl Default for TooltipActivation {
    fn default() -> Self {
        Self::IMMEDIATE
    }
}

/// Tooltip dismissal conditions.
///
/// Defaults to [`Self::NONE`].
#[derive(Copy, Clone, Debug)]
#[cfg_attr(feature = "bevy_reflect", derive(bevy_reflect::Reflect))]
pub struct TooltipDismissal {
    /// The distance from the activation point beyond which the tooltip will be dismissed.
    pub on_distance: f32,
    /// Whether the tooltip should be dismissed on click.
    pub on_click: bool,
}

impl TooltipDismissal {
    /// No tooltip dismissal.
    pub const NONE: Self = Self {
        on_distance: f32::INFINITY,
        on_click: false,
    };

    /// Dismiss tooltip on click.
    pub const ON_CLICK: Self = Self {
        on_distance: f32::INFINITY,
        on_click: true,
    };
}

impl Default for TooltipDismissal {
    fn default() -> Self {
        Self::NONE
    }
}

/// Tooltip transfer conditions.
///
/// When a transfer occurs, the next tooltip's [activation delay](TooltipActivation::delay) will be skipped.
///
/// Defaults to [`Self::NONE`].
#[derive(Copy, Clone, Debug)]
#[cfg_attr(feature = "bevy_reflect", derive(bevy_reflect::Reflect))]
pub struct TooltipTransfer {
    /// Only transfer to elements within the same group, or to self if `None`.
    pub group: Option<i8>,
    /// Only transfer to elements within the same layer or lower.
    pub layer: i8,
    /// Only transfer within this duration after the cursor moves away from the old target (in milliseconds).
    pub timeout: u16,
    /// Only transfer if the old tooltip was active.
    pub from_active: bool,
}

impl TooltipTransfer {
    /// No tooltip transfer.
    pub const NONE: Self = Self {
        group: None,
        layer: 0,
        timeout: 0,
        from_active: true,
    };

    /// Short-duration tooltip transfer.
    pub const SHORT: Self = Self {
        group: Some(0),
        layer: 0,
        timeout: 100,
        from_active: true,
    };
}

impl From<u16> for TooltipTransfer {
    fn from(value: u16) -> Self {
        Self {
            group: Some(0),
            layer: 0,
            timeout: value,
            from_active: true,
        }
    }
}

impl Default for TooltipTransfer {
    fn default() -> Self {
        Self::NONE
    }
}

/// A [`SystemSet`] for tooltip systems.
#[derive(SystemSet, Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub enum TooltipSystems {
    /// Update and show / hide the tooltip content (runs in [`PreUpdate`]).
    Content,
    /// Position the tooltip using its calculated size (runs in [`PostUpdate`]).
    Placement,
}
