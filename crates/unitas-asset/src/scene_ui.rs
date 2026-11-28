use std::collections::{HashMap, HashSet};

use unitas_core::EntityId;

use crate::meta::GuidIndex;
use crate::unity_yaml::{Node, Stream};

use super::scene_sprite::{sprite_border, ui_image_draw_mode};
use super::{
    color4_or, component_entity, reference_id, sprite_guid, vector2_or, yaml_bool, yaml_f32,
    yaml_i32, yaml_u32, AssetGuid, GameObjectRecords, SpriteDrawMode, MONO_BEHAVIOUR_CLASS,
};

pub const CANVAS_CLASS: u32 = 223;
pub const RECT_TRANSFORM_CLASS: u32 = 224;
use super::{
    BUTTON_SCRIPT_GUID, CANVAS_SCALER_SCRIPT_GUID, CONTENT_SIZE_FITTER_SCRIPT_GUID,
    HORIZONTAL_LAYOUT_GROUP_SCRIPT_GUID, IMAGE_SCRIPT_GUID, TEXT_SCRIPT_GUID,
    VERTICAL_LAYOUT_GROUP_SCRIPT_GUID,
};

const DEFAULT_UI_COLOR: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
const DEFAULT_UI_FONT_SIZE: f32 = 14.0;
const DEFAULT_UI_ENABLED: bool = true;
const DEFAULT_REFERENCE_RESOLUTION: [f32; 2] = [800.0, 600.0];
const DEFAULT_MATCH_WIDTH_OR_HEIGHT: f32 = 0.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CanvasRenderMode {
    ScreenSpaceOverlay,
    ScreenSpaceCamera,
    WorldSpace,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CanvasScaleMode {
    ConstantPixelSize,
    ScaleWithScreenSize,
    ConstantPhysicalSize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CanvasMatchMode {
    MatchWidthOrHeight,
    Shrink,
    Expand,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CanvasScalerComponent {
    pub ui_scale_mode: CanvasScaleMode,
    pub reference_resolution: [f32; 2],
    pub screen_match_mode: CanvasMatchMode,
    pub match_width_or_height: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CanvasComponent {
    pub entity: EntityId,
    pub render_mode: CanvasRenderMode,
    pub sorting_order: i64,
    pub scaler: Option<CanvasScalerComponent>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RectTransformComponent {
    pub entity: EntityId,
    pub parent: Option<EntityId>,
    pub anchor_min: [f32; 2],
    pub anchor_max: [f32; 2],
    pub anchored_position: [f32; 2],
    pub size_delta: [f32; 2],
    pub pivot: [f32; 2],
}

#[derive(Debug, Clone, PartialEq)]
pub struct UiImageComponent {
    pub entity: EntityId,
    pub color: [f32; 4],
    pub sprite_guid: Option<AssetGuid>,
    pub enabled: bool,
    pub draw_mode: SpriteDrawMode,
    pub sprite_border: [f32; 4],
}

#[derive(Debug, Clone, PartialEq)]
pub struct UiTextComponent {
    pub entity: EntityId,
    pub text: String,
    pub color: [f32; 4],
    pub font_size: f32,
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct UiButtonComponent {
    pub entity: EntityId,
    pub target_graphic: Option<EntityId>,
    pub interactable: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct UiPadding {
    pub left: i32,
    pub right: i32,
    pub top: i32,
    pub bottom: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LayoutGroupAxis {
    #[default]
    Horizontal,
    Vertical,
}

#[derive(Debug, Clone, PartialEq)]
pub struct UiLayoutGroupComponent {
    pub entity: EntityId,
    pub axis: LayoutGroupAxis,
    pub padding: UiPadding,
    pub spacing: f32,
    pub child_alignment: u32,
    pub child_control_width: bool,
    pub child_control_height: bool,
    pub child_force_expand_width: bool,
    pub child_force_expand_height: bool,
    pub enabled: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ContentSizeFitMode {
    #[default]
    Unconstrained = 0,
    MinSize = 1,
    PreferredSize = 2,
}

impl ContentSizeFitMode {
    pub const fn from_u32(code: u32) -> Option<Self> {
        match code {
            0 => Some(Self::Unconstrained),
            1 => Some(Self::MinSize),
            2 => Some(Self::PreferredSize),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiContentSizeFitterComponent {
    pub entity: EntityId,
    pub horizontal_fit: ContentSizeFitMode,
    pub vertical_fit: ContentSizeFitMode,
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum UiElementComponent {
    Image(UiImageComponent),
    Text(UiTextComponent),
    Button(UiButtonComponent),
}

impl UiElementComponent {
    pub fn entity(&self) -> EntityId {
        match self {
            Self::Image(image) => image.entity,
            Self::Text(text) => text.entity,
            Self::Button(button) => button.entity,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct UiBinding {
    pub canvases: Vec<CanvasComponent>,
    pub rect_transforms: Vec<RectTransformComponent>,
    pub elements: Vec<UiElementComponent>,
    pub layout_groups: Vec<UiLayoutGroupComponent>,
    pub content_size_fitters: Vec<UiContentSizeFitterComponent>,
}

impl UiBinding {
    pub fn shift_entity_ids(&mut self, base: u64) {
        for canvas in &mut self.canvases {
            canvas.entity = EntityId::new(base + canvas.entity.as_u64());
        }
        for rect in &mut self.rect_transforms {
            rect.entity = EntityId::new(base + rect.entity.as_u64());
            rect.parent = rect
                .parent
                .map(|parent| EntityId::new(base + parent.as_u64()));
        }
        for element in &mut self.elements {
            match element {
                UiElementComponent::Image(image) => {
                    image.entity = EntityId::new(base + image.entity.as_u64());
                }
                UiElementComponent::Text(text) => {
                    text.entity = EntityId::new(base + text.entity.as_u64());
                }
                UiElementComponent::Button(button) => {
                    button.entity = EntityId::new(base + button.entity.as_u64());
                    button.target_graphic = button
                        .target_graphic
                        .map(|target| EntityId::new(base + target.as_u64()));
                }
            }
        }
        for group in &mut self.layout_groups {
            group.entity = EntityId::new(base + group.entity.as_u64());
        }
        for fitter in &mut self.content_size_fitters {
            fitter.entity = EntityId::new(base + fitter.entity.as_u64());
        }
    }
}

pub(super) fn collect_ui(
    stream: &Stream,
    game_objects: &GameObjectRecords,
    instance_roots: &HashSet<i64>,
    index: Option<&GuidIndex>,
) -> UiBinding {
    let mut binding = UiBinding::default();
    let mut collectors = UiCollectors::default();
    let mut rect_entities: HashMap<i64, EntityId> = HashMap::new();
    let mut rect_fathers: HashMap<i64, i64> = HashMap::new();
    let mut rects: Vec<(i64, RectTransformComponent)> = Vec::new();
    for doc in &stream.documents {
        if doc.stripped {
            continue;
        }
        if doc.class_id == RECT_TRANSFORM_CLASS {
            let Some(body) = doc.root.get("RectTransform") else {
                continue;
            };
            let Some(entity) = component_entity(body, game_objects, instance_roots) else {
                continue;
            };
            rect_entities.insert(doc.file_id, entity);
            if let Ok(Some(father)) = reference_id(body.get("m_Father")) {
                rect_fathers.insert(doc.file_id, father);
            }
            rects.push((doc.file_id, parse_rect_transform(entity, body)));
        } else if doc.class_id == CANVAS_CLASS {
            let Some(body) = doc.root.get("Canvas") else {
                continue;
            };
            let Some(entity) = component_entity(body, game_objects, instance_roots) else {
                continue;
            };
            binding.canvases.push(CanvasComponent {
                entity,
                render_mode: canvas_render_mode(body.get("m_RenderMode")),
                sorting_order: body
                    .get("m_SortingOrder")
                    .and_then(Node::as_i64)
                    .unwrap_or(0),
                scaler: None,
            });
        }
    }
    for (file_id, rect) in &mut rects {
        rect.parent = rect_fathers
            .get(file_id)
            .and_then(|f| rect_entities.get(f))
            .copied();
    }
    binding.rect_transforms = rects.into_iter().map(|(_, rect)| rect).collect();
    for doc in &stream.documents {
        if doc.stripped || doc.class_id != MONO_BEHAVIOUR_CLASS {
            continue;
        }
        let Some(body) = doc.root.get("MonoBehaviour") else {
            continue;
        };
        let Some(guid) = body
            .get("m_Script")
            .and_then(|n| n.get("guid"))
            .and_then(Node::as_str)
            .map(str::trim)
        else {
            continue;
        };
        let Some(entity) = component_entity(body, game_objects, instance_roots) else {
            continue;
        };
        collect_ui_mono_behaviour(
            doc.file_id,
            entity,
            guid,
            body,
            index,
            &mut binding,
            &mut collectors,
        );
    }
    for (slot, target) in collectors.button_targets {
        if let Some(entity) = collectors.graphics.get(&target).copied() {
            if let Some(UiElementComponent::Button(button)) = binding.elements.get_mut(slot) {
                button.target_graphic = Some(entity);
            }
        }
    }
    binding
}

#[derive(Default)]
struct UiCollectors {
    graphics: HashMap<i64, EntityId>,
    button_targets: Vec<(usize, i64)>,
}

fn parse_rect_transform(entity: EntityId, body: &Node) -> RectTransformComponent {
    RectTransformComponent {
        entity,
        parent: None,
        anchor_min: vector2_or(body.get("m_AnchorMin"), [0.0, 0.0]),
        anchor_max: vector2_or(body.get("m_AnchorMax"), [1.0, 1.0]),
        anchored_position: vector2_or(body.get("m_AnchoredPosition"), [0.0, 0.0]),
        size_delta: vector2_or(body.get("m_SizeDelta"), [0.0, 0.0]),
        pivot: vector2_or(body.get("m_Pivot"), [0.5, 0.5]),
    }
}

fn collect_ui_mono_behaviour(
    file_id: i64,
    entity: EntityId,
    guid: &str,
    body: &Node,
    index: Option<&GuidIndex>,
    binding: &mut UiBinding,
    collectors: &mut UiCollectors,
) {
    match guid {
        IMAGE_SCRIPT_GUID => {
            collectors.graphics.insert(file_id, entity);
            let sprite = body.get("m_Sprite");
            let sprite_guid = sprite_guid(sprite);
            binding
                .elements
                .push(UiElementComponent::Image(UiImageComponent {
                    entity,
                    color: color4_or(body.get("m_Color"), DEFAULT_UI_COLOR),
                    sprite_border: sprite_border(index, sprite_guid.as_deref()),
                    sprite_guid,
                    enabled: yaml_bool(body.get("m_Enabled"), DEFAULT_UI_ENABLED),
                    draw_mode: ui_image_draw_mode(body.get("m_Type")),
                }));
        }
        TEXT_SCRIPT_GUID => binding
            .elements
            .push(UiElementComponent::Text(UiTextComponent {
                entity,
                text: body
                    .get("m_Text")
                    .and_then(Node::as_str)
                    .unwrap_or_default()
                    .to_string(),
                color: color4_or(body.get("m_Color"), DEFAULT_UI_COLOR),
                font_size: yaml_f32(body.get("m_FontSize"), DEFAULT_UI_FONT_SIZE, |v| v > 0.0),
                enabled: yaml_bool(body.get("m_Enabled"), DEFAULT_UI_ENABLED),
            })),
        BUTTON_SCRIPT_GUID => {
            let index = binding.elements.len();
            binding
                .elements
                .push(UiElementComponent::Button(UiButtonComponent {
                    entity,
                    target_graphic: None,
                    interactable: yaml_bool(body.get("m_Interactable"), DEFAULT_UI_ENABLED),
                }));
            if let Ok(Some(target)) = reference_id(body.get("m_TargetGraphic")) {
                collectors.button_targets.push((index, target));
            }
        }
        CANVAS_SCALER_SCRIPT_GUID => {
            if let Some(canvas) = binding.canvases.iter_mut().find(|c| c.entity == entity) {
                canvas.scaler = Some(CanvasScalerComponent {
                    ui_scale_mode: canvas_scale_mode(body.get("m_UiScaleMode")),
                    reference_resolution: vector2_or(
                        body.get("m_ReferenceResolution"),
                        DEFAULT_REFERENCE_RESOLUTION,
                    ),
                    screen_match_mode: canvas_match_mode(body.get("m_ScreenMatchMode")),
                    match_width_or_height: yaml_f32(
                        body.get("m_MatchWidthOrHeight"),
                        DEFAULT_MATCH_WIDTH_OR_HEIGHT,
                        |v| (0.0..=1.0).contains(&v),
                    ),
                });
            }
        }
        HORIZONTAL_LAYOUT_GROUP_SCRIPT_GUID => binding.layout_groups.push(parse_layout_group(
            entity,
            LayoutGroupAxis::Horizontal,
            body,
        )),
        VERTICAL_LAYOUT_GROUP_SCRIPT_GUID => {
            binding
                .layout_groups
                .push(parse_layout_group(entity, LayoutGroupAxis::Vertical, body));
        }
        CONTENT_SIZE_FITTER_SCRIPT_GUID => binding
            .content_size_fitters
            .push(parse_content_size_fitter(entity, body)),
        _ => {}
    }
}

fn parse_layout_group(
    entity: EntityId,
    axis: LayoutGroupAxis,
    body: &Node,
) -> UiLayoutGroupComponent {
    let padding = body
        .get("m_Padding")
        .map_or_else(UiPadding::default, |n| UiPadding {
            left: yaml_i32(n.get("m_Left"), 0),
            right: yaml_i32(n.get("m_Right"), 0),
            top: yaml_i32(n.get("m_Top"), 0),
            bottom: yaml_i32(n.get("m_Bottom"), 0),
        });
    UiLayoutGroupComponent {
        entity,
        axis,
        padding,
        spacing: yaml_f32(body.get("m_Spacing"), 0.0, f32::is_finite),
        child_alignment: yaml_u32(body.get("m_ChildAlignment"), 0).min(8),
        child_control_width: yaml_bool(body.get("m_ChildControlWidth"), true),
        child_control_height: yaml_bool(body.get("m_ChildControlHeight"), true),
        child_force_expand_width: yaml_bool(body.get("m_ChildForceExpandWidth"), true),
        child_force_expand_height: yaml_bool(body.get("m_ChildForceExpandHeight"), true),
        enabled: yaml_bool(body.get("m_Enabled"), DEFAULT_UI_ENABLED),
    }
}

fn parse_content_size_fitter(entity: EntityId, body: &Node) -> UiContentSizeFitterComponent {
    UiContentSizeFitterComponent {
        entity,
        horizontal_fit: ContentSizeFitMode::from_u32(yaml_u32(body.get("m_HorizontalFit"), 0))
            .unwrap_or(ContentSizeFitMode::Unconstrained),
        vertical_fit: ContentSizeFitMode::from_u32(yaml_u32(body.get("m_VerticalFit"), 0))
            .unwrap_or(ContentSizeFitMode::Unconstrained),
        enabled: yaml_bool(body.get("m_Enabled"), DEFAULT_UI_ENABLED),
    }
}

fn canvas_render_mode(node: Option<&Node>) -> CanvasRenderMode {
    match node.and_then(Node::as_i64).unwrap_or(0) {
        1 => CanvasRenderMode::ScreenSpaceCamera,
        2 => CanvasRenderMode::WorldSpace,
        _ => CanvasRenderMode::ScreenSpaceOverlay,
    }
}

fn canvas_scale_mode(node: Option<&Node>) -> CanvasScaleMode {
    match node.and_then(Node::as_i64).unwrap_or(0) {
        1 => CanvasScaleMode::ScaleWithScreenSize,
        2 => CanvasScaleMode::ConstantPhysicalSize,
        _ => CanvasScaleMode::ConstantPixelSize,
    }
}

fn canvas_match_mode(node: Option<&Node>) -> CanvasMatchMode {
    match node.and_then(Node::as_i64).unwrap_or(0) {
        1 => CanvasMatchMode::Shrink,
        2 => CanvasMatchMode::Expand,
        _ => CanvasMatchMode::MatchWidthOrHeight,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::assemble_scene_bundle;
    use crate::unity_yaml;
    use std::error::Error;

    const UI_SCENE: &str = "%YAML 1.1
%TAG !u! tag:unity3d.com,2011:
--- !u!1 &1
GameObject: {m_Name: HudCanvas, m_Layer: 5}
--- !u!224 &2
RectTransform: {m_GameObject: {fileID: 1}, m_Father: {fileID: 0}, m_AnchorMin: {x: 0, y: 0}, m_AnchorMax: {x: 1, y: 1}, m_AnchoredPosition: {x: 0, y: 0}, m_SizeDelta: {x: 0, y: 0}, m_Pivot: {x: 0.5, y: 0.5}}
--- !u!223 &3
Canvas: {m_GameObject: {fileID: 1}, m_Enabled: 1, m_RenderMode: 0, m_SortingOrder: 7}
--- !u!114 &4
MonoBehaviour: {m_GameObject: {fileID: 1}, m_Enabled: 1, m_Script: {fileID: 11500000, guid: 0cd44c1031e13a943bb63640046fad76, type: 3}, m_UiScaleMode: 1, m_ReferenceResolution: {x: 800, y: 600}, m_ScreenMatchMode: 0, m_MatchWidthOrHeight: 0.5}
--- !u!1 &5
GameObject: {m_Name: HealthBar, m_Layer: 5}
--- !u!224 &6
RectTransform: {m_GameObject: {fileID: 5}, m_Father: {fileID: 2}, m_AnchorMin: {x: 0.5, y: 0.5}, m_AnchorMax: {x: 0.5, y: 0.5}, m_AnchoredPosition: {x: -120, y: 30}, m_SizeDelta: {x: 200, y: 40}, m_Pivot: {x: 0.5, y: 0.5}}
--- !u!114 &7
MonoBehaviour: {m_GameObject: {fileID: 5}, m_Enabled: 1, m_Script: {fileID: 11500000, guid: fe87c0e1cc204ed48ad3b37840f39efc, type: 3}, m_Color: {r: 1, g: 0.5, b: 0.25, a: 0.75}, m_Sprite: {fileID: 21300000, guid: 22222222222222222222222222222222, type: 3}, m_UnknownFutureField: 12}
--- !u!1 &8
GameObject: {m_Name: ScoreLabel, m_Layer: 5}
--- !u!224 &9
RectTransform: {m_GameObject: {fileID: 8}, m_Father: {fileID: 2}, m_AnchorMin: {x: 0, y: 1}, m_AnchorMax: {x: 0, y: 1}, m_AnchoredPosition: {x: 16, y: -12}, m_SizeDelta: {x: 320, y: 28}, m_Pivot: {x: 0, y: 1}}
--- !u!114 &10
MonoBehaviour: {m_GameObject: {fileID: 8}, m_Enabled: 1, m_Script: {fileID: 11500000, guid: 5f7201a12d95ffc409449d95f23cf332, type: 3}, m_Text: \"Score 0\", m_FontSize: 18, m_Color: {r: 1, g: 1, b: 1, a: 1}}
";

    const BUTTON_SCENE: &str = "%YAML 1.1
%TAG !u! tag:unity3d.com,2011:
--- !u!1 &1
GameObject: {m_Name: PlayButton, m_Layer: 5}
--- !u!224 &2
RectTransform: {m_GameObject: {fileID: 1}, m_Father: {fileID: 0}, m_AnchorMin: {x: 0.5, y: 0.5}, m_AnchorMax: {x: 0.5, y: 0.5}, m_AnchoredPosition: {x: 0, y: 0}, m_SizeDelta: {x: 160, y: 40}, m_Pivot: {x: 0.5, y: 0.5}}
--- !u!1 &5
GameObject: {m_Name: ButtonFace, m_Layer: 5}
--- !u!224 &6
RectTransform: {m_GameObject: {fileID: 5}, m_Father: {fileID: 2}, m_AnchorMin: {x: 0, y: 0}, m_AnchorMax: {x: 1, y: 1}, m_AnchoredPosition: {x: 0, y: 0}, m_SizeDelta: {x: 0, y: 0}, m_Pivot: {x: 0.5, y: 0.5}}
--- !u!114 &7
MonoBehaviour: {m_GameObject: {fileID: 5}, m_Enabled: 1, m_Script: {fileID: 11500000, guid: fe87c0e1cc204ed48ad3b37840f39efc, type: 3}, m_Color: {r: 1, g: 1, b: 1, a: 1}}
--- !u!114 &8
MonoBehaviour: {m_GameObject: {fileID: 1}, m_Enabled: 1, m_Script: {fileID: 11500000, guid: 4e29b1a8efbd4b44bb3f3716e73f07ff, type: 3}, m_TargetGraphic: {fileID: 7}, m_Interactable: 1}
";

    #[test]
    fn ui_scene_exposes_a_button_with_its_target_graphic() -> Result<(), Box<dyn Error>> {
        let stream = unity_yaml::parse(BUTTON_SCENE)?;
        let assembled = assemble_scene_bundle(&stream)?;
        let button = assembled
            .ui
            .elements
            .iter()
            .find_map(|element| match element {
                UiElementComponent::Button(button) => Some(button),
                _ => None,
            })
            .ok_or("the scene has no button")?;
        assert_eq!(button.entity, EntityId::new(0));
        assert_eq!(button.target_graphic, Some(EntityId::new(1)));
        assert!(button.interactable);
        Ok(())
    }

    #[test]
    fn shifting_the_ui_binding_moves_every_object_id() -> Result<(), Box<dyn Error>> {
        let mut binding = UiBinding {
            canvases: vec![CanvasComponent {
                entity: EntityId::new(0),
                render_mode: CanvasRenderMode::ScreenSpaceOverlay,
                sorting_order: 0,
                scaler: None,
            }],
            rect_transforms: vec![RectTransformComponent {
                entity: EntityId::new(1),
                parent: Some(EntityId::new(0)),
                anchor_min: [0.0, 0.0],
                anchor_max: [1.0, 1.0],
                anchored_position: [0.0, 0.0],
                size_delta: [0.0, 0.0],
                pivot: [0.5, 0.5],
            }],
            elements: vec![UiElementComponent::Button(UiButtonComponent {
                entity: EntityId::new(1),
                target_graphic: Some(EntityId::new(2)),
                interactable: true,
            })],
            layout_groups: Vec::new(),
            content_size_fitters: Vec::new(),
        };
        binding.shift_entity_ids(100);
        assert_eq!(binding.canvases[0].entity, EntityId::new(100));
        assert_eq!(binding.rect_transforms[0].entity, EntityId::new(101));
        assert_eq!(binding.rect_transforms[0].parent, Some(EntityId::new(100)));
        let UiElementComponent::Button(button) = &binding.elements[0] else {
            return Err(String::from("the element is not a button").into());
        };
        assert_eq!(button.entity, EntityId::new(101));
        assert_eq!(button.target_graphic, Some(EntityId::new(102)));
        Ok(())
    }

    #[test]
    fn ui_scene_exposes_canvas_rect_transform_image_and_text() -> Result<(), Box<dyn Error>> {
        let assembled = assemble_scene_bundle(&unity_yaml::parse(UI_SCENE)?)?;
        let canvas = &assembled.ui.canvases[0];
        assert_eq!((canvas.entity, canvas.sorting_order), (EntityId::new(0), 7));
        assert_eq!(canvas.render_mode, CanvasRenderMode::ScreenSpaceOverlay);
        let scaler = canvas.scaler.as_ref().ok_or("the canvas has no scaler")?;
        assert_eq!(scaler.ui_scale_mode, CanvasScaleMode::ScaleWithScreenSize);
        assert_eq!(scaler.reference_resolution, [800.0, 600.0]);
        assert_eq!(scaler.match_width_or_height, 0.5);
        assert_eq!(assembled.ui.rect_transforms.len(), 3);
        let bar = &assembled.ui.rect_transforms[1];
        assert_eq!(
            (bar.entity, bar.parent),
            (EntityId::new(1), Some(EntityId::new(0)))
        );
        assert_eq!((bar.anchor_min, bar.anchor_max), ([0.5, 0.5], [0.5, 0.5]));
        assert_eq!(bar.anchored_position, [-120.0, 30.0]);
        assert_eq!((bar.size_delta, bar.pivot), ([200.0, 40.0], [0.5, 0.5]));
        let UiElementComponent::Image(img) = &assembled.ui.elements[0] else {
            return Err("the first element is not an image".into());
        };
        assert_eq!((img.entity, img.enabled), (EntityId::new(1), true));
        assert_eq!(img.color, [1.0, 0.5, 0.25, 0.75]);
        assert_eq!(
            img.sprite_guid.as_deref(),
            Some("22222222222222222222222222222222")
        );
        let UiElementComponent::Text(txt) = &assembled.ui.elements[1] else {
            return Err("the second element is not a text".into());
        };
        assert_eq!(
            (txt.entity, txt.text.as_str(), txt.font_size),
            (EntityId::new(2), "Score 0", 18.0)
        );
        assert_eq!((txt.color, txt.enabled), ([1.0, 1.0, 1.0, 1.0], true));
        Ok(())
    }

    const LAYOUT_GROUP_SCENE: &str = "%YAML 1.1
%TAG !u! tag:unity3d.com,2011:
--- !u!1 &1
GameObject: {m_Name: MenuPanel, m_Layer: 5}
--- !u!224 &2
RectTransform: {m_GameObject: {fileID: 1}, m_Father: {fileID: 0}, m_SizeDelta: {x: 300, y: 200}}
--- !u!114 &3
MonoBehaviour: {m_GameObject: {fileID: 1}, m_Enabled: 1, m_Script: {fileID: 11500000, guid: 59f8146938fff824cb5fd77236b75775, type: 3}, m_Padding: {m_Left: 10, m_Right: 12, m_Top: 8, m_Bottom: 16}, m_ChildAlignment: 4, m_Spacing: 6, m_ChildForceExpandWidth: 1, m_ChildForceExpandHeight: 0, m_ChildControlWidth: 1, m_ChildControlHeight: 0}
--- !u!114 &4
MonoBehaviour: {m_GameObject: {fileID: 1}, m_Enabled: 1, m_Script: {fileID: 11500000, guid: 3245ec927659c4140ac4f8d17403cc18, type: 3}, m_HorizontalFit: 1, m_VerticalFit: 2}
--- !u!1 &5
GameObject: {m_Name: ToolbarRow, m_Layer: 5}
--- !u!224 &6
RectTransform: {m_GameObject: {fileID: 5}, m_Father: {fileID: 2}, m_SizeDelta: {x: 0, y: 40}}
--- !u!114 &7
MonoBehaviour: {m_GameObject: {fileID: 5}, m_Enabled: 1, m_Script: {fileID: 11500000, guid: 306cc8c2b49d7114eaa3623786fc2126, type: 3}, m_Padding: {m_Left: 4, m_Right: 4, m_Top: 2, m_Bottom: 2}, m_ChildAlignment: 3, m_Spacing: 8, m_ChildForceExpandWidth: 0, m_ChildForceExpandHeight: 1, m_ChildControlWidth: 1, m_ChildControlHeight: 1}
";

    #[test]
    fn ui_scene_exposes_layout_groups_and_content_size_fitter() -> Result<(), Box<dyn Error>> {
        let assembled = assemble_scene_bundle(&unity_yaml::parse(LAYOUT_GROUP_SCENE)?)?;
        assert_eq!(assembled.ui.layout_groups.len(), 2);
        let v = &assembled.ui.layout_groups[0];
        assert_eq!(v.entity, EntityId::new(0));
        assert_eq!(v.axis, LayoutGroupAxis::Vertical);
        assert_eq!((v.padding.left, v.padding.right), (10, 12));
        assert_eq!((v.padding.top, v.padding.bottom), (8, 16));
        assert_eq!((v.child_alignment, v.spacing, v.enabled), (4, 6.0, true));
        assert!(v.child_control_width && !v.child_control_height);
        assert!(v.child_force_expand_width && !v.child_force_expand_height);
        let h = &assembled.ui.layout_groups[1];
        assert_eq!(h.entity, EntityId::new(1));
        assert_eq!(h.axis, LayoutGroupAxis::Horizontal);
        assert_eq!((h.padding.left, h.padding.right), (4, 4));
        assert_eq!((h.padding.top, h.padding.bottom), (2, 2));
        assert_eq!((h.child_alignment, h.spacing), (3, 8.0));
        assert!(h.child_control_width && h.child_control_height);
        assert!(!h.child_force_expand_width && h.child_force_expand_height);
        let f = &assembled.ui.content_size_fitters[0];
        assert_eq!((f.entity, f.enabled), (EntityId::new(0), true));
        assert_eq!(f.horizontal_fit, ContentSizeFitMode::MinSize);
        assert_eq!(f.vertical_fit, ContentSizeFitMode::PreferredSize);
        Ok(())
    }
}
