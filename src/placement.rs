use bevy_app::{App, PostUpdate};
use bevy_ecs::{
    schedule::IntoScheduleConfigs as _,
    system::{Commands, Query, Res},
};
use bevy_math::{Rect, Vec2};
use bevy_render::camera::Camera;
use bevy_sprite::Anchor;
use bevy_transform::components::{GlobalTransform, Transform};
use bevy_ui::{ComputedNode, DefaultUiCamera, Node, UiRect, UiTargetCamera, Val};
use tiny_bail::prelude::*;

use crate::{
    PrimaryTooltip, TooltipContent, TooltipSystems,
    context::{TooltipContext, TooltipState},
};

pub(super) fn plugin(app: &mut App) {
    app.add_systems(PostUpdate, place_tooltip.in_set(TooltipSystems::Placement));
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
}

impl From<Anchor> for TooltipPlacement {
    fn from(value: Anchor) -> Self {
        Self {
            tooltip_anchor: match value {
                Anchor::Center | Anchor::Custom(_) => Anchor::Center,
                Anchor::BottomLeft => Anchor::TopRight,
                Anchor::BottomCenter => Anchor::TopCenter,
                Anchor::BottomRight => Anchor::TopLeft,
                Anchor::CenterLeft => Anchor::CenterRight,
                Anchor::CenterRight => Anchor::CenterLeft,
                Anchor::TopLeft => Anchor::BottomRight,
                Anchor::TopCenter => Anchor::BottomCenter,
                Anchor::TopRight => Anchor::BottomLeft,
            },
            target_anchor: Some(value),
            offset_x: Val::ZERO,
            offset_y: Val::ZERO,
            clamp_padding: UiRect::ZERO,
        }
    }
}

impl From<Vec2> for TooltipPlacement {
    fn from(value: Vec2) -> Self {
        Self {
            tooltip_anchor: Anchor::TopLeft,
            target_anchor: None,
            offset_x: Val::Px(value.x),
            offset_y: Val::Px(value.y),
            clamp_padding: UiRect::ZERO,
        }
    }
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
    target_query: Query<(&GlobalTransform, &ComputedNode)>,
    target_camera_query: Query<&UiTargetCamera>,
    default_ui_camera: DefaultUiCamera,
    camera_query: Query<&Camera>,
    mut tooltip_query: Query<(&mut Node, &mut Transform, &GlobalTransform, &ComputedNode)>,
) {
    rq!(matches!(ctx.state, TooltipState::Active));
    let (target_gt, target_computed) = rq!(target_query.get(ctx.target));
    let entity = match &ctx.tooltip.content {
        TooltipContent::Primary(_) => primary.container,
        &TooltipContent::Custom(id) => id,
    };
    let (mut node, mut transform, gt, computed) = r!(tooltip_query.get_mut(entity));

    // Identify the target camera and viewport rect.
    let camera_entity = r!(target_camera_query
        .get(ctx.target)
        .map(UiTargetCamera::entity)
        .ok()
        .or(default_ui_camera.get()));
    let camera = r!(camera_query.get(camera_entity));
    let viewport = r!(camera.logical_viewport_rect());
    // Insert instead of mutate because the tooltip entity might not spawn with a `UiTargetCamera` component.
    commands
        .entity(entity)
        .insert(UiTargetCamera(camera_entity));

    let placement = &ctx.tooltip.placement;

    // Calculate target position.
    let mut pos = if let Some(target_anchor) = placement.target_anchor {
        let target_rect =
            Rect::from_center_size(target_gt.translation().truncate(), target_computed.size());
        target_rect.center() - target_rect.size() * target_anchor.as_vec() * Vec2::new(-1.0, 1.0)
    } else {
        ctx.cursor_pos
    };

    // Apply tooltip anchor to target position.
    let tooltip_rect = Rect::from_center_size(gt.translation().truncate(), computed.size());
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

    // Set position via `Node`.
    let top_left = pos - tooltip_rect.half_size();
    node.top = Val::Px(top_left.y);
    node.left = Val::Px(top_left.x);

    // Set position via `Transform`.
    // This system has to run after `UiSystem::Layout` so that its size is calculated
    // from the updated text. However, that means that `Node` positioning will be
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
