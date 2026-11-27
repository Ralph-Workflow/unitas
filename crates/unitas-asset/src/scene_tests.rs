use super::*;
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
