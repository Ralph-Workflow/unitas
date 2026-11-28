use std::collections::HashMap;
use std::sync::{Mutex, OnceLock, RwLock};

use unitas_asset::scene::{
    CanvasComponent, CanvasMatchMode, CanvasRenderMode, CanvasScaleMode, ContentSizeFitMode,
    LayoutGroupAxis, RectTransformComponent, SpriteDrawMode, UiBinding, UiButtonComponent,
    UiContentSizeFitterComponent, UiElementComponent, UiImageComponent, UiLayoutGroupComponent,
    UiTextComponent,
};
use unitas_core::EntityId;
use unitas_render::glyphs::{FontLibrary, TextStyle, layout_text};
use unitas_render::ui_pass::{
    FOCUS_INDICATOR_COLOR, HorizontalAlign, VerticalAlign, focus_border_rects, ui_sprite_size,
};

use crate::input::{
    GAMEPAD_AXIS_LEFT_X, GAMEPAD_AXIS_LEFT_Y, GAMEPAD_BUTTON_DPAD_DOWN, GAMEPAD_BUTTON_DPAD_LEFT,
    GAMEPAD_BUTTON_DPAD_RIGHT, GAMEPAD_BUTTON_DPAD_UP, GAMEPAD_BUTTON_SOUTH, InputState,
    KEY_BACKSPACE, KEY_DELETE, KEY_DOWN_ARROW, KEY_ESCAPE, KEY_LEFT_ARROW, KEY_LEFT_SHIFT,
    KEY_RETURN, KEY_RIGHT_ARROW, KEY_RIGHT_SHIFT, KEY_SPACE, KEY_TAB, KEY_UP_ARROW, STATE_PRESSED,
};
use crate::ui::UiTextItem;

pub use unitas_render::ui_pass::SpriteRef;

pub const KEY_HOME: u32 = 278;
pub const KEY_END: u32 = 279;

#[derive(Clone, Debug, PartialEq)]
pub struct UiInputFieldComponent {
    pub entity: EntityId,
    pub text: String,
    pub placeholder: String,
    pub caret_position: usize,
    pub interactable: bool,
    pub character_limit: usize,
    pub color: [f32; 4],
    pub placeholder_color: [f32; 4],
    pub background_color: [f32; 4],
    pub font_size: f32,
    pub enabled: bool,
    pub editing_original_text: Option<String>,
    pub was_canceled: bool,
}

impl Default for UiInputFieldComponent {
    fn default() -> Self {
        Self {
            entity: EntityId::new(0),
            text: String::new(),
            placeholder: String::new(),
            caret_position: 0,
            interactable: true,
            character_limit: 0,
            color: [1.0, 1.0, 1.0, 1.0],
            placeholder_color: [0.6, 0.6, 0.6, 0.85],
            background_color: [0.16, 0.18, 0.24, 1.0],
            font_size: 16.0,
            enabled: true,
            editing_original_text: None,
            was_canceled: false,
        }
    }
}

impl UiInputFieldComponent {
    pub fn clamp_caret(text: &str, pos: usize) -> usize {
        let clamped = pos.min(text.len());
        if text.is_char_boundary(clamped) {
            return clamped;
        }
        let mut idx = clamped;
        while idx > 0 && !text.is_char_boundary(idx) {
            idx -= 1;
        }
        idx
    }

