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
//! ```ignore
//! app.add_plugins(TooltipPlugin::default());
//! ```
//!
//! Spawn a UI node with a [`Tooltip`]:
//!
//! ```ignore
//! commands.spawn((
//!     NodeBundle::default(),
//!     Interaction::default(),
//!     Tooltip::from_section("Hello, world!", TextStyle::default()),
//! ));
//! ```
//!
//! # Advanced
//!
//! To customize tooltip behavior, see [`Tooltip`].
//!
//! To set a custom primary tooltip, see [`TooltipPlugin`] and [`PrimaryTooltip`].
//! For fully custom per-entity tooltips, use [`TooltipContent::Custom`].

mod context;
mod placement;

/// Re-exports for commonly used types.
///
/// # Usage
///
/// ```
/// use pyri_tooltip::prelude::*;
/// ```
pub mod prelude {
    pub use super::{
        PrimaryTooltip, Tooltip, TooltipActivation, TooltipContent, TooltipPlacement,
        TooltipPlugin, TooltipSet, TooltipTransfer,
    };
}

use bevy_app::{Plugin, PostUpdate, PreUpdate};
use bevy_color::Color;
use bevy_core::Name;
#[cfg(feature = "bevy_reflect")]
use bevy_ecs::reflect::{ReflectComponent, ReflectResource};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    schedule::{IntoSystemSetConfigs as _, SystemSet},
    system::Resource,
    world::World,
};
use bevy_hierarchy::BuildWorldChildren as _;
use bevy_render::view::Visibility;
use bevy_sprite::Anchor;
use bevy_text::{JustifyText, Text, TextSection, TextStyle};
use bevy_transform::TransformSystem;
use bevy_ui::{
    node_bundles::{NodeBundle, TextBundle},
    PositionType, Style, UiRect, UiSystem, Val, ZIndex,
};

pub use placement::TooltipPlacement;

/// A [`Plugin`] that sets up the tooltip widget system.
pub struct TooltipPlugin {
    /// Set a custom entity for [`PrimaryTooltip::container`], or spawn the default container
    /// entity if `None`.
    ///
    /// This entity should include all of the components of [`NodeBundle`], with
    /// [`Visibility::Hidden`] and [`Style::position_type`] set to [`PositionType::Absolute`].
    pub container: Entity,
    /// Set a custom entity for [`PrimaryTooltip::text`], or spawn the default text entity if
    /// `None`.
    ///
    /// This entity should include all of the components of [`TextBundle`] and be a child of
    /// [`Self::container`].
    pub text: Entity,
}

impl Plugin for TooltipPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.register_type::<PrimaryTooltip>();
        let primary_tooltip = PrimaryTooltip::new(app.world_mut(), self.container, self.text);
        app.insert_resource(primary_tooltip);

        app.register_type::<Tooltip>();

        app.configure_sets(PreUpdate, (UiSystem::Focus, TooltipSet::Content).chain());
        app.configure_sets(
            PostUpdate,
            (
                UiSystem::Layout,
                TooltipSet::Placement,
                TransformSystem::TransformPropagate,
            )
                .chain(),
        );
        app.add_plugins((context::plugin, placement::plugin));
    }
}

