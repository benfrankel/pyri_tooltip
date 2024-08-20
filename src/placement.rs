use bevy_app::{App, PostUpdate};
use bevy_ecs::{
    entity::Entity,
    query::With,
    schedule::IntoSystemConfigs as _,
    system::{Commands, Query, Res},
};
use bevy_hierarchy::Parent;
use bevy_math::Vec2;
use bevy_render::camera::Camera;
use bevy_sprite::Anchor;
use bevy_transform::components::{GlobalTransform, Transform};
use bevy_ui::{IsDefaultUiCamera, Node, Style, TargetCamera, UiRect, Val};
use tiny_bail::prelude::*;

use crate::{
    context::{TooltipContext, TooltipState},
    PrimaryTooltip, TooltipEntity, TooltipSet,
};

pub(super) fn plugin(app: &mut App) {
    app.add_systems(PostUpdate, place_tooltip.in_set(TooltipSet::Placement));
}

/// The tooltip placement configuration.
///
/// Defaults to [`Self::CURSOR_CENTERED`].
#[derive(Copy, Clone, Debug)]
#[cfg_attr(feature = "bevy_reflect", derive(bevy_reflect::Reflect))]
pub struct TooltipPlacement {
    /// The anchor point on the tooltip entity.
    pub tooltip_anchor: Anchor,
    /// The target position expressed as an anchor point on the target entity, or `None` to use the cursor position instead.
    pub target_anchor: Option<Anchor>,
    /// An additional horizontal offset for the tooltip entity.
    pub offset_x: Val,
    /// An additional vertical offset for the tooltip entity.
    pub offset_y: Val,
    /// Clamp the tooltip entity within the window with additional padding.
    pub clamp_padding: UiRect,
}

impl TooltipPlacement {
    /// Show tooltip centered at cursor.
    pub const CURSOR_CENTERED: Self = Self {
        tooltip_anchor: Anchor::Center,
        target_anchor: None,
        offset_x: Val::ZERO,
        offset_y: Val::ZERO,
        clamp_padding: UiRect::ZERO,
    };

    /// Show tooltip at cursor.
    pub const CURSOR: Self = Self {
        tooltip_anchor: Anchor::TopLeft,
        target_anchor: None,
        offset_x: Val::Px(16.0),
        offset_y: Val::Px(16.0),
        clamp_padding: UiRect::ZERO,
    };

    /// Show tooltip to the left of target.
    pub const LEFT: Self = Self {
        tooltip_anchor: Anchor::CenterRight,
        target_anchor: Some(Anchor::CenterLeft),
        offset_x: Val::ZERO,
        offset_y: Val::ZERO,
        clamp_padding: UiRect::ZERO,
    };

    /// Show tooltip to the right of target.
    pub const RIGHT: Self = Self {
        tooltip_anchor: Anchor::CenterLeft,
        target_anchor: Some(Anchor::CenterRight),
        offset_x: Val::ZERO,
        offset_y: Val::ZERO,
        clamp_padding: UiRect::ZERO,
    };

    /// Show tooltip above target.
    pub const TOP: Self = Self {
        tooltip_anchor: Anchor::BottomCenter,
        target_anchor: Some(Anchor::TopCenter),
        offset_x: Val::ZERO,
        offset_y: Val::ZERO,
        clamp_padding: UiRect::ZERO,
    };

    /// Show tooltip below target.
    pub const BOTTOM: Self = Self {
        tooltip_anchor: Anchor::TopCenter,
        target_anchor: Some(Anchor::BottomCenter),
        offset_x: Val::ZERO,
        offset_y: Val::ZERO,
        clamp_padding: UiRect::ZERO,
    };
}

impl Default for TooltipPlacement {
    fn default() -> Self {
        Self::CURSOR_CENTERED
    }
}