    pub fn char_count(&self) -> usize {
        self.text.chars().count()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InputFieldValueChangedEvent {
    pub entity: EntityId,
    pub text: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InputFieldEndEditEvent {
    pub entity: EntityId,
    pub text: String,
    pub canceled: bool,
}

const MAX_HIERARCHY_DEPTH: usize = 64;
const FOCUS_BORDER_THICKNESS: f32 = 2.0;
const STICK_NAVIGATION_THRESHOLD: f64 = 0.5;
const FULL_UV: [f32; 4] = [0.0, 0.0, 1.0, 1.0];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum TextAnchor {
    #[default]
    UpperLeft = 0,
    UpperCenter = 1,
    UpperRight = 2,
    MiddleLeft = 3,
    MiddleCenter = 4,
    MiddleRight = 5,
    LowerLeft = 6,
    LowerCenter = 7,
    LowerRight = 8,
}

impl TextAnchor {
    pub const fn from_u32(code: u32) -> Option<Self> {
        match code {
            0 => Some(Self::UpperLeft),
            1 => Some(Self::UpperCenter),
            2 => Some(Self::UpperRight),
            3 => Some(Self::MiddleLeft),
            4 => Some(Self::MiddleCenter),
            5 => Some(Self::MiddleRight),
            6 => Some(Self::LowerLeft),
            7 => Some(Self::LowerCenter),
            8 => Some(Self::LowerRight),
            _ => None,
        }
    }

    pub const fn horizontal(self) -> HorizontalAlign {
        match self {
            Self::UpperLeft | Self::MiddleLeft | Self::LowerLeft => HorizontalAlign::Left,
            Self::UpperCenter | Self::MiddleCenter | Self::LowerCenter => HorizontalAlign::Center,
            Self::UpperRight | Self::MiddleRight | Self::LowerRight => HorizontalAlign::Right,
        }
    }

    pub const fn vertical(self) -> VerticalAlign {
        match self {
            Self::UpperLeft | Self::UpperCenter | Self::UpperRight => VerticalAlign::Top,
            Self::MiddleLeft | Self::MiddleCenter | Self::MiddleRight => VerticalAlign::Middle,
            Self::LowerLeft | Self::LowerCenter | Self::LowerRight => VerticalAlign::Bottom,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum HorizontalWrapMode {
    Wrap = 0,
    #[default]
    Overflow = 1,
}

impl HorizontalWrapMode {
    pub const fn from_u32(code: u32) -> Option<Self> {
        match code {
            0 => Some(Self::Wrap),
            1 => Some(Self::Overflow),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum VerticalWrapMode {
    Truncate = 0,
    #[default]
    Overflow = 1,
}

impl VerticalWrapMode {
    pub const fn from_u32(code: u32) -> Option<Self> {
        match code {
            0 => Some(Self::Truncate),
            1 => Some(Self::Overflow),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextLayoutConfig {
    pub alignment: TextAnchor,
    pub horizontal_overflow: HorizontalWrapMode,
    pub vertical_overflow: VerticalWrapMode,
    pub line_spacing: f32,
}

impl Default for TextLayoutConfig {
    fn default() -> Self {
        Self {
            alignment: TextAnchor::UpperLeft,
            horizontal_overflow: HorizontalWrapMode::Overflow,
            vertical_overflow: VerticalWrapMode::Overflow,
            line_spacing: 1.0,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct UiDrawItem {
    pub rect_px: [f32; 4],
    pub color: [f32; 4],
    pub sprite: Option<SpriteRef>,
    pub text: Option<UiTextItem>,
    pub text_layout: TextLayoutConfig,
    pub text_clip_rect: Option<[f32; 4]>,
    pub clip_rect: Option<[f32; 4]>,
}

impl unitas_render::ui_pass::UiElement for UiDrawItem {
    fn rect(&self) -> [f32; 4] {
        self.rect_px
    }

    fn color(&self) -> [f32; 4] {
        self.color
    }

    fn sprite(&self) -> Option<&SpriteRef> {
        self.sprite.as_ref()
    }

    fn clip_rect(&self) -> Option<[f32; 4]> {
        self.clip_rect
    }

    fn text(&self) -> Option<unitas_render::ui_pass::UiText<'_>> {
        self.text.as_ref().map(|item| {
            let wrap = self.text_layout.horizontal_overflow == HorizontalWrapMode::Wrap;
            unitas_render::ui_pass::UiText {
                text: &item.text,
                x: item.x,
                y: item.y,
                size_px: item.size_px,
                color: item.color,
                horizontal_align: self.text_layout.alignment.horizontal(),
                vertical_align: self.text_layout.alignment.vertical(),
                wrap,
                max_width: wrap.then_some(self.rect_px[2]),
                line_spacing: self.text_layout.line_spacing,
                clip_rect: self.text_clip_rect,
            }
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CanvasRect {
    pub min: [f32; 2],
    pub size: [f32; 2],
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpriteQuad {
    pub rect: [f32; 4],
    pub uv: [f32; 4],
}

fn clamp_sliced_borders(border: [f32; 4], rect: [f32; 4], scale: f32) -> (f32, f32, f32, f32) {
    let (mut left, mut bottom, mut right, mut top) = (
        border[0] * scale,
        border[1] * scale,
        border[2] * scale,
        border[3] * scale,
    );
    let (width, height) = (rect[2], rect[3]);
    if left + right > width {
        let difference = left - right;
        left = 0.5 * width + 0.5 * difference;
        right = 0.5 * width - 0.5 * difference;
    }
    if bottom + top > height {
        let difference = top - bottom;
        bottom = 0.5 * height - 0.5 * difference;
        top = 0.5 * height + 0.5 * difference;
    }
    (left, bottom, right, top)
}

pub fn slice_sprite_quads(
    border: [f32; 4],
    sprite_size: [f32; 2],
    rect: [f32; 4],
    uv_rect: [f32; 4],
    scale: f32,
) -> Vec<SpriteQuad> {
    let (left, bottom, right, top) = clamp_sliced_borders(border, rect, scale);
    let (width, height) = (rect[2], rect[3]);
    let (u0, v0, u1, v1) = (uv_rect[0], uv_rect[1], uv_rect[2], uv_rect[3]);
    let (u_span, v_span) = (u1 - u0, v1 - v0);
    let u_left = u0 + border[0] / sprite_size[0] * u_span;
    let u_right = u1 - border[2] / sprite_size[0] * u_span;
    let v_top = v0 + border[3] / sprite_size[1] * v_span;
    let v_bottom = v1 - border[1] / sprite_size[1] * v_span;
    let (x, y) = (rect[0], rect[1]);
    let (mid_x, end_x) = (x + left, x + width - right);
    let (mid_y, end_y) = (y + top, y + height - bottom);
    let (mid_w, mid_h) = (end_x - mid_x, end_y - mid_y);
    let quad = |r, uv| SpriteQuad { rect: r, uv };
    vec![
        quad([x, y, left, top], [u0, v0, u_left, v_top]),
        quad([mid_x, y, mid_w, top], [u_left, v0, u_right, v_top]),
        quad([end_x, y, right, top], [u_right, v0, u1, v_top]),
        quad([x, mid_y, left, mid_h], [u0, v_top, u_left, v_bottom]),
        quad(
            [mid_x, mid_y, mid_w, mid_h],
            [u_left, v_top, u_right, v_bottom],
        ),
        quad([end_x, mid_y, right, mid_h], [u_right, v_top, u1, v_bottom]),
        quad([x, end_y, left, bottom], [u0, v_bottom, u_left, v1]),
        quad(
            [mid_x, end_y, mid_w, bottom],
            [u_left, v_bottom, u_right, v1],
        ),
        quad([end_x, end_y, right, bottom], [u_right, v_bottom, u1, v1]),
    ]
}

pub fn tile_sprite_quads(
    sprite_size: [f32; 2],
    rect: [f32; 4],
    uv_rect: [f32; 4],
    scale: f32,
) -> Vec<SpriteQuad> {
    let tile_width = sprite_size[0] * scale;
    let tile_height = sprite_size[1] * scale;
    if tile_width <= 0.0 || tile_height <= 0.0 {
        return Vec::new();
    }
    let columns = (rect[2] / tile_width).ceil().max(1.0) as usize;
    let rows = (rect[3] / tile_height).ceil().max(1.0) as usize;
    let mut quads = Vec::with_capacity(columns * rows);
    for row in 0..rows {
        for column in 0..columns {
            let x = rect[0] + column as f32 * tile_width;
            let y = rect[1] + row as f32 * tile_height;
            let width = tile_width.min(rect[0] + rect[2] - x);
            let height = tile_height.min(rect[1] + rect[3] - y);
            if width <= 0.0 || height <= 0.0 {
                continue;
            }
            quads.push(SpriteQuad {
                rect: [x, y, width, height],
                uv: uv_rect,
            });
        }
    }
    quads
}

#[derive(Clone, Debug, PartialEq)]
pub struct TextOverride {
    pub text: String,
    pub font_size: f32,
    pub color: [f32; 4],
    pub layout: TextLayoutConfig,
}

pub fn resolve_canvas_rect(rect: &RectTransformComponent, parent: CanvasRect) -> CanvasRect {
    let anchor_size = [
        (rect.anchor_max[0] - rect.anchor_min[0]) * parent.size[0],
        (rect.anchor_max[1] - rect.anchor_min[1]) * parent.size[1],
    ];
    let size = [
        anchor_size[0] + rect.size_delta[0],
        anchor_size[1] + rect.size_delta[1],
    ];
    let pivot = [
        rect.anchor_min[0] * parent.size[0]
            + anchor_size[0] * rect.pivot[0]
            + rect.anchored_position[0],
        rect.anchor_min[1] * parent.size[1]
            + anchor_size[1] * rect.pivot[1]
            + rect.anchored_position[1],
    ];
    CanvasRect {
        min: [
            parent.min[0] + pivot[0] - rect.pivot[0] * size[0],
            parent.min[1] + pivot[1] - rect.pivot[1] * size[1],
        ],
        size,
    }
}

pub const FALLBACK_SCREEN_DPI: f32 = 96.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum PhysicalUnit {
    #[default]
    Centimeters,
    Millimeters,
    Inches,
    Points,
    Picas,
}

impl PhysicalUnit {
    pub const fn from_u32(code: u32) -> Option<Self> {
        match code {
            0 => Some(Self::Centimeters),
            1 => Some(Self::Millimeters),
            2 => Some(Self::Inches),
            3 => Some(Self::Points),
            4 => Some(Self::Picas),
            _ => None,
        }
    }

    pub const fn units_per_inch(self) -> f32 {
        match self {
            Self::Centimeters => 2.54,
            Self::Millimeters => 25.4,
            Self::Inches => 1.0,
            Self::Points => 72.0,
            Self::Picas => 6.0,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CanvasScalerOverride {
    pub entity: EntityId,
    pub ui_scale_mode: CanvasScaleMode,
    pub scale_factor: f32,
    pub reference_resolution: [f32; 2],
    pub screen_match_mode: CanvasMatchMode,
    pub match_width_or_height: f32,
    pub physical_unit: PhysicalUnit,
    pub fallback_dpi: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct UiToggleComponent {
    pub entity: EntityId,
    pub is_on: bool,
    pub interactable: bool,
    pub enabled: bool,
    pub graphic: Option<EntityId>,
    pub group: Option<EntityId>,
}

impl Default for UiToggleComponent {
    fn default() -> Self {
        Self {
            entity: EntityId::new(0),
            is_on: false,
            interactable: true,
            enabled: true,
            graphic: None,
            group: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToggleValueChangedEvent {
    pub entity: EntityId,
    pub is_on: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum SliderDirection {
    #[default]
    LeftToRight,
    RightToLeft,
    BottomToTop,
    TopToBottom,
}

impl SliderDirection {
    pub const fn from_u32(code: u32) -> Option<Self> {
        match code {
            0 => Some(Self::LeftToRight),
            1 => Some(Self::RightToLeft),
            2 => Some(Self::BottomToTop),
            3 => Some(Self::TopToBottom),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct UiSliderComponent {
    pub entity: EntityId,
    pub value: f32,
    pub min_value: f32,
    pub max_value: f32,
    pub whole_numbers: bool,
    pub interactable: bool,
    pub enabled: bool,
    pub direction: SliderDirection,
    pub fill_rect: Option<EntityId>,
    pub handle_rect: Option<EntityId>,
}

impl Default for UiSliderComponent {
    fn default() -> Self {
        Self {
            entity: EntityId::new(0),
            value: 0.0,
            min_value: 0.0,
            max_value: 1.0,
            whole_numbers: false,
            interactable: true,
            enabled: true,
            direction: SliderDirection::LeftToRight,
            fill_rect: None,
            handle_rect: None,
        }
    }
}

impl UiSliderComponent {
    pub fn normalized_value(&self) -> f32 {
        let span = self.max_value - self.min_value;
        if span <= 0.0 {
            return 0.0;
        }
        ((self.value - self.min_value) / span).clamp(0.0, 1.0)
    }

    pub fn set_normalized_value(&mut self, t: f32) {
        let mut value = self.min_value + t.clamp(0.0, 1.0) * (self.max_value - self.min_value);
        if self.whole_numbers {
            value = value.round();
        }
        let low = self.min_value.min(self.max_value);
        let high = self.min_value.max(self.max_value);
        self.value = value.clamp(low, high);
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SliderValueChangedEvent {
    pub entity: EntityId,
    pub value: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct UiCanvasGroupComponent {
    pub entity: EntityId,
    pub alpha: f32,
    pub interactable: bool,
    pub blocks_raycasts: bool,
    pub ignore_parent_groups: bool,
    pub enabled: bool,
}

impl Default for UiCanvasGroupComponent {
    fn default() -> Self {
        Self {
            entity: EntityId::new(0),
            alpha: 1.0,
            interactable: true,
            blocks_raycasts: true,
            ignore_parent_groups: false,
            enabled: true,
        }
    }
}

pub fn canvas_scale_factor(canvas: &CanvasComponent, screen_size: [f32; 2]) -> f32 {
    let scaler = match canvas.scaler.as_ref() {
        Some(scaler) => scaler,
        None => return 1.0,
    };
    if scaler.ui_scale_mode != CanvasScaleMode::ScaleWithScreenSize {
        return 1.0;
    }
    screen_size_scale(
        scaler.reference_resolution,
        scaler.screen_match_mode,
        scaler.match_width_or_height,
        screen_size,
    )
}

fn screen_size_scale(
    reference: [f32; 2],
    match_mode: CanvasMatchMode,
    match_value: f32,
    screen_size: [f32; 2],
) -> f32 {
    if reference[0] <= 0.0 || reference[1] <= 0.0 {
        return 1.0;
    }
    let width_ratio = screen_size[0] / reference[0];
    let height_ratio = screen_size[1] / reference[1];
    match match_mode {
        CanvasMatchMode::MatchWidthOrHeight => {
            let t = match_value.clamp(0.0, 1.0);
            let log_w = width_ratio.max(f32::MIN_POSITIVE).log2();
            let log_h = height_ratio.max(f32::MIN_POSITIVE).log2();
            2f32.powf(log_w + (log_h - log_w) * t)
        }
        CanvasMatchMode::Shrink => width_ratio.min(height_ratio),
        CanvasMatchMode::Expand => width_ratio.max(height_ratio),
    }
}

fn physical_size_scale(unit: PhysicalUnit, fallback_dpi: f32, screen_dpi: f32) -> f32 {
    let dpi = if screen_dpi > 0.0 {
        screen_dpi
    } else {
        fallback_dpi
    };
    dpi / unit.units_per_inch()
}

fn scaler_override_scale(
    override_: &CanvasScalerOverride,
    screen_size: [f32; 2],
    screen_dpi: f32,
) -> f32 {
    match override_.ui_scale_mode {
        CanvasScaleMode::ConstantPixelSize => override_.scale_factor,
        CanvasScaleMode::ConstantPhysicalSize => {
            physical_size_scale(override_.physical_unit, override_.fallback_dpi, screen_dpi)
        }
        CanvasScaleMode::ScaleWithScreenSize => screen_size_scale(
            override_.reference_resolution,
            override_.screen_match_mode,
            override_.match_width_or_height,
            screen_size,
        ),
    }
}

pub struct CanvasState {
    binding: UiBinding,
    rects: HashMap<EntityId, RectTransformComponent>,
    focused: Option<EntityId>,
    focus_axis_prev: [f64; 2],
    pending_clicks: Vec<u64>,
    element_enabled: HashMap<EntityId, bool>,
    button_interactable: HashMap<EntityId, bool>,
    input_fields: HashMap<EntityId, UiInputFieldComponent>,
    input_field_order: Vec<EntityId>,
    pending_value_changed: Vec<InputFieldValueChangedEvent>,
    pending_end_edit: Vec<InputFieldEndEditEvent>,
    toggles: HashMap<EntityId, UiToggleComponent>,
    toggle_order: Vec<EntityId>,
    pending_toggle_changed: Vec<ToggleValueChangedEvent>,
    pressed_toggle: Option<EntityId>,
    sliders: HashMap<EntityId, UiSliderComponent>,
    slider_order: Vec<EntityId>,
    pending_slider_changed: Vec<SliderValueChangedEvent>,
    pressed_slider: Option<EntityId>,
    rect_masks: HashMap<EntityId, bool>,
    canvas_groups: HashMap<EntityId, UiCanvasGroupComponent>,
    scaler_overrides: RwLock<HashMap<EntityId, CanvasScalerOverride>>,
    screen_dpi: f32,
    layout_group_overrides: RwLock<HashMap<EntityId, UiLayoutGroupComponent>>,
    fitter_overrides: RwLock<HashMap<EntityId, UiContentSizeFitterComponent>>,
}

impl CanvasState {
    pub fn new(binding: UiBinding) -> Self {
        let rects = binding
            .rect_transforms
            .iter()
            .map(|rect| (rect.entity, rect.clone()))
            .collect();
        Self {
            binding,
            rects,
            focused: None,
            focus_axis_prev: [0.0; 2],
            pending_clicks: Vec::new(),
            element_enabled: HashMap::new(),
            button_interactable: HashMap::new(),
            input_fields: HashMap::new(),
            input_field_order: Vec::new(),
            pending_value_changed: Vec::new(),
            pending_end_edit: Vec::new(),
            toggles: HashMap::new(),
            toggle_order: Vec::new(),
            pending_toggle_changed: Vec::new(),
            pressed_toggle: None,
            sliders: HashMap::new(),
            slider_order: Vec::new(),
            pending_slider_changed: Vec::new(),
            pressed_slider: None,
            rect_masks: HashMap::new(),
            canvas_groups: HashMap::new(),
            scaler_overrides: RwLock::new(HashMap::new()),
            screen_dpi: 0.0,
            layout_group_overrides: RwLock::new(HashMap::new()),
            fitter_overrides: RwLock::new(HashMap::new()),
        }
    }

    pub fn binding(&self) -> &UiBinding {
        &self.binding
    }

    pub fn ensure_canvas_and_rect(&mut self, entity: EntityId, screen_size: [f32; 2]) {
        if self.binding.canvases.is_empty() {
            let canvas_entity = EntityId::new(u64::MAX - 1);
            self.binding.canvases.push(CanvasComponent {
                entity: canvas_entity,
                render_mode: CanvasRenderMode::ScreenSpaceOverlay,
                sorting_order: 0,
                scaler: None,
            });
            let canvas_rect = RectTransformComponent {
                entity: canvas_entity,
                parent: None,
                anchor_min: [0.5, 0.5],
                anchor_max: [0.5, 0.5],
                anchored_position: [0.0, 0.0],
                size_delta: screen_size,
                pivot: [0.5, 0.5],
            };
            self.binding.rect_transforms.push(canvas_rect.clone());
            self.rects.insert(canvas_entity, canvas_rect);
        }
        if !self.rects.contains_key(&entity) {
            let parent = self.binding.canvases.first().map(|c| c.entity);
            let rect = RectTransformComponent {
                entity,
                parent,
                anchor_min: [0.5, 0.5],
                anchor_max: [0.5, 0.5],
                anchored_position: [0.0, 0.0],
                size_delta: [240.0, 36.0],
                pivot: [0.5, 0.5],
            };
            self.binding.rect_transforms.push(rect.clone());
            self.rects.insert(entity, rect);
        }
    }

    pub fn set_input_field(&mut self, mut field: UiInputFieldComponent) {
        field.caret_position =
            UiInputFieldComponent::clamp_caret(&field.text, field.caret_position);
        let entity = field.entity;
        if let Some(existing) = self.input_fields.get(&entity)
            && field.editing_original_text.is_none()
        {
            field.editing_original_text = existing.editing_original_text.clone();
        }
        if !self.input_field_order.contains(&entity) {
            self.input_field_order.push(entity);
        }
        self.input_fields.insert(entity, field);
    }

    pub fn input_field(&self, entity: EntityId) -> Option<&UiInputFieldComponent> {
        self.input_fields.get(&entity)
    }

    pub fn input_field_mut(&mut self, entity: EntityId) -> Option<&mut UiInputFieldComponent> {
        self.input_fields.get_mut(&entity)
    }

    pub fn set_input_field_text(&mut self, entity: EntityId, text: &str, notify: bool) -> bool {
        let Some(field) = self.input_fields.get_mut(&entity) else {
            return false;
        };
        let mut truncated = text.to_string();
        if field.character_limit > 0 && truncated.chars().count() > field.character_limit {
            truncated = truncated.chars().take(field.character_limit).collect();
        }
        let changed = field.text != truncated;
        field.text = truncated;
        field.caret_position = field.text.len();
        if changed && notify {
            self.pending_value_changed
                .push(InputFieldValueChangedEvent {
                    entity,
                    text: field.text.clone(),
                });
        }
        true
    }

    pub fn set_input_field_placeholder(&mut self, entity: EntityId, placeholder: &str) -> bool {
        let Some(field) = self.input_fields.get_mut(&entity) else {
            return false;
        };
        field.placeholder = placeholder.to_string();
        true
    }

    pub fn set_input_field_interactable(&mut self, entity: EntityId, interactable: bool) -> bool {
        let Some(field) = self.input_fields.get_mut(&entity) else {
            return false;
        };
        field.interactable = interactable;
        if !interactable && self.focused == Some(entity) {
            field.editing_original_text = None;
            self.focused = None;
        }
        true
    }

    pub fn set_input_field_character_limit(&mut self, entity: EntityId, limit: usize) -> bool {
        let Some(field) = self.input_fields.get_mut(&entity) else {
            return false;
        };
        field.character_limit = limit;
        if limit > 0 && field.text.chars().count() > limit {
            field.text = field.text.chars().take(limit).collect();
            field.caret_position =
                UiInputFieldComponent::clamp_caret(&field.text, field.caret_position);
        }
        true
    }

    pub fn set_input_field_caret_position(&mut self, entity: EntityId, caret: usize) -> bool {
        let Some(field) = self.input_fields.get_mut(&entity) else {
            return false;
        };
        field.caret_position = UiInputFieldComponent::clamp_caret(&field.text, caret);
        true
    }

    pub fn focus_input_field(&mut self, entity: EntityId, focused: bool) -> bool {
        let Some(field) = self.input_fields.get_mut(&entity) else {
            return false;
        };
        if focused {
            if !field.interactable || !field.enabled {
                return false;
            }
            if field.editing_original_text.is_none() {
                field.editing_original_text = Some(field.text.clone());
            }
            field.was_canceled = false;
            field.caret_position =
                UiInputFieldComponent::clamp_caret(&field.text, field.text.len());
            self.focused = Some(entity);
            true
        } else {
            let was_editing = field.editing_original_text.take().is_some();
            if self.focused == Some(entity) {
                self.focused = None;
            }
            if was_editing {
                let text = field.text.clone();
                let canceled = field.was_canceled;
                self.pending_end_edit.push(InputFieldEndEditEvent {
                    entity,
                    text,
                    canceled,
                });
            }
            true
        }
    }

    pub fn take_input_field_value_changed(&mut self) -> Vec<InputFieldValueChangedEvent> {
        std::mem::take(&mut self.pending_value_changed)
    }

    pub fn take_input_field_end_edit(&mut self) -> Vec<InputFieldEndEditEvent> {
        std::mem::take(&mut self.pending_end_edit)
    }

    pub fn set_screen_dpi(&mut self, dpi: f32) {
        if dpi.is_finite() && dpi >= 0.0 {
            self.screen_dpi = dpi;
        }
    }

    pub fn set_canvas_scaler(&self, scaler: CanvasScalerOverride) -> bool {
        let Ok(mut guard) = self.scaler_overrides.write() else {
            return false;
        };
        guard.insert(scaler.entity, scaler);
        true
    }

    fn active_scaler_scale(&self, canvas: &CanvasComponent, screen_size: [f32; 2]) -> f32 {
        if let Ok(guard) = self.scaler_overrides.read()
            && let Some(override_) = guard.get(&canvas.entity)
        {
            return scaler_override_scale(override_, screen_size, self.screen_dpi);
        }
        canvas_scale_factor(canvas, screen_size)
    }

    pub fn set_toggle(&mut self, toggle: UiToggleComponent) {
        let entity = toggle.entity;
        if !self.toggle_order.contains(&entity) {
            self.toggle_order.push(entity);
        }
        self.toggles.insert(entity, toggle);
    }

    pub fn toggle(&self, entity: EntityId) -> Option<&UiToggleComponent> {
        self.toggles.get(&entity)
    }

    pub fn set_toggle_value(&mut self, entity: EntityId, is_on: bool, notify: bool) -> bool {
        let Some(toggle) = self.toggles.get_mut(&entity) else {
            return false;
        };
        let changed = toggle.is_on != is_on;
        toggle.is_on = is_on;
        let group = toggle.group;
        if is_on && let Some(group) = group {
            let peers: Vec<EntityId> = self
                .toggles
                .iter()
                .filter(|(id, other)| **id != entity && other.group == Some(group) && other.is_on)
                .map(|(id, _)| *id)
                .collect();
            for peer in peers {
                if let Some(peer_toggle) = self.toggles.get_mut(&peer) {
                    peer_toggle.is_on = false;
                }
            }
        }
        if changed && notify {
            self.pending_toggle_changed
                .push(ToggleValueChangedEvent { entity, is_on });
        }
        true
    }

    pub fn take_toggle_value_changed(&mut self) -> Vec<ToggleValueChangedEvent> {
        std::mem::take(&mut self.pending_toggle_changed)
    }

    pub fn hit_test_toggle(&self, position: (f32, f32), screen_size: [f32; 2]) -> Option<EntityId> {
        self.hit_test_control(
            &self.toggle_order,
            position,
            screen_size,
            |canvas, entity| {
                canvas.toggles.get(&entity).is_some_and(|toggle| {
                    toggle.interactable
                        && canvas.is_element_enabled(entity, toggle.enabled)
                        && canvas.group_interactable(entity)
                })
            },
        )
    }

    pub fn set_slider(&mut self, slider: UiSliderComponent) {
        let entity = slider.entity;
        if !self.slider_order.contains(&entity) {
            self.slider_order.push(entity);
        }
        self.sliders.insert(entity, slider);
    }

    pub fn slider(&self, entity: EntityId) -> Option<&UiSliderComponent> {
        self.sliders.get(&entity)
    }

    pub fn set_slider_value(&mut self, entity: EntityId, value: f32, notify: bool) -> bool {
        if !value.is_finite() {
            return false;
        }
        let Some(slider) = self.sliders.get_mut(&entity) else {
            return false;
        };
        let low = slider.min_value.min(slider.max_value);
        let high = slider.min_value.max(slider.max_value);
        let mut clamped = value.clamp(low, high);
        if slider.whole_numbers {
            clamped = clamped.round();
        }
        let changed = (clamped - slider.value).abs() > f32::EPSILON;
        slider.value = clamped;
        if changed && notify {
            self.pending_slider_changed.push(SliderValueChangedEvent {
                entity,
                value: clamped,
            });
        }
        true
    }

    pub fn take_slider_value_changed(&mut self) -> Vec<SliderValueChangedEvent> {
        std::mem::take(&mut self.pending_slider_changed)
    }

    pub fn hit_test_slider(&self, position: (f32, f32), screen_size: [f32; 2]) -> Option<EntityId> {
        self.hit_test_control(
            &self.slider_order,
            position,
            screen_size,
            |canvas, entity| {
                canvas.sliders.get(&entity).is_some_and(|slider| {
                    slider.interactable
                        && canvas.is_element_enabled(entity, slider.enabled)
                        && canvas.group_interactable(entity)
                })
            },
        )
    }

    fn hit_test_control(
        &self,
        order: &[EntityId],
        position: (f32, f32),
        screen_size: [f32; 2],
        allowed: impl Fn(&Self, EntityId) -> bool,
    ) -> Option<EntityId> {
        let (px, py) = position;
        for &entity in order.iter().rev() {
            if !allowed(self, entity) || !self.group_blocks_raycasts(entity) {
                continue;
            }
            let Some((rect, _)) = self.element_rect(entity, screen_size) else {
                continue;
            };
            if px < rect[0] || py < rect[1] || px > rect[0] + rect[2] || py > rect[1] + rect[3] {
                continue;
            }
            if self.mask_allows_point(entity, position, screen_size) {
                return Some(entity);
            }
        }
        None
    }

    pub fn press_pointer(&mut self, position: (f32, f32), screen_size: [f32; 2]) {
        self.pressed_toggle = self.hit_test_toggle(position, screen_size);
        self.pressed_slider = self.hit_test_slider(position, screen_size);
        if let Some(slider) = self.pressed_slider {
            self.drag_slider_to(slider, position, screen_size);
        }
    }

    pub fn move_pointer(&mut self, position: (f32, f32), screen_size: [f32; 2]) {
        if let Some(slider) = self.pressed_slider {
            self.drag_slider_to(slider, position, screen_size);
        }
    }

    pub fn release_pointer(&mut self, position: (f32, f32), screen_size: [f32; 2]) {
        if let Some(slider) = self.pressed_slider.take() {
            self.drag_slider_to(slider, position, screen_size);
        }
        if let Some(toggle) = self.pressed_toggle.take()
            && self.hit_test_toggle(position, screen_size) == Some(toggle)
        {
            let is_on = !self.toggles.get(&toggle).is_some_and(|t| t.is_on);
            let _ = self.set_toggle_value(toggle, is_on, true);
        }
    }

    fn drag_slider_to(&mut self, entity: EntityId, position: (f32, f32), screen_size: [f32; 2]) {
        let Some((rect, _)) = self.element_rect(entity, screen_size) else {
            return;
        };
        let (px, py) = position;
        let t = {
            let Some(slider) = self.sliders.get(&entity) else {
                return;
            };
            match slider.direction {
                SliderDirection::LeftToRight => (px - rect[0]) / rect[2],
                SliderDirection::RightToLeft => (rect[0] + rect[2] - px) / rect[2],
                SliderDirection::BottomToTop => (rect[1] + rect[3] - py) / rect[3],
                SliderDirection::TopToBottom => (py - rect[1]) / rect[3],
            }
        };
        let Some(slider) = self.sliders.get_mut(&entity) else {
            return;
        };
        let before = slider.value;
        slider.set_normalized_value(t);
        if (slider.value - before).abs() > f32::EPSILON {
            let value = slider.value;
            self.pending_slider_changed
                .push(SliderValueChangedEvent { entity, value });
        }
    }

    pub fn set_rect_mask(&mut self, entity: EntityId, enabled: bool) {
        self.rect_masks.insert(entity, enabled);
    }

    fn mask_rect_from(
        &self,
        entity: EntityId,
        resolved: &HashMap<EntityId, ([f32; 4], f32)>,
    ) -> Option<[f32; 4]> {
        let chain = self.chain_to_canvas(entity)?;
        for node in chain.iter().rev().skip(1) {
            if self.rect_masks.get(node).copied().unwrap_or(false)
                && let Some((rect, _)) = resolved.get(node).copied()
            {
                return Some(rect);
            }
        }
        None
    }

    fn mask_allows_point(
        &self,
        entity: EntityId,
        position: (f32, f32),
        screen_size: [f32; 2],
    ) -> bool {
        let Some(chain) = self.chain_to_canvas(entity) else {
            return false;
        };
        for node in chain.iter().rev().skip(1) {
            if self.rect_masks.get(node).copied().unwrap_or(false)
                && let Some((rect, _)) = self.element_rect(*node, screen_size)
            {
                let (px, py) = position;
                if px < rect[0] || py < rect[1] || px > rect[0] + rect[2] || py > rect[1] + rect[3]
                {
                    return false;
                }
            }
        }
        true
    }

    pub fn set_canvas_group(&mut self, group: UiCanvasGroupComponent) -> bool {
        if group.entity.as_u64() == 0 {
            return false;
        }
        self.canvas_groups.insert(group.entity, group);
        true
    }

    fn applicable_groups(&self, entity: EntityId) -> Vec<&UiCanvasGroupComponent> {
        let mut groups = Vec::new();
        if let Some(chain) = self.chain_to_canvas(entity) {
            for node in chain {
                if let Some(group) = self.canvas_groups.get(&node).filter(|group| group.enabled) {
                    if group.ignore_parent_groups {
                        groups.clear();
                    }
                    groups.push(group);
                }
            }
        }
        groups
    }

    fn group_alpha(&self, entity: EntityId) -> f32 {
        self.applicable_groups(entity)
            .iter()
            .fold(1.0, |alpha, group| alpha * group.alpha.clamp(0.0, 1.0))
    }

    fn group_interactable(&self, entity: EntityId) -> bool {
        self.applicable_groups(entity)
            .iter()
            .all(|group| group.interactable)
    }

    fn group_blocks_raycasts(&self, entity: EntityId) -> bool {
        self.applicable_groups(entity)
            .iter()
            .all(|group| group.blocks_raycasts)
    }

    pub fn element_receives_pointer(
        &self,
        entity: EntityId,
        screen_size: [f32; 2],
        point: (f32, f32),
    ) -> bool {
        if !self.group_blocks_raycasts(entity) {
            return false;
        }
        let Some((rect, _)) = self.element_rect(entity, screen_size) else {
            return false;
        };
        let (px, py) = point;
        px >= rect[0]
            && py >= rect[1]
            && px <= rect[0] + rect[2]
            && py <= rect[1] + rect[3]
            && self.mask_allows_point(entity, point, screen_size)
    }

    pub fn set_element_enabled(&mut self, entity: EntityId, enabled: bool) {
        self.element_enabled.insert(entity, enabled);
    }

    pub fn set_button_interactable(&mut self, entity: EntityId, interactable: bool) {
        self.button_interactable.insert(entity, interactable);
    }

    pub fn set_focused_button(&mut self, entity: Option<EntityId>) {
        if let Some(prev) = self.focused
            && Some(prev) != entity
            && let Some(prev_field) = self.input_fields.get_mut(&prev)
        {
            prev_field.editing_original_text = None;
        }
        if let Some(target) = entity
            && let Some(field) = self.input_fields.get_mut(&target)
            && field.editing_original_text.is_none()
        {
            field.editing_original_text = Some(field.text.clone());
            field.was_canceled = false;
        }
        self.focused = entity;
    }

    pub fn set_layout_group(&self, group: UiLayoutGroupComponent) -> bool {
        let Ok(mut guard) = self.layout_group_overrides.write() else {
            return false;
        };
        guard.insert(group.entity, group);
        true
    }

    pub fn set_content_size_fitter(&self, fitter: UiContentSizeFitterComponent) -> bool {
        let Ok(mut guard) = self.fitter_overrides.write() else {
            return false;
        };
        guard.insert(fitter.entity, fitter);
        true
    }

    pub fn focusable_buttons(&self, screen_size: [f32; 2]) -> Vec<EntityId> {
        let mut focusables = Vec::new();
        if screen_size[0] <= 0.0 || screen_size[1] <= 0.0 {
            return focusables;
        }
        for element in &self.binding.elements {
            let UiElementComponent::Button(button) = element else {
                continue;
            };
            if self.is_button_focusable(button, screen_size) && !focusables.contains(&button.entity)
            {
                focusables.push(button.entity);
            }
        }
        for &entity in &self.input_field_order {
            if let Some(field) = self.input_fields.get(&entity)
                && self.is_input_field_focusable(field, screen_size)
                && !focusables.contains(&entity)
            {
                focusables.push(entity);
            }
        }
        focusables
    }

    pub fn focused_button(&self, screen_size: [f32; 2]) -> Option<EntityId> {
        let focusables = self.focusable_buttons(screen_size);
        self.focused
            .filter(|id| focusables.contains(id))
            .or_else(|| focusables.first().copied())
    }

    pub fn take_focused_clicks(&mut self) -> Vec<u64> {
        std::mem::take(&mut self.pending_clicks)
    }

    pub fn hit_test_button(&self, position: (f32, f32), screen_size: [f32; 2]) -> Option<EntityId> {
        let (px, py) = position;
        for entity in self.focusable_buttons(screen_size).into_iter().rev() {
            if self.input_fields.contains_key(&entity) {
                continue;
            }
            if !self.group_blocks_raycasts(entity) {
                continue;
            }
            let Some((rect, _)) = self.element_rect(entity, screen_size) else {
                continue;
            };
            if px >= rect[0]
                && py >= rect[1]
                && px <= rect[0] + rect[2]
                && py <= rect[1] + rect[3]
                && self.mask_allows_point(entity, position, screen_size)
            {
                return Some(entity);
            }
        }
        None
    }

    pub fn hit_test_input_field(
        &self,
        position: (f32, f32),
        screen_size: [f32; 2],
    ) -> Option<EntityId> {
        let (px, py) = position;
        for &entity in self.input_field_order.iter().rev() {
            let Some(field) = self.input_fields.get(&entity) else {
                continue;
            };
            if !self.is_input_field_focusable(field, screen_size)
                || !self.group_blocks_raycasts(entity)
            {
                continue;
            }
            let Some((rect, _)) = self.element_rect(entity, screen_size) else {
                continue;
            };
            if px >= rect[0]
                && py >= rect[1]
                && px <= rect[0] + rect[2]
                && py <= rect[1] + rect[3]
                && self.mask_allows_point(entity, position, screen_size)
            {
                return Some(entity);
            }
        }
        None
    }

    fn current_focus_target(&mut self, focusables: &[EntityId]) -> Option<EntityId> {
        let current = self
            .focused
            .filter(|id| focusables.contains(id))
            .or_else(|| focusables.first().copied());
        if let Some(target) = current
            && let Some(field) = self.input_fields.get_mut(&target)
            && field.editing_original_text.is_none()
        {
            field.editing_original_text = Some(field.text.clone());
            field.was_canceled = false;
        }
        current
    }

    fn collect_submit_click(&mut self, input: &InputState) -> Vec<EntityId> {
        let submit = input
            .button_state("Submit")
            .is_some_and(|state| state & STATE_PRESSED != 0)
            || input.is_key_pressed(KEY_RETURN)
            || input.is_key_pressed(KEY_SPACE)
            || input.is_gamepad_button_pressed(GAMEPAD_BUTTON_SOUTH);
        let mut clicked = Vec::new();
        if submit
            && let Some(entity) = self.focused
            && !self.input_fields.contains_key(&entity)
        {
            self.pending_clicks.push(entity.as_u64());
            clicked.push(entity);
        }
        clicked
    }

    pub fn handle_focus(&mut self, input: &InputState, screen_size: [f32; 2]) -> Vec<EntityId> {
        let focusables = self.focusable_buttons(screen_size);
        let current = self.current_focus_target(&focusables);
        self.focused = current;

        let stick_x = input.gamepad_axis(GAMEPAD_AXIS_LEFT_X);
        let stick_y = input.gamepad_axis(GAMEPAD_AXIS_LEFT_Y);
        let horizontal = input.axis_raw("Horizontal").unwrap_or(stick_x);
        let vertical = input.axis_raw("Vertical").unwrap_or(stick_y);
        let prev_x = self.focus_axis_prev[0];
        let prev_y = self.focus_axis_prev[1];
        self.focus_axis_prev = [horizontal, vertical];

        if !focusables.is_empty() {
            let shift_down =
                input.is_key_down(KEY_LEFT_SHIFT) || input.is_key_down(KEY_RIGHT_SHIFT);
            if input.is_key_pressed(KEY_TAB) {
                let step = if shift_down { -1 } else { 1 };
                let next = step_focus(&focusables, current, step);
                self.set_focused_button(Some(next));
                let _ = input.flush_queued_chars();
                return Vec::new();
            }
        }

        if let Some(focused_id) = self.focused
            && self.input_fields.contains_key(&focused_id)
        {
            self.handle_input_field_editing(focused_id, input);
            return Vec::new();
        }

        let _ = input.flush_queued_chars();

        if !focusables.is_empty()
            && let Some(direction) =
                navigation_direction(input, [prev_x, prev_y], [horizontal, vertical])
        {
            let next = self.navigate_direction(&focusables, current, direction, screen_size);
            self.set_focused_button(Some(next));
        }

        self.collect_submit_click(input)
    }

    fn begin_field_edit(&mut self, entity: EntityId) -> bool {
        let Some(field) = self.input_fields.get_mut(&entity) else {
            return false;
        };
        if field.editing_original_text.is_none() {
            field.editing_original_text = Some(field.text.clone());
            field.was_canceled = false;
        }
        field.caret_position =
            UiInputFieldComponent::clamp_caret(&field.text, field.caret_position);
        true
    }

    fn insert_field_chars(&mut self, entity: EntityId, chars: &[char]) {
        let Some(field) = self.input_fields.get_mut(&entity) else {
            return;
        };
        for &ch in chars {
            if field.character_limit == 0 || field.text.chars().count() < field.character_limit {
                field.text.insert(field.caret_position, ch);
                field.caret_position += ch.len_utf8();
                self.pending_value_changed
                    .push(InputFieldValueChangedEvent {
                        entity,
                        text: field.text.clone(),
                    });
            }
        }
    }

    fn delete_field_chars(&mut self, entity: EntityId, input: &InputState) {
        let Some(field) = self.input_fields.get_mut(&entity) else {
            return;
        };
        if input.is_key_pressed(KEY_BACKSPACE) && field.caret_position > 0 {
            let prev = prev_char_boundary(&field.text, field.caret_position);
            field.text.replace_range(prev..field.caret_position, "");
            field.caret_position = prev;
            self.pending_value_changed
                .push(InputFieldValueChangedEvent {
                    entity,
                    text: field.text.clone(),
                });
        }

        if input.is_key_pressed(KEY_DELETE) && field.caret_position < field.text.len() {
            let next = next_char_boundary(&field.text, field.caret_position);
            field.text.replace_range(field.caret_position..next, "");
            self.pending_value_changed
                .push(InputFieldValueChangedEvent {
                    entity,
                    text: field.text.clone(),
                });
        }
    }

    fn move_field_caret(&mut self, entity: EntityId, input: &InputState) {
        let Some(field) = self.input_fields.get_mut(&entity) else {
            return;
        };
        if input.is_key_pressed(KEY_LEFT_ARROW) && field.caret_position > 0 {
            field.caret_position = prev_char_boundary(&field.text, field.caret_position);
        }
        if input.is_key_pressed(KEY_RIGHT_ARROW) && field.caret_position < field.text.len() {
            field.caret_position = next_char_boundary(&field.text, field.caret_position);
        }
        if input.is_key_pressed(KEY_HOME) {
            field.caret_position = 0;
        }
        if input.is_key_pressed(KEY_END) {
            field.caret_position = field.text.len();
        }
    }

    fn end_field_edit(&mut self, entity: EntityId, input: &InputState) {
        let Some(field) = self.input_fields.get_mut(&entity) else {
            return;
        };
        if input.is_key_pressed(KEY_RETURN) {
            field.was_canceled = false;
            field.editing_original_text = Some(field.text.clone());
            self.pending_end_edit.push(InputFieldEndEditEvent {
                entity,
                text: field.text.clone(),
                canceled: false,
            });
        } else if input.is_key_pressed(KEY_ESCAPE) {
            field.was_canceled = true;
            if let Some(original) = field.editing_original_text.clone()
                && field.text != original
            {
                field.text = original;
                field.caret_position = field.text.len();
                self.pending_value_changed
                    .push(InputFieldValueChangedEvent {
                        entity,
                        text: field.text.clone(),
                    });
            }
            self.pending_end_edit.push(InputFieldEndEditEvent {
                entity,
                text: field.text.clone(),
                canceled: true,
            });
        }
    }

    fn handle_input_field_editing(&mut self, entity: EntityId, input: &InputState) {
        let chars = input.flush_queued_chars();
        if !self.begin_field_edit(entity) {
            return;
        }
        self.insert_field_chars(entity, &chars);
        self.delete_field_chars(entity, input);
        self.move_field_caret(entity, input);
        self.end_field_edit(entity, input);
    }

    pub fn draw_items(&self, screen_size: [f32; 2]) -> Vec<UiDrawItem> {
        self.draw_items_with(screen_size, &HashMap::new())
    }

    pub fn draw_items_with(
        &self,
        screen_size: [f32; 2],
        texts: &HashMap<EntityId, TextOverride>,
    ) -> Vec<UiDrawItem> {
        let mut items = Vec::new();
        if screen_size[0] <= 0.0 || screen_size[1] <= 0.0 {
            return items;
        }
        let resolved_rects = self.resolve_all_element_rects(screen_size, texts);
        for element in &self.binding.elements {
            let (rect_px, scale) = match resolved_rects.get(&element.entity()).copied() {
                Some(resolved) => resolved,
                None => continue,
            };
            if !self.is_hierarchy_visible(element.entity()) {
                continue;
            }
            let rect_px = self.slider_part_rect(element.entity(), rect_px);
            let modifiers = DrawModifiers {
                clip: self.mask_rect_from(element.entity(), &resolved_rects),
                alpha: self.group_alpha(element.entity()),
            };
            self.push_element_draw_items(element, rect_px, scale, texts, modifiers, &mut items);
        }
        let focused_id = self.focused_button(screen_size);
        for &entity in &self.input_field_order {
            let Some(field) = self.input_fields.get(&entity) else {
                continue;
            };
            if !self.is_element_enabled(entity, field.enabled) || !self.is_hierarchy_visible(entity)
            {
                continue;
            }
            let Some((rect_px, scale)) = resolved_rects.get(&entity).copied() else {
                continue;
            };
            let modifiers = DrawModifiers {
                clip: self.mask_rect_from(entity, &resolved_rects),
                alpha: self.group_alpha(entity),
            };
            self.push_input_field_draw_items(
                field,
                rect_px,
                scale,
                focused_id == Some(entity),
                modifiers,
                &mut items,
            );
        }
        self.push_focus_border_items(focused_id, &resolved_rects, &mut items);
        items
    }

    fn slider_part_rect(&self, entity: EntityId, rect: [f32; 4]) -> [f32; 4] {
        for slider in self.sliders.values() {
            if !slider.enabled {
                continue;
            }
            let t = slider.normalized_value();
            if slider.fill_rect == Some(entity) {
                return match slider.direction {
                    SliderDirection::LeftToRight => [rect[0], rect[1], rect[2] * t, rect[3]],
                    SliderDirection::RightToLeft => {
                        [rect[0] + rect[2] * (1.0 - t), rect[1], rect[2] * t, rect[3]]
                    }
                    SliderDirection::BottomToTop => {
                        [rect[0], rect[1] + rect[3] * (1.0 - t), rect[2], rect[3] * t]
                    }
                    SliderDirection::TopToBottom => [rect[0], rect[1], rect[2], rect[3] * t],
                };
            }
            if slider.handle_rect == Some(entity) {
                return match slider.direction {
                    SliderDirection::LeftToRight => {
                        [rect[0] + rect[2] * t, rect[1], rect[2], rect[3]]
                    }
                    SliderDirection::RightToLeft => {
                        [rect[0] + rect[2] * (1.0 - t), rect[1], rect[2], rect[3]]
                    }
                    SliderDirection::BottomToTop => {
                        [rect[0], rect[1] + rect[3] * (1.0 - t), rect[2], rect[3]]
                    }
                    SliderDirection::TopToBottom => {
                        [rect[0], rect[1] + rect[3] * t, rect[2], rect[3]]
                    }
                };
            }
        }
        rect
    }

    fn toggle_graphic_hidden(&self, entity: EntityId) -> bool {
        self.toggles
            .values()
            .any(|toggle| toggle.graphic == Some(entity) && toggle.enabled && !toggle.is_on)
    }

    fn push_element_draw_items(
        &self,
        element: &UiElementComponent,
        rect_px: [f32; 4],
        scale: f32,
        texts: &HashMap<EntityId, TextOverride>,
        modifiers: DrawModifiers,
        items: &mut Vec<UiDrawItem>,
    ) {
        match element {
            UiElementComponent::Image(image) => {
                self.push_image_draw_items(
                    image,
                    rect_px,
                    scale,
                    modifiers.clip,
                    modifiers.alpha,
                    items,
                );
            }
            UiElementComponent::Text(text) => {
                self.push_text_draw_items(text, rect_px, scale, texts, modifiers, items);
            }
            UiElementComponent::Button(_) => {}
        }
    }

    fn push_image_draw_items(
        &self,
        image: &UiImageComponent,
        rect_px: [f32; 4],
        scale: f32,
        clip: Option<[f32; 4]>,
        alpha: f32,
        items: &mut Vec<UiDrawItem>,
    ) {
        if !self.is_element_enabled(image.entity, image.enabled)
            || self.toggle_graphic_hidden(image.entity)
        {
            return;
        }
        let color = [
            image.color[0],
            image.color[1],
            image.color[2],
            image.color[3] * alpha,
        ];
        let sprite_guid = image.sprite_guid.clone();
        let sprite_size = sprite_guid.as_deref().and_then(ui_sprite_size);
        let quads = match (image.draw_mode, sprite_size) {
            (SpriteDrawMode::Sliced, Some(size)) => {
                slice_sprite_quads(image.sprite_border, size, rect_px, FULL_UV, scale)
            }
            (SpriteDrawMode::Tiled, Some(size)) => tile_sprite_quads(size, rect_px, FULL_UV, scale),
            _ => vec![SpriteQuad {
                rect: rect_px,
                uv: FULL_UV,
            }],
        };
        for quad in quads {
            items.push(UiDrawItem {
                rect_px: quad.rect,
                color,
                sprite: sprite_guid.as_ref().map(|guid| SpriteRef {
                    texture_guid: guid.clone(),
                    uv_rect: quad.uv,
                }),
                text: None,
                text_layout: TextLayoutConfig::default(),
                text_clip_rect: None,
                clip_rect: clip,
            });
        }
    }

    fn push_text_draw_items(
        &self,
        text: &UiTextComponent,
        rect_px: [f32; 4],
        scale: f32,
        texts: &HashMap<EntityId, TextOverride>,
        modifiers: DrawModifiers,
        items: &mut Vec<UiDrawItem>,
    ) {
        if !self.is_element_enabled(text.entity, text.enabled) {
            return;
        }
        let state = texts.get(&text.entity);
        let content = state.map_or(text.text.as_str(), |state| state.text.as_str());
        let mut color = state.map_or(text.color, |state| state.color);
        color[3] *= modifiers.alpha;
        let font_size = state.map_or(text.font_size, |state| state.font_size);
        let text_layout = state.map_or_else(TextLayoutConfig::default, |s| s.layout);
        let truncate_clip =
            (text_layout.vertical_overflow == VerticalWrapMode::Truncate).then_some(rect_px);
        let text_clip_rect = intersect_clip(truncate_clip, modifiers.clip);
        items.push(UiDrawItem {
            rect_px,
            color,
            sprite: None,
            text: Some(UiTextItem {
                text: content.to_string(),
                x: rect_px[0],
                y: rect_px[1],
                size_px: font_size * scale,
                color,
            }),
            text_layout,
            text_clip_rect,
            clip_rect: modifiers.clip,
        });
    }

    fn push_focus_border_items(
        &self,
        focused_id: Option<EntityId>,
        resolved_rects: &HashMap<EntityId, ([f32; 4], f32)>,
        items: &mut Vec<UiDrawItem>,
    ) {
        let Some(focused_id) = focused_id else {
            return;
        };
        let Some((rect_px, scale)) = resolved_rects.get(&focused_id).copied() else {
            return;
        };
        let thickness = (FOCUS_BORDER_THICKNESS * scale).round().max(1.0);
        for edge_rect in focus_border_rects(rect_px, thickness) {
            items.push(UiDrawItem {
                rect_px: edge_rect,
                color: FOCUS_INDICATOR_COLOR,
                sprite: None,
                text: None,
                text_layout: TextLayoutConfig::default(),
                text_clip_rect: None,
                clip_rect: None,
            });
        }
    }

    fn push_input_field_draw_items(
        &self,
        field: &UiInputFieldComponent,
        rect_px: [f32; 4],
        scale: f32,
        is_focused: bool,
        modifiers: DrawModifiers,
        items: &mut Vec<UiDrawItem>,
    ) {
        let clip = modifiers.clip;
        let alpha = modifiers.alpha;
        let has_image =
            self.binding.elements.iter().any(
                |el| matches!(el, UiElementComponent::Image(img) if img.entity == field.entity),
            );
        if !has_image {
            items.push(UiDrawItem {
                rect_px,
                color: [
                    field.background_color[0],
                    field.background_color[1],
                    field.background_color[2],
                    field.background_color[3] * alpha,
                ],
                sprite: None,
                text: None,
                text_layout: TextLayoutConfig::default(),
                text_clip_rect: None,
                clip_rect: clip,
            });
        }
        let pad_x = (6.0 * scale).min(rect_px[2] * 0.25);
        let pad_y = (4.0 * scale).min(rect_px[3] * 0.2);
        let text_rect = [
            rect_px[0] + pad_x,
            rect_px[1] + pad_y,
            (rect_px[2] - 2.0 * pad_x).max(0.0),
            (rect_px[3] - 2.0 * pad_y).max(0.0),
        ];
        let font_px = (field.font_size * scale).max(1.0);
        let (display_text, mut text_color) = if !field.text.is_empty() {
            (field.text.as_str(), field.color)
        } else {
            (field.placeholder.as_str(), field.placeholder_color)
        };
        text_color[3] *= alpha;
        if !display_text.is_empty() {
            items.push(UiDrawItem {
                rect_px: text_rect,
                color: text_color,
                sprite: None,
                text: Some(UiTextItem {
                    text: display_text.to_string(),
                    x: text_rect[0],
                    y: text_rect[1],
                    size_px: font_px,
                    color: text_color,
                }),
                text_layout: TextLayoutConfig::default(),
                text_clip_rect: Some(rect_px),
                clip_rect: clip,
            });
        }
        if is_focused {
            push_input_field_caret_item(field, rect_px, text_rect, font_px, scale, clip, items);
        }
    }

    fn is_input_field_focusable(
        &self,
        field: &UiInputFieldComponent,
        screen_size: [f32; 2],
    ) -> bool {
        if !field.interactable
            || !self.is_element_enabled(field.entity, field.enabled)
            || !self.group_interactable(field.entity)
        {
            return false;
        }
        let Some((rect_px, _)) = self.element_rect(field.entity, screen_size) else {
            return false;
        };
        if rect_px[2] <= 0.0 || rect_px[3] <= 0.0 {
            return false;
        }
        self.is_hierarchy_visible(field.entity)
    }

    fn is_element_enabled(&self, entity: EntityId, default_enabled: bool) -> bool {
        self.element_enabled
            .get(&entity)
            .copied()
            .unwrap_or(default_enabled)
    }

    fn is_hierarchy_visible(&self, entity: EntityId) -> bool {
        let Some(chain) = self.chain_to_canvas(entity) else {
            return false;
        };
        for ancestor in chain {
            for element in &self.binding.elements {
                if let UiElementComponent::Image(image) = element
                    && image.entity == ancestor
                    && !self.is_element_enabled(image.entity, image.enabled)
                {
                    return false;
                }
            }
        }
        true
    }

    fn is_button_focusable(&self, button: &UiButtonComponent, screen_size: [f32; 2]) -> bool {
        let interactable = self
            .button_interactable
            .get(&button.entity)
            .copied()
            .unwrap_or(button.interactable);
        if !interactable || !self.group_interactable(button.entity) {
            return false;
        }
        let Some((rect_px, _)) = self.element_rect(button.entity, screen_size) else {
            return false;
        };
        if rect_px[2] <= 0.0 || rect_px[3] <= 0.0 {
            return false;
        }
        if !self.is_hierarchy_visible(button.entity) {
            return false;
        }
        if let Some(target) = button.target_graphic {
            for element in &self.binding.elements {
                match element {
                    UiElementComponent::Image(image) if image.entity == target => {
                        if !self.is_element_enabled(image.entity, image.enabled) {
                            return false;
                        }
                    }
                    UiElementComponent::Text(text) if text.entity == target => {
                        if !self.is_element_enabled(text.entity, text.enabled) {
                            return false;
                        }
                    }
                    _ => {}
                }
            }
        }
        true
    }

    fn navigate_direction(
        &self,
        focusables: &[EntityId],
        current: Option<EntityId>,
        direction: [f32; 2],
        screen_size: [f32; 2],
    ) -> EntityId {
        let Some(current_id) = current else {
            return focusables[0];
        };
        let Some((current_rect, _)) = self.element_rect(current_id, screen_size) else {
            return step_focus(
                focusables,
                Some(current_id),
                2 * i32::from(direction[0] > 0.0 || direction[1] > 0.0) - 1,
            );
        };
        let current_center = [
            current_rect[0] + current_rect[2] * 0.5,
            current_rect[1] + current_rect[3] * 0.5,
        ];
        let mut best: Option<(EntityId, f32)> = None;
        for &candidate in focusables {
            if candidate == current_id {
                continue;
            }
            let Some((rect, _)) = self.element_rect(candidate, screen_size) else {
                continue;
            };
            let center = [rect[0] + rect[2] * 0.5, rect[1] + rect[3] * 0.5];
            let dx = center[0] - current_center[0];
            let dy = center[1] - current_center[1];
            let dist_sq = dx * dx + dy * dy;
            if dist_sq <= f32::EPSILON {
                continue;
            }
            let dist = dist_sq.sqrt();
            let dot = (dx * direction[0] + dy * direction[1]) / dist;
            if dot <= 0.1 {
                continue;
            }
            let score = dot / dist;
            if best.is_none_or(|(_, best_score)| score > best_score) {
                best = Some((candidate, score));
            }
        }
        if let Some((winner, _)) = best {
            return winner;
        }
        let fallback_step = 2 * i32::from(direction[0] > 0.0 || direction[1] > 0.0) - 1;
        step_focus(focusables, Some(current_id), fallback_step)
    }

    pub fn element_rect(&self, entity: EntityId, screen_size: [f32; 2]) -> Option<([f32; 4], f32)> {
        self.element_rect_with(entity, screen_size, &HashMap::new())
    }

    pub fn element_rect_with(
        &self,
        entity: EntityId,
        screen_size: [f32; 2],
        texts: &HashMap<EntityId, TextOverride>,
    ) -> Option<([f32; 4], f32)> {
        if screen_size[0] <= 0.0 || screen_size[1] <= 0.0 {
            return None;
        }
        let resolved = self.resolve_all_element_rects(screen_size, texts);
        resolved.get(&entity).copied()
    }

    fn resolve_all_element_rects(
        &self,
        screen_size: [f32; 2],
        texts: &HashMap<EntityId, TextOverride>,
    ) -> HashMap<EntityId, ([f32; 4], f32)> {
        let mut results = HashMap::new();
        if screen_size[0] <= 0.0 || screen_size[1] <= 0.0 {
            return results;
        }
        let pref_sizes = self.compute_preferred_sizes(texts);
        for canvas in &self.binding.canvases {
            if canvas.render_mode != CanvasRenderMode::ScreenSpaceOverlay {
                continue;
            }
            let scale = self.active_scaler_scale(canvas, screen_size);
            if scale <= 0.0 {
                continue;
            }
            let canvas_size = [screen_size[0] / scale, screen_size[1] / scale];
            let root_rect = CanvasRect {
                min: [0.0, 0.0],
                size: canvas_size,
            };
            let ctx = SubtreeLayoutContext {
                canvas_size,
                scale,
                pref_sizes: &pref_sizes,
            };
            self.resolve_subtree_rects(canvas.entity, root_rect, &ctx, &mut results, 0);
        }
        results
    }

    fn resolve_subtree_rects(
        &self,
        entity: EntityId,
        current_rect: CanvasRect,
        ctx: &SubtreeLayoutContext<'_>,
        results: &mut HashMap<EntityId, ([f32; 4], f32)>,
        depth: usize,
    ) {
        if depth >= MAX_HIERARCHY_DEPTH {
            return;
        }
        results.insert(
            entity,
            (
                canvas_rect_to_pixels(current_rect, ctx.canvas_size, ctx.scale),
                ctx.scale,
            ),
        );
        if let Some(group) = self.active_layout_group(entity) {
            let children = self.active_children_of(entity);
            if children.is_empty() {
                return;
            }
            let child_rects = resolve_layout_group_children(
                &group,
                current_rect,
                &children,
                &self.rects,
                ctx.pref_sizes,
            );
            for (child, child_rect) in children.into_iter().zip(child_rects) {
                self.resolve_subtree_rects(child, child_rect, ctx, results, depth + 1);
            }
            return;
        }
        let children = self.children_of(entity);
        if children.is_empty() {
            return;
        }
        for child in children {
            let Some(transform) = self.rects.get(&child) else {
                continue;
            };
            let mut child_rect = resolve_canvas_rect(transform, current_rect);
            if let Some(fitter) = self.active_content_size_fitter(child) {
                let pref = ctx
                    .pref_sizes
                    .get(&child)
                    .copied()
                    .unwrap_or(transform.size_delta);
                child_rect = apply_content_size_fitter(&fitter, transform, child_rect, pref);
            }
            self.resolve_subtree_rects(child, child_rect, ctx, results, depth + 1);
        }
    }

    fn compute_preferred_sizes(
        &self,
        texts: &HashMap<EntityId, TextOverride>,
    ) -> HashMap<EntityId, [f32; 2]> {
        let mut sizes = HashMap::new();
        for canvas in &self.binding.canvases {
            self.compute_node_preferred_size(canvas.entity, texts, &mut sizes, 0);
        }
        for rect in &self.binding.rect_transforms {
            if !sizes.contains_key(&rect.entity) {
                self.compute_node_preferred_size(rect.entity, texts, &mut sizes, 0);
            }
        }
        sizes
    }

    fn compute_node_preferred_size(
        &self,
        entity: EntityId,
        texts: &HashMap<EntityId, TextOverride>,
        sizes: &mut HashMap<EntityId, [f32; 2]>,
        depth: usize,
    ) -> [f32; 2] {
        if let Some(&cached) = sizes.get(&entity) {
            return cached;
        }
        if depth >= MAX_HIERARCHY_DEPTH {
            return [0.0, 0.0];
        }
        let children = self.active_children_of(entity);
        let mut child_prefs = Vec::with_capacity(children.len());
        for &child in &children {
            child_prefs.push(self.compute_node_preferred_size(child, texts, sizes, depth + 1));
        }
        let size = if let Some(group) = self.active_layout_group(entity) {
            layout_group_preferred_size(&group, &child_prefs)
        } else if let Some(text_pref) = self.text_preferred_size(entity, texts) {
            text_pref
        } else {
            self.rects.get(&entity).map_or([0.0, 0.0], |r| {
                [r.size_delta[0].max(0.0), r.size_delta[1].max(0.0)]
            })
        };
        sizes.insert(entity, size);
        size
    }

    fn text_preferred_size(
        &self,
        entity: EntityId,
        texts: &HashMap<EntityId, TextOverride>,
    ) -> Option<[f32; 2]> {
        for element in &self.binding.elements {
            let UiElementComponent::Text(text) = element else {
                continue;
            };
            if text.entity != entity || !self.is_element_enabled(text.entity, text.enabled) {
                continue;
            }
            let state = texts.get(&text.entity);
            let content = state.map_or(text.text.as_str(), |s| s.text.as_str());
            let font_size = state.map_or(text.font_size, |s| s.font_size);
            let color = state.map_or(text.color, |s| s.color);
            return Some(measure_text_preferred(content, font_size, color));
        }
        None
    }

    fn children_of(&self, parent: EntityId) -> Vec<EntityId> {
        let mut children = Vec::new();
        for rect in &self.binding.rect_transforms {
            if rect.parent == Some(parent) {
                children.push(rect.entity);
            }
        }
        children
    }

    fn active_children_of(&self, parent: EntityId) -> Vec<EntityId> {
        let mut children = Vec::new();
        for rect in &self.binding.rect_transforms {
            if rect.parent == Some(parent) && self.is_node_active(rect.entity) {
                children.push(rect.entity);
            }
        }
        children
    }

    fn is_node_active(&self, entity: EntityId) -> bool {
        if let Some(&enabled) = self.element_enabled.get(&entity) {
            return enabled;
        }
        for element in &self.binding.elements {
            match element {
                UiElementComponent::Image(img) if img.entity == entity => return img.enabled,
                UiElementComponent::Text(txt) if txt.entity == entity => return txt.enabled,
                _ => {}
            }
        }
        true
    }

    fn active_layout_group(&self, entity: EntityId) -> Option<UiLayoutGroupComponent> {
        if let Ok(guard) = self.layout_group_overrides.read()
            && let Some(group) = guard.get(&entity)
        {
            return self
                .is_element_enabled(group.entity, group.enabled)
                .then(|| group.clone());
        }
        self.binding
            .layout_groups
            .iter()
            .find(|g| g.entity == entity && self.is_element_enabled(g.entity, g.enabled))
            .cloned()
    }

    fn active_content_size_fitter(&self, entity: EntityId) -> Option<UiContentSizeFitterComponent> {
        if let Ok(guard) = self.fitter_overrides.read()
            && let Some(fitter) = guard.get(&entity)
        {
            return self
                .is_element_enabled(fitter.entity, fitter.enabled)
                .then(|| fitter.clone());
        }
        self.binding
            .content_size_fitters
            .iter()
            .find(|f| f.entity == entity && self.is_element_enabled(f.entity, f.enabled))
            .cloned()
    }

    fn chain_to_canvas(&self, entity: EntityId) -> Option<Vec<EntityId>> {
        let mut chain = Vec::new();
        let mut current = Some(entity);
        while let Some(node) = current {
            if chain.len() >= MAX_HIERARCHY_DEPTH {
                return None;
            }
            chain.push(node);
            if self
                .binding
                .canvases
                .iter()
                .any(|canvas| canvas.entity == node)
            {
                chain.reverse();
                return Some(chain);
            }
            current = self.rects.get(&node).and_then(|rect| rect.parent);
        }
        None
    }
}

fn push_input_field_caret_item(
    field: &UiInputFieldComponent,
    rect_px: [f32; 4],
    text_rect: [f32; 4],
    font_px: f32,
    scale: f32,
    clip: Option<[f32; 4]>,
    items: &mut Vec<UiDrawItem>,
) {
    let caret_idx = UiInputFieldComponent::clamp_caret(&field.text, field.caret_position);
    let prefix = &field.text[..caret_idx];
    let prefix_width = measure_text_preferred(prefix, font_px, field.color)[0];
    let caret_w = (2.0 * scale).round().max(2.0);
    let caret_h = font_px.min(text_rect[3]).max(4.0);
    let max_x = (text_rect[0] + text_rect[2] - caret_w).max(text_rect[0]);
    let caret_x = (text_rect[0] + prefix_width).min(max_x);
    let caret_y = text_rect[1];
    items.push(UiDrawItem {
        rect_px: [caret_x, caret_y, caret_w, caret_h],
        color: field.color,
        sprite: None,
        text: None,
        text_layout: TextLayoutConfig::default(),
        text_clip_rect: Some(rect_px),
        clip_rect: clip,
    });
}

fn intersect_clip(a: Option<[f32; 4]>, b: Option<[f32; 4]>) -> Option<[f32; 4]> {
    match (a, b) {
        (Some(a), Some(b)) => {
            let x = a[0].max(b[0]);
            let y = a[1].max(b[1]);
            let right = (a[0] + a[2]).min(b[0] + b[2]);
            let bottom = (a[1] + a[3]).min(b[1] + b[3]);
            (right > x && bottom > y).then_some([x, y, right - x, bottom - y])
        }
        (Some(a), None) => Some(a),
        (None, Some(b)) => Some(b),
        (None, None) => None,
    }
}

fn step_focus(focusables: &[EntityId], current: Option<EntityId>, step: i32) -> EntityId {
    let index = current
        .and_then(|id| focusables.iter().position(|&candidate| candidate == id))
        .unwrap_or(0);
    let next = (index as i32 + step).rem_euclid(focusables.len() as i32) as usize;
    focusables[next]
}

fn prev_char_boundary(text: &str, pos: usize) -> usize {
    let mut idx = pos.min(text.len());
    if idx == 0 {
        return 0;
    }
    idx -= 1;
    while idx > 0 && !text.is_char_boundary(idx) {
        idx -= 1;
    }
    idx
}

fn next_char_boundary(text: &str, pos: usize) -> usize {
    let mut idx = pos.min(text.len());
    if idx >= text.len() {
        return text.len();
    }
    idx += 1;
    while idx < text.len() && !text.is_char_boundary(idx) {
        idx += 1;
    }
    idx
}

fn navigation_direction(
    input: &InputState,
    prev_axes: [f64; 2],
    axes: [f64; 2],
) -> Option<[f32; 2]> {
    if input.is_key_pressed(KEY_RIGHT_ARROW)
        || input.is_gamepad_button_pressed(GAMEPAD_BUTTON_DPAD_RIGHT)
        || (prev_axes[0] < STICK_NAVIGATION_THRESHOLD && axes[0] >= STICK_NAVIGATION_THRESHOLD)
    {
        return Some([1.0, 0.0]);
    }
    if input.is_key_pressed(KEY_LEFT_ARROW)
        || input.is_gamepad_button_pressed(GAMEPAD_BUTTON_DPAD_LEFT)
        || (prev_axes[0] > -STICK_NAVIGATION_THRESHOLD && axes[0] <= -STICK_NAVIGATION_THRESHOLD)
    {
        return Some([-1.0, 0.0]);
    }
    if input.is_key_pressed(KEY_DOWN_ARROW)
        || input.is_gamepad_button_pressed(GAMEPAD_BUTTON_DPAD_DOWN)
        || (prev_axes[1] > -STICK_NAVIGATION_THRESHOLD && axes[1] <= -STICK_NAVIGATION_THRESHOLD)
    {
        return Some([0.0, 1.0]);
    }
    if input.is_key_pressed(KEY_UP_ARROW)
        || input.is_gamepad_button_pressed(GAMEPAD_BUTTON_DPAD_UP)
        || (prev_axes[1] < STICK_NAVIGATION_THRESHOLD && axes[1] >= STICK_NAVIGATION_THRESHOLD)
    {
        return Some([0.0, -1.0]);
    }
    None
}

fn canvas_rect_to_pixels(rect: CanvasRect, canvas_size: [f32; 2], scale: f32) -> [f32; 4] {
    let x = rect.min[0] * scale;
    let width = rect.size[0] * scale;
    let top = (canvas_size[1] - rect.min[1] - rect.size[1]) * scale;
    let height = rect.size[1] * scale;
    [x, top, width, height]
}

struct SubtreeLayoutContext<'a> {
    canvas_size: [f32; 2],
    scale: f32,
    pref_sizes: &'a HashMap<EntityId, [f32; 2]>,
}

#[derive(Clone, Copy)]
struct DrawModifiers {
    clip: Option<[f32; 4]>,
    alpha: f32,
}

fn measure_text_preferred(content: &str, font_size: f32, color: [f32; 4]) -> [f32; 2] {
    static FONTS: OnceLock<Mutex<FontLibrary>> = OnceLock::new();
    if content.is_empty() || font_size <= 0.0 {
        return [0.0, 0.0];
    }
    let mutex = FONTS.get_or_init(|| {
        let mut lib = FontLibrary::new();
        lib.load_bundled_font();
        Mutex::new(lib)
    });
    let Ok(mut fonts) = mutex.lock() else {
        return [content.len() as f32 * font_size * 0.6, font_size];
    };
    let style = TextStyle::new(font_size, font_size, color);
    let layout = layout_text(&mut fonts, content, None, &style);
    [layout.width, layout.height.max(font_size)]
}

fn layout_group_preferred_size(
    group: &UiLayoutGroupComponent,
    child_prefs: &[[f32; 2]],
) -> [f32; 2] {
    let pad_h = (group.padding.left + group.padding.right) as f32;
    let pad_v = (group.padding.top + group.padding.bottom) as f32;
    if child_prefs.is_empty() {
        return [pad_h.max(0.0), pad_v.max(0.0)];
    }
    let total_spacing = group.spacing * (child_prefs.len().saturating_sub(1) as f32);
    match group.axis {
        LayoutGroupAxis::Horizontal => {
            let sum_w: f32 = child_prefs.iter().map(|p| p[0]).sum();
            let max_h: f32 = child_prefs.iter().map(|p| p[1]).fold(0.0, f32::max);
            [
                (pad_h + sum_w + total_spacing).max(0.0),
                (pad_v + max_h).max(0.0),
            ]
        }
        LayoutGroupAxis::Vertical => {
            let max_w: f32 = child_prefs.iter().map(|p| p[0]).fold(0.0, f32::max);
            let sum_h: f32 = child_prefs.iter().map(|p| p[1]).sum();
            [
                (pad_h + max_w).max(0.0),
                (pad_v + sum_h + total_spacing).max(0.0),
            ]
        }
    }
}

fn apply_content_size_fitter(
    fitter: &UiContentSizeFitterComponent,
    transform: &RectTransformComponent,
    mut rect: CanvasRect,
    pref: [f32; 2],
) -> CanvasRect {
    if matches!(
        fitter.horizontal_fit,
        ContentSizeFitMode::MinSize | ContentSizeFitMode::PreferredSize
    ) {
        let old_w = rect.size[0];
        let new_w = pref[0].max(0.0);
        rect.min[0] += (old_w - new_w) * transform.pivot[0];
        rect.size[0] = new_w;
    }
    if matches!(
        fitter.vertical_fit,
        ContentSizeFitMode::MinSize | ContentSizeFitMode::PreferredSize
    ) {
        let old_h = rect.size[1];
        let new_h = pref[1].max(0.0);
        rect.min[1] += (old_h - new_h) * transform.pivot[1];
        rect.size[1] = new_h;
    }
    rect
}

fn alignment_fractions(alignment: u32) -> (f32, f32) {
    let anchor = TextAnchor::from_u32(alignment).unwrap_or(TextAnchor::UpperLeft);
    let h = match anchor.horizontal() {
        HorizontalAlign::Left => 0.0,
        HorizontalAlign::Center => 0.5,
        HorizontalAlign::Right => 1.0,
    };
    let v = match anchor.vertical() {
        VerticalAlign::Top => 0.0,
        VerticalAlign::Middle => 0.5,
        VerticalAlign::Bottom => 1.0,
    };
    (h, v)
}

fn resolve_layout_group_children(
    group: &UiLayoutGroupComponent,
    parent_rect: CanvasRect,
    children: &[EntityId],
    rects: &HashMap<EntityId, RectTransformComponent>,
    pref_sizes: &HashMap<EntityId, [f32; 2]>,
) -> Vec<CanvasRect> {
    let count = children.len();
    if count == 0 {
        return Vec::new();
    }
    let pad_l = group.padding.left as f32;
    let pad_r = group.padding.right as f32;
    let pad_t = group.padding.top as f32;
    let pad_b = group.padding.bottom as f32;
    let inner_w = (parent_rect.size[0] - pad_l - pad_r).max(0.0);
    let inner_h = (parent_rect.size[1] - pad_t - pad_b).max(0.0);
    let base_sizes: Vec<[f32; 2]> = children
        .iter()
        .map(|id| {
            let raw = rects.get(id).map_or([0.0, 0.0], |r| r.size_delta);
            let pref = pref_sizes.get(id).copied().unwrap_or(raw);
            [
                if group.child_control_width {
                    pref[0]
                } else {
                    raw[0]
                }
                .max(0.0),
                if group.child_control_height {
                    pref[1]
                } else {
                    raw[1]
                }
                .max(0.0),
            ]
        })
        .collect();
    let (align_x, align_y) = alignment_fractions(group.child_alignment);
    match group.axis {
        LayoutGroupAxis::Horizontal => layout_horizontal_children(
            group,
            parent_rect,
            inner_w,
            inner_h,
            &base_sizes,
            (align_x, align_y),
        ),
        LayoutGroupAxis::Vertical => layout_vertical_children(
            group,
            parent_rect,
            inner_w,
            inner_h,
            &base_sizes,
            (align_x, align_y),
        ),
    }
}

fn layout_horizontal_children(
    group: &UiLayoutGroupComponent,
    parent_rect: CanvasRect,
    inner_w: f32,
    inner_h: f32,
    base_sizes: &[[f32; 2]],
    align: (f32, f32),
) -> Vec<CanvasRect> {
    let count = base_sizes.len();
    let total_spacing = group.spacing * (count.saturating_sub(1) as f32);
    let sum_w: f32 = base_sizes.iter().map(|s| s[0]).sum();
    let extra_w = (inner_w - (sum_w + total_spacing)).max(0.0);
    let expand_add = if group.child_force_expand_width {
        extra_w / (count as f32)
    } else {
        0.0
    };
    let start_offset_x = if group.child_force_expand_width {
        0.0
    } else {
        extra_w * align.0
    };
    let mut cursor_x = parent_rect.min[0] + (group.padding.left as f32) + start_offset_x;
    let top_y = parent_rect.min[1] + parent_rect.size[1] - (group.padding.top as f32);
    let mut out = Vec::with_capacity(count);
    for base in base_sizes {
        let w = if group.child_control_width {
            base[0] + expand_add
        } else {
            base[0]
        };
        let h = if group.child_control_height && group.child_force_expand_height {
            inner_h
        } else {
            base[1]
        };
        let slack_y = (inner_h - h).max(0.0);
        let child_top_y = top_y - slack_y * align.1;
        out.push(CanvasRect {
            min: [cursor_x, child_top_y - h],
            size: [w, h],
        });
        cursor_x += w + group.spacing;
    }
    out
}

fn layout_vertical_children(
    group: &UiLayoutGroupComponent,
    parent_rect: CanvasRect,
    inner_w: f32,
    inner_h: f32,
    base_sizes: &[[f32; 2]],
    align: (f32, f32),
) -> Vec<CanvasRect> {
    let count = base_sizes.len();
    let total_spacing = group.spacing * (count.saturating_sub(1) as f32);
    let sum_h: f32 = base_sizes.iter().map(|s| s[1]).sum();
    let extra_h = (inner_h - (sum_h + total_spacing)).max(0.0);
    let expand_add = if group.child_force_expand_height {
        extra_h / (count as f32)
    } else {
        0.0
    };
    let start_offset_y = if group.child_force_expand_height {
        0.0
    } else {
        extra_h * align.1
    };
    let left_x = parent_rect.min[0] + (group.padding.left as f32);
    let mut cursor_top_y =
        parent_rect.min[1] + parent_rect.size[1] - (group.padding.top as f32) - start_offset_y;
    let mut out = Vec::with_capacity(count);
    for base in base_sizes {
        let h = if group.child_control_height {
            base[1] + expand_add
        } else {
            base[1]
        };
        let w = if group.child_control_width && group.child_force_expand_width {
            inner_w
        } else {
            base[0]
        };
        let slack_x = (inner_w - w).max(0.0);
        let child_min_x = left_x + slack_x * align.0;
        out.push(CanvasRect {
            min: [child_min_x, cursor_top_y - h],
            size: [w, h],
        });
        cursor_top_y -= h + group.spacing;
    }
    out
}