impl Default for TooltipPlugin {
    fn default() -> Self {
        Self {
            container: Entity::PLACEHOLDER,
            text: Entity::PLACEHOLDER,
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
pub struct PrimaryTooltip {
    /// The [`Entity`] ID of the UI node to be used as the primary tooltip.
    pub container: Entity,
    /// The [`Entity`] ID of the UI node to be used as the primary tooltip's text.
    pub text: Entity,
}

impl PrimaryTooltip {
    fn new(world: &mut World, container: Entity, text: Entity) -> Self {
        let container = if container != Entity::PLACEHOLDER {
            container
        } else {
            world
                .spawn((
                    Name::new("PrimaryTooltip"),
                    NodeBundle {
                        style: Style {
                            position_type: PositionType::Absolute,
                            max_width: Val::Vw(40.0),
                            padding: UiRect::all(Val::Px(8.0)),
                            ..Default::default()
                        },
                        background_color: Color::srgba(0.2, 0.2, 0.3, 0.95).into(),
                        visibility: Visibility::Hidden,
                        z_index: ZIndex::Global(999),
                        ..Default::default()
                    },
                ))
                .id()
        };

        let text = if text != Entity::PLACEHOLDER {
            text
        } else {
            world
                .spawn((Name::new("Text"), TextBundle::default()))
                .set_parent(container)
                .id()
        };

        Self { container, text }
    }
}

// TODO: Animation, wedge (like a speech bubble), easier content customization / icons.
/// A [`Component`] that specifies a tooltip to be displayed on hover.
///
/// This will only work on entities that also include the following components:
/// - [`NodeBundle`] components
/// - [`Interaction`](bevy_ui::Interaction)
///
/// The default behavior consists of the following values:
/// - [`TooltipActivation::IDLE`]
/// - [`TooltipTransfer::NONE`]
/// - [`TooltipPlacement::CURSOR`]
#[derive(Component, Clone, Debug)]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(bevy_reflect::Reflect),
    reflect(Component)
)]
pub struct Tooltip {
    /// The conditions for activating the tooltip.
    pub activation: TooltipActivation,
    /// The conditions for dismissing the tooltip.
    pub dismissal: TooltipDismissal,
    /// The conditions for skipping the next tooltip's activation delay.
    pub transfer: TooltipTransfer,
    /// How the position of the tooltip entity should be determined.
    pub placement: TooltipPlacement,
    /// The tooltip entity and content to be displayed.
    pub content: TooltipContent,
}

impl Tooltip {
    /// Create a new fixed `Tooltip`.
    pub fn fixed(placement: Anchor, content: impl Into<TooltipContent>) -> Self {
        Self {
            activation: TooltipActivation::IMMEDIATE,
            dismissal: TooltipDismissal::NONE,
            transfer: TooltipTransfer::SHORT,
            placement: placement.into(),
            content: content.into(),
        }
    }

    /// Create a new cursor `Tooltip`.
    pub fn cursor(content: impl Into<TooltipContent>) -> Self {
        Self {
            activation: TooltipActivation::IDLE,
            dismissal: TooltipDismissal::ON_CLICK,
            transfer: TooltipTransfer::NONE,
            placement: TooltipPlacement::CURSOR,
            content: content.into(),
        }
    }

    /// Set [`JustifyText`].
    ///
    /// NOTE: This does nothing for custom tooltips.
    pub fn with_justify(mut self, justify_text: JustifyText) -> Self {
        // TODO: Warn otherwise?
        if let TooltipContent::Primary(text) = &mut self.content {
            text.justify = justify_text;
        }
        self
    }

    /// Set [`TooltipActivation`].
    pub fn with_activation(mut self, activation: impl Into<TooltipActivation>) -> Self {
        self.activation = activation.into();
        self
    }

    /// Set [`TooltipDismissal`].
    pub fn with_dismissal(mut self, dismissal: impl Into<TooltipDismissal>) -> Self {
        self.dismissal = dismissal.into();
        self
    }

    /// Set [`TooltipTransfer`].
    pub fn with_transfer(mut self, transfer: impl Into<TooltipTransfer>) -> Self {
        self.transfer = transfer.into();
        self
    }

    /// Set [`TooltipPlacement`].
    pub fn with_placement(mut self, placement: impl Into<TooltipPlacement>) -> Self {
        self.placement = placement.into();
        self
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

/// The tooltip transfer conditions.
///
/// When a transfer occurs, the next tooltip's activation delay will be skipped.
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

/// The tooltip entity and content to be displayed.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "bevy_reflect", derive(bevy_reflect::Reflect))]
pub enum TooltipContent {
    /// Use the primary tooltip entity with custom [`Text`].
    Primary(Text),
    /// Use a fully custom entity as the tooltip.
    Custom(Entity),
}

impl From<&str> for TooltipContent {
    fn from(value: &str) -> Self {
        Self::Primary(Text::from_section(value.to_string(), TextStyle::default()))
    }
}

impl From<String> for TooltipContent {
    fn from(value: String) -> Self {
        Self::Primary(Text::from_section(value, TextStyle::default()))
    }
}

impl From<TextSection> for TooltipContent {
    fn from(value: TextSection) -> Self {
        Self::Primary(Text::from_section(value.value, value.style))
    }
}

impl From<Vec<TextSection>> for TooltipContent {
    fn from(value: Vec<TextSection>) -> Self {
        Self::Primary(Text::from_sections(value))
    }
}

impl From<Text> for TooltipContent {
    fn from(value: Text) -> Self {
        Self::Primary(value)
    }
}

impl From<Entity> for TooltipContent {
    fn from(value: Entity) -> Self {
        Self::Custom(value)
    }
}

/// A [`SystemSet`] for tooltip systems.
#[derive(SystemSet, Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub enum TooltipSet {
    /// Update and show / hide the tooltip content (runs in [`PreUpdate`]).
    Content,
    /// Position the tooltip using its calculated size (runs in [`PostUpdate`]).
    Placement,
}