// TODO: Only run on `ShowTooltip` event OR if using target anchor + target has moved or resized.
fn place_tooltip(
    mut commands: Commands,
    ctx: Res<TooltipContext>,
    primary: Res<PrimaryTooltip>,
    camera_query: Query<(Entity, &Camera)>,
    target_camera_query: Query<&TargetCamera>,
    parent_query: Query<&Parent>,
    default_camera_query: Query<(Entity, &Camera), With<IsDefaultUiCamera>>,
    target_query: Query<(&GlobalTransform, &Node)>,
    mut tooltip_query: Query<(&mut Style, &mut Transform, &GlobalTransform, &Node)>,
) {
    rq!(matches!(ctx.state, TooltipState::Active));
    let (target_gt, target_node) = rq!(target_query.get(ctx.target));
    let entity = match &ctx.tooltip.entity {
        TooltipEntity::Primary(_) => primary.container,
        &TooltipEntity::Custom(id) => id,
    };
    let (mut style, mut transform, gt, node) = or_return!(tooltip_query.get_mut(entity));

    // Identify the target camera and viewport rect.
    let (camera_entity, camera) = if let Ok(camera) = camera_query.get_single() {
        camera
    } else {
        let mut target = ctx.target;
        loop {
            if let Ok(target_camera) = target_camera_query.get(target) {
                break r!(camera_query.get(target_camera.0));
            } else if let Ok(parent) = parent_query.get(target) {
                target = parent.get();
            } else {
                break r!(default_camera_query.get_single());
            }
        }
    };
    let viewport = r!(camera.logical_viewport_rect());
    // Insert instead of mutate because the tooltip entity might not spawn with a `TargetCamera` component.
    commands.entity(entity).insert(TargetCamera(camera_entity));

    let placement = &ctx.tooltip.placement;

    // Calculate target position.
    let mut pos = if let Some(target_anchor) = placement.target_anchor {
        let target_rect = target_node.logical_rect(target_gt);
        target_rect.center() - target_rect.size() * target_anchor.as_vec() * Vec2::new(-1.0, 1.0)
    } else {
        ctx.cursor_pos
    };

    // Apply tooltip anchor to target position.
    let tooltip_rect = node.logical_rect(gt);
    let tooltip_anchor =
        tooltip_rect.size() * placement.tooltip_anchor.as_vec() * Vec2::new(-1.0, 1.0);
    pos += tooltip_anchor;

    // Resolve offset `Val`s.
    let size = viewport.size();
    let offset_x = placement.offset_x.resolve(size.x, size).unwrap_or_default();
    let offset_y = placement.offset_y.resolve(size.y, size).unwrap_or_default();

    // Apply offset.
    pos += Vec2::new(offset_x, offset_y);

    // Resolve clamp padding `Val`s.
    let UiRect {
        left,
        right,
        top,
        bottom,
    } = placement.clamp_padding;
    let left = left.resolve(size.x, size).unwrap_or_default();
    let right = right.resolve(size.x, size).unwrap_or_default();
    let top = top.resolve(size.x, size).unwrap_or_default();
    let bottom = bottom.resolve(size.x, size).unwrap_or_default();

    // Apply clamping.
    let half_size = tooltip_rect.half_size();
    let mut left = half_size.x + left;
    let mut right = size.x - half_size.x - right;
    if left > right {
        let mid = (left + right) / 2.0;
        left = mid;
        right = mid;
    }
    let mut top = half_size.y + top;
    let mut bottom = size.y - half_size.y - bottom;
    if top > bottom {
        let mid = (top + bottom) / 2.0;
        top = mid;
        bottom = mid;
    }
    pos = pos.clamp(Vec2::new(left, top), Vec2::new(right, bottom));

    // Set position via `Style`.
    let top_left = pos - tooltip_rect.half_size();
    style.top = Val::Px(top_left.y);
    style.left = Val::Px(top_left.x);

    // Set position via `Transform`.
    // This system has to run after `UiSystem::Layout` so that its size is calculated
    // from the updated text. However, that means that `Style` positioning will be
    // delayed by 1 frame. As a workaround, update the `Transform` directly as well.
    pos = round_layout_coords(pos);
    transform.translation.x = pos.x;
    transform.translation.y = pos.y;
}

/// Taken from `bevy_ui`, used in `ui_layout_system`.
fn round_ties_up(value: f32) -> f32 {
    if value.fract() != -0.5 {
        value.round()
    } else {
        value.ceil()
    }
}

/// Taken from `bevy_ui`, used in `ui_layout_system`.
fn round_layout_coords(value: Vec2) -> Vec2 {
    Vec2 {
        x: round_ties_up(value.x),
        y: round_ties_up(value.y),
    }
}
