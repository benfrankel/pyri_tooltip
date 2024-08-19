//! Featureful tooltips for Bevy.
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
        PrimaryTooltip, Tooltip, TooltipActivation, TooltipEntity, TooltipPlacement, TooltipPlugin,
        TooltipTransfer,
    };
}

use bevy_app::Plugin;
use bevy_color::Color;
use bevy_core::Name;
#[cfg(feature = "bevy_reflect")]
use bevy_ecs::reflect::{ReflectComponent, ReflectResource};
use bevy_ecs::{component::Component, entity::Entity, system::Resource, world::World};
use bevy_hierarchy::BuildWorldChildren as _;
use bevy_render::view::Visibility;
use bevy_text::{Text, TextSection, TextStyle};
use bevy_ui::{
    node_bundles::{NodeBundle, TextBundle},
    PositionType, Style, UiRect, Val, ZIndex,
};

pub use placement::TooltipPlacement;

/// A [`Plugin`] that sets up the tooltip widget system.
#[derive(Default)]
pub struct TooltipPlugin {
    /// Set a custom entity for [`PrimaryTooltip::container`], or spawn the default container
    /// entity if `None`.
    ///
    /// This entity should include all of the components of [`NodeBundle`], with
    /// [`Visibility::Hidden`] and [`Style::position_type`] set to [`PositionType::Absolute`].
    pub container: Option<Entity>,
    /// Set a custom entity for [`PrimaryTooltip::text`], or spawn the default text entity if
    /// `None`.
    ///
    /// This entity should include all of the components of [`TextBundle`].
    pub text: Option<Entity>,
}

impl Plugin for TooltipPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.register_type::<PrimaryTooltip>();
        let primary_tooltip = PrimaryTooltip::new(app.world_mut(), self.container, self.text);
        app.insert_resource(primary_tooltip);

        app.register_type::<Tooltip>();

        app.add_plugins((context::plugin, placement::plugin));
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
    fn new(world: &mut World, container: Option<Entity>, text: Option<Entity>) -> Self {
        let container = container.unwrap_or_else(|| {
            world
                .spawn((
                    Name::new("PrimaryTooltip"),
                    NodeBundle {
                        style: Style {
                            position_type: PositionType::Absolute,
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
        });

        let text = text.unwrap_or_else(|| {
            world
                .spawn((Name::new("Text"), TextBundle::default()))
                .set_parent(container)
                .id()
        });

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
#[derive(Component)]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(bevy_reflect::Reflect),
    reflect(Component)
)]
pub struct Tooltip {
    /// The conditions for activating and dismissing the tooltip.
    pub activation: TooltipActivation,
    /// The conditions for skipping the next tooltip's activation delay.
    pub transfer: TooltipTransfer,
    /// How the position of the tooltip entity should be determined.
    pub placement: TooltipPlacement,
    /// The entity to display as the tooltip.
    pub entity: TooltipEntity,
}

impl Tooltip {
    /// Use the provided tooltip entity and default behavior.
    fn new(entity: TooltipEntity) -> Self {
        Self {
            activation: TooltipActivation::IDLE,
            transfer: TooltipTransfer::NONE,
            placement: TooltipPlacement::CURSOR,
            entity,
        }
    }

    /// Use the primary tooltip entity with a single [`TextSection`] and default behavior.
    pub fn from_section(value: impl Into<String>, style: TextStyle) -> Self {
        Self::new(TooltipEntity::Primary(Text::from_section(value, style)))
    }

    /// Use the primary tooltip entity with a list of [`TextSection`]s and default behavior.
    pub fn from_sections(sections: impl IntoIterator<Item = TextSection>) -> Self {
        Self::new(TooltipEntity::Primary(Text::from_sections(sections)))
    }

    /// Use the primary tooltip entity with a given [`Text`] and default behavior.
    pub fn from_text(text: impl Into<Text>) -> Self {
        Self::new(TooltipEntity::Primary(text.into()))
    }

    /// Use a custom tooltip entity and default behavior.
    pub fn custom(entity: Entity) -> Self {
        Self::new(TooltipEntity::Custom(entity))
    }

    /// Set a custom [`TooltipActivation`].
    pub fn with_activation(mut self, activation: TooltipActivation) -> Self {
        self.activation = activation;
        self
    }

    /// Set a custom [`TooltipTransfer`].
    pub fn with_transfer(mut self, transfer: TooltipTransfer) -> Self {
        self.transfer = transfer;
        self
    }

    /// Set a custom [`TooltipPlacement`].
    pub fn with_placement(mut self, placement: TooltipPlacement) -> Self {
        self.placement = placement;
        self
    }
}

/// The tooltip activation and dismissal conditions.
///
/// Defaults to [`Self::IMMEDIATE`].
#[derive(Copy, Clone, Debug)]
#[cfg_attr(feature = "bevy_reflect", derive(bevy_reflect::Reflect))]
pub struct TooltipActivation {
    /// The hover duration before the tooltip will activate (in milliseconds).
    pub delay: u16,
    /// Whether to reset the activation delay timer whenever the cursor moves.
    pub reset_delay_on_cursor_move: bool,
    /// The radius around the activation point beyond which the tooltip will be dismissed.
    pub dismiss_radius: f32,
    // TODO: pub dismiss_on_click: bool,
}

impl TooltipActivation {
    /// Show tooltip immediately on hover.
    pub const IMMEDIATE: Self = Self {
        delay: 0,
        reset_delay_on_cursor_move: false,
        dismiss_radius: f32::INFINITY,
    };

    /// Show tooltip after a short while.
    pub const SHORT_DELAY: Self = Self {
        delay: 200,
        reset_delay_on_cursor_move: false,
        dismiss_radius: f32::INFINITY,
    };

    /// Show tooltip after a while.
    pub const DELAY: Self = Self {
        delay: 400,
        reset_delay_on_cursor_move: false,
        dismiss_radius: f32::INFINITY,
    };

    /// Show tooltip after a long while.
    pub const LONG_DELAY: Self = Self {
        delay: 600,
        reset_delay_on_cursor_move: false,
        dismiss_radius: f32::INFINITY,
    };

    /// Show tooltip after the cursor stays idle for a short while.
    pub const SHORT_IDLE: Self = Self {
        delay: 200,
        reset_delay_on_cursor_move: true,
        dismiss_radius: f32::INFINITY,
    };

    /// Show tooltip after the cursor stays idle for a while.
    pub const IDLE: Self = Self {
        delay: 400,
        reset_delay_on_cursor_move: true,
        dismiss_radius: f32::INFINITY,
    };

    /// Show tooltip after the cursor stays idle for a long while.
    pub const LONG_IDLE: Self = Self {
        delay: 600,
        reset_delay_on_cursor_move: true,
        dismiss_radius: f32::INFINITY,
    };
}

impl Default for TooltipActivation {
    fn default() -> Self {
        Self::IMMEDIATE
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

impl Default for TooltipTransfer {
    fn default() -> Self {
        Self::NONE
    }
}

/// The tooltip entity and content to be displayed.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "bevy_reflect", derive(bevy_reflect::Reflect))]
pub enum TooltipEntity {
    /// Use the primary tooltip entity with custom [`Text`].
    Primary(Text),
    /// Use a fully custom entity as the tooltip.
    Custom(Entity),
}
