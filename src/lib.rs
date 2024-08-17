//! TODO

use core::f32;

use bevy_app::Plugin;
use bevy_core::Name;
#[cfg(feature = "bevy_reflect")]
use bevy_ecs::reflect::{ReflectComponent, ReflectResource};
use bevy_ecs::{component::Component, entity::Entity, system::Resource, world::World};
use bevy_hierarchy::BuildWorldChildren as _;
use bevy_render::view::Visibility;
use bevy_sprite::Anchor;
use bevy_text::{Text, TextSection, TextStyle};
use bevy_ui::{
    node_bundles::{NodeBundle, TextBundle},
    PositionType, Style, UiRect, Val, ZIndex,
};

/// TODO
#[derive(Default)]
pub struct TooltipPlugin {
    /// TODO
    pub container_entity: Option<Entity>,
    /// TODO
    pub text_entity: Option<Entity>,
}

impl Plugin for TooltipPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.register_type::<PrimaryTooltip>();
        let primary_tooltip =
            PrimaryTooltip::new(app.world_mut(), self.container_entity, self.text_entity);
        app.insert_resource(primary_tooltip);

        app.register_type::<Tooltip>();
    }
}

/// TODO
#[derive(Resource)]
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
                            max_width: Val::Vw(40.0),
                            padding: UiRect::all(Val::Px(8.0)),
                            ..Default::default()
                        },
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
/// TODO
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
    /// Use the given tooltip entity and default behavior.
    fn new(entity: TooltipEntity) -> Self {
        Self {
            activation: Default::default(),
            transfer: Default::default(),
            placement: Default::default(),
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

/// TODO
#[cfg_attr(feature = "bevy_reflect", derive(bevy_reflect::Reflect))]
pub struct TooltipActivation {
    /// The hover duration before the tooltip will activate (in milliseconds).
    pub delay: u16,
    /// Whether to reset the activation delay timer whenever the cursor moves.
    pub reset_delay_on_cursor_move: bool,
    /// The cursor distance from the activation point beyond which the tooltip will be dismissed.
    pub dismiss_distance: f32,
}

impl TooltipActivation {
    /// The default `TooltipActivation`.
    pub const DEFAULT: Self = Self {
        delay: 100,
        reset_delay_on_cursor_move: false,
        dismiss_distance: f32::INFINITY,
    };
}

impl Default for TooltipActivation {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// TODO
#[cfg_attr(feature = "bevy_reflect", derive(bevy_reflect::Reflect))]
pub struct TooltipTransfer {
    /// Only transfer to elements in the same group, or to self if `None`.
    pub group: Option<i8>,
    /// Only transfer to elements in the same layer or lower.
    pub layer: i8,
    /// Only transfer within this duration after the cursor moves away from the old target (in milliseconds), or forever if `None`.
    pub timeout: Option<u16>,
    /// Only transfer if the old tooltip was active.
    pub from_active: bool,
}

impl TooltipTransfer {
    /// The default `TooltipTransfer`.
    pub const DEFAULT: Self = Self {
        group: None,
        layer: 0,
        timeout: Some(0),
        from_active: true,
    };
}

impl Default for TooltipTransfer {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// TODO
#[cfg_attr(feature = "bevy_reflect", derive(bevy_reflect::Reflect))]
pub struct TooltipPlacement {
    /// The anchor point on the tooltip entity.
    pub anchor: Anchor,
    /// The target position expressed as an anchor point on the target entity, or `None` to use the cursor position instead.
    pub target: Option<Anchor>,
    /// An additional horizontal offset for the tooltip entity.
    pub offset_x: Val,
    /// An additional vertical offset for the tooltip entity.
    pub offset_y: Val,
    /// Clamp the tooltip entity within the window with additional padding.
    pub clamp_padding: UiRect,
}

impl TooltipPlacement {
    /// The default `TooltipPlacement`.
    pub const DEFAULT: Self = Self {
        anchor: Anchor::TopLeft,
        target: None,
        offset_x: Val::Px(16.0),
        offset_y: Val::Px(16.0),
        clamp_padding: UiRect::all(Val::ZERO),
    };
}

impl Default for TooltipPlacement {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// TODO
#[cfg_attr(feature = "bevy_reflect", derive(bevy_reflect::Reflect))]
pub enum TooltipEntity {
    /// Use the primary tooltip entity with custom text.
    Primary(Text),
    /// Use a fully custom entity as the tooltip.
    Custom(Entity),
}
