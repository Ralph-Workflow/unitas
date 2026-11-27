using System;
using System.IO;
using Unitas.Bridge.Scripting;
using UnityEngine;
using UnityEngine.UI;

namespace Unitas.Bridge.Tests
{
    /// <summary>
    /// The bridge host pumps the native click queue once per frame, so the
    /// managed uGUI button of a clicked scene object receives the click the
    /// native side dispatched for it.
    /// </summary>
    public static class UiClickPumpTests
    {
        // The script GUID of the uGUI Button component, the one the native
        // scene importer recognizes, so the native side owns the button.
        private const string ButtonObjectName = "PlayButton";
        private const string LockedButtonObjectName = "LockedButton";
        private const string LabelObjectName = "ScoreLabel";
        private const string CanvasObjectName = "HudCanvas";
        private const double FrameSeconds = 0.016;
        private const double ButtonCenterX = 400.0;
        private const double ButtonCenterY = 300.0;

        public static void Run()
        {
            string tempDir = Path.Combine(
                Path.GetTempPath(),
                "unitas_bridge_ui_click_" + Guid.NewGuid().ToString("N"));
            Directory.CreateDirectory(tempDir);
            string scenePath = Path.Combine(tempDir, "ui_click.unity");
            File.WriteAllText(scenePath, UiSceneYaml);
            string builtInScenePath = Path.Combine(tempDir, "ui_built_in.unity");
            File.WriteAllText(builtInScenePath, BuiltInUiSceneYaml);
            try
            {
                ScriptGlueRegistry.Clear();
                ClickReachesTheManagedButton(scenePath);
                BuiltInUiComponentsBindToTheScene(builtInScenePath);
            }
            finally
            {
                ScriptGlueRegistry.Clear();
                Directory.Delete(tempDir, true);
            }
        }

        private static void ClickReachesTheManagedButton(string scenePath)
        {
            int clicks = 0;
            using (BridgeHost host = new BridgeHost())
            {
                Check.That("The bridge host installs the native uGUI provider", Ui.Provider != null);
                host.LoadScene(scenePath);
                host.Tick(FrameSeconds);

                GameObject? owner = GameObject.Find(ButtonObjectName);
                Check.That("The scene button object exists once the scene loads", owner != null);
                Button? button = owner != null ? owner.GetComponent<Button>() : null;
                Check.That("The scene button object carries a managed uGUI button", button != null);
                if (button == null)
                {
                    return;
                }

                Check.That("The managed uGUI button is bound to the native scene object", button.NativeObjectId != 0UL);
                button.onClick.AddListener(() => clicks++);

                host.InjectPointerPosition(ButtonCenterX, ButtonCenterY);
                host.Tick(FrameSeconds);
                host.InjectMouseButton(0, true);
                host.Tick(FrameSeconds);
                host.InjectMouseButton(0, false);
                Check.Equal("The click waits for the frame that pumps it", 0, clicks);

                host.Tick(FrameSeconds);
                Check.Equal("The pumped click reaches the managed onClick handler", 1, clicks);
            }
        }

        // A scene that states its interface with the built-in uGUI components:
        // the bridge registers those script GUIDs, so the managed counterparts
        // bind to the scene objects and carry the state the scene gave them.
        private static void BuiltInUiComponentsBindToTheScene(string scenePath)
        {
            int clicks = 0;
            int lockedClicks = 0;
            using (BridgeHost host = new BridgeHost())
            {
                host.LoadScene(scenePath);
                host.Tick(FrameSeconds);

                GameObject? label = GameObject.Find(LabelObjectName);
                Check.That("The scene label object exists once the scene loads", label != null);
                Text? text = label != null ? label.GetComponent<Text>() : null;
                Check.That("The scene label object carries a managed uGUI text", text != null);
                if (text == null)
                {
                    return;
                }

                Check.Equal("The managed uGUI text shows the text the scene gave it", "Score 0", text.text);
                Check.Equal("The managed uGUI text keeps the font size the scene gave it", 18, text.fontSize);
                Check.That("The managed uGUI text defaults to UpperLeft alignment", text.alignment == TextAnchor.UpperLeft);
                Check.That("The managed uGUI text defaults to Overflow horizontalOverflow", text.horizontalOverflow == HorizontalWrapMode.Overflow);
                Check.That("The managed uGUI text defaults to Overflow verticalOverflow", text.verticalOverflow == VerticalWrapMode.Overflow);
                Check.Equal("The managed uGUI text defaults to lineSpacing 1", 1f, text.lineSpacing);

                text.alignment = TextAnchor.MiddleCenter;
                text.horizontalOverflow = HorizontalWrapMode.Wrap;
                text.verticalOverflow = VerticalWrapMode.Truncate;
                text.lineSpacing = 1.5f;
                Check.That("The managed uGUI text updates alignment", text.alignment == TextAnchor.MiddleCenter);
                Check.That("The managed uGUI text updates horizontalOverflow", text.horizontalOverflow == HorizontalWrapMode.Wrap);
                Check.That("The managed uGUI text updates verticalOverflow", text.verticalOverflow == VerticalWrapMode.Truncate);
                Check.Equal("The managed uGUI text updates lineSpacing", 1.5f, text.lineSpacing);

                GameObject? canvas = GameObject.Find(CanvasObjectName);
                CanvasScaler? scaler = canvas != null ? canvas.GetComponent<CanvasScaler>() : null;
                Check.That("The scene canvas object carries a managed uGUI canvas scaler", scaler != null);
                if (scaler != null)
                {
                    Check.That(
                        "The managed uGUI canvas scaler keeps the scale mode the scene gave it",
                        scaler.uiScaleMode == CanvasScaler.ScaleMode.ScaleWithScreenSize);
                    Check.Equal("The managed uGUI canvas scaler keeps the reference width", 800f, scaler.referenceResolution.x);
                    Check.Equal("The managed uGUI canvas scaler keeps the reference height", 600f, scaler.referenceResolution.y);
                }

                GameObject? owner = GameObject.Find(ButtonObjectName);
                Button? button = owner != null ? owner.GetComponent<Button>() : null;
                Check.That("The scene button object carries a managed uGUI button", button != null);
                if (button == null)
                {
                    return;
                }

                Check.That("The managed uGUI button is bound to the native scene object", button.NativeObjectId != 0UL);
                Check.That("The managed uGUI button stays interactable", button.interactable);
                button.onClick.AddListener(() => clicks++);

                GameObject? lockedOwner = GameObject.Find(LockedButtonObjectName);
                Button? locked = lockedOwner != null ? lockedOwner.GetComponent<Button>() : null;
                Check.That("The locked scene button object carries a managed uGUI button", locked != null);
                if (locked != null)
                {
                    Check.That("The locked managed uGUI button stays uninteractable", !locked.interactable);
                    locked.onClick.AddListener(() => lockedClicks++);
                }

                host.InjectPointerPosition(ButtonCenterX, ButtonCenterY);
                host.Tick(FrameSeconds);
                host.InjectMouseButton(0, true);
                host.Tick(FrameSeconds);
                host.InjectMouseButton(0, false);
                Check.Equal("The click waits for the frame that pumps it", 0, clicks);

                host.Tick(FrameSeconds);
                Check.Equal("The pumped click reaches the managed onClick handler", 1, clicks);
                Check.Equal("The locked button takes no click", 0, lockedClicks);
            }
        }

        // A canvas of 800 by 600 reference pixels with one button in its middle,
        // the shape the native uGUI importer reads a scene's interface from.
        private const string UiSceneYaml = @"%YAML 1.1
%TAG !u! tag:unity3d.com,2011:
--- !u!1 &1
GameObject:
  m_Name: HudCanvas
  m_Layer: 5
--- !u!4 &2
Transform:
  m_GameObject: {fileID: 1}
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalPosition: {x: 0, y: 0, z: 0}
  m_LocalScale: {x: 1, y: 1, z: 1}
  m_Children: []
  m_Father: {fileID: 0}
--- !u!224 &3
RectTransform:
  m_GameObject: {fileID: 1}
  m_Father: {fileID: 0}
  m_AnchorMin: {x: 0, y: 0}
  m_AnchorMax: {x: 1, y: 1}
  m_AnchoredPosition: {x: 0, y: 0}
  m_SizeDelta: {x: 0, y: 0}
  m_Pivot: {x: 0.5, y: 0.5}
--- !u!223 &4
Canvas:
  m_GameObject: {fileID: 1}
  m_Enabled: 1
  m_RenderMode: 0
  m_SortingOrder: 0
--- !u!114 &5
MonoBehaviour:
  m_GameObject: {fileID: 1}
  m_Enabled: 1
  m_Script: {fileID: 11500000, guid: 0cd44c1031e13a943bb63640046fad76, type: 3}
  m_UiScaleMode: 1
  m_ReferenceResolution: {x: 800, y: 600}
  m_ScreenMatchMode: 0
  m_MatchWidthOrHeight: 0.5
--- !u!1 &20
GameObject:
  m_Name: PlayButton
  m_Layer: 5
--- !u!4 &21
Transform:
  m_GameObject: {fileID: 20}
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalPosition: {x: 0, y: 0, z: 0}
  m_LocalScale: {x: 1, y: 1, z: 1}
  m_Children: []
  m_Father: {fileID: 2}
--- !u!224 &22
RectTransform:
  m_GameObject: {fileID: 20}
  m_Father: {fileID: 3}
  m_AnchorMin: {x: 0.5, y: 0.5}
  m_AnchorMax: {x: 0.5, y: 0.5}
  m_AnchoredPosition: {x: 0, y: 0}
  m_SizeDelta: {x: 200, y: 60}
  m_Pivot: {x: 0.5, y: 0.5}
--- !u!1 &30
GameObject:
  m_Name: ButtonFace
  m_Layer: 5
--- !u!4 &31
Transform:
  m_GameObject: {fileID: 30}
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalPosition: {x: 0, y: 0, z: 0}
  m_LocalScale: {x: 1, y: 1, z: 1}
  m_Children: []
  m_Father: {fileID: 21}
--- !u!224 &32
RectTransform:
  m_GameObject: {fileID: 30}
  m_Father: {fileID: 22}
  m_AnchorMin: {x: 0, y: 0}
  m_AnchorMax: {x: 1, y: 1}
  m_AnchoredPosition: {x: 0, y: 0}
  m_SizeDelta: {x: 0, y: 0}
  m_Pivot: {x: 0.5, y: 0.5}
--- !u!114 &33
MonoBehaviour:
  m_GameObject: {fileID: 30}
  m_Enabled: 1
  m_Script: {fileID: 11500000, guid: fe87c0e1cc204ed48ad3b37840f39efc, type: 3}
  m_Color: {r: 0.9, g: 0.9, b: 0.9, a: 1}
--- !u!114 &34
MonoBehaviour:
  m_GameObject: {fileID: 20}
  m_Enabled: 1
  m_Script: {fileID: 11500000, guid: 4e29b1a8efbd4b44bb3f3716e73f07ff, type: 3}
  m_TargetGraphic: {fileID: 33}
  m_Interactable: 1
";

        // A scene that states its interface with the built-in uGUI components:
        // a canvas with a scaler, a label, a button and a button that the scene
        // marks as not interactable.
        private const string BuiltInUiSceneYaml = @"%YAML 1.1
%TAG !u! tag:unity3d.com,2011:
--- !u!1 &1
GameObject:
  m_Name: HudCanvas
  m_Layer: 5
--- !u!4 &2
Transform:
  m_GameObject: {fileID: 1}
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalPosition: {x: 0, y: 0, z: 0}
  m_LocalScale: {x: 1, y: 1, z: 1}
  m_Children: []
  m_Father: {fileID: 0}
--- !u!224 &3
RectTransform:
  m_GameObject: {fileID: 1}
  m_Father: {fileID: 0}
  m_AnchorMin: {x: 0, y: 0}
  m_AnchorMax: {x: 1, y: 1}
  m_AnchoredPosition: {x: 0, y: 0}
  m_SizeDelta: {x: 0, y: 0}
  m_Pivot: {x: 0.5, y: 0.5}
--- !u!223 &4
Canvas:
  m_GameObject: {fileID: 1}
  m_Enabled: 1
  m_RenderMode: 0
  m_SortingOrder: 0
--- !u!114 &5
MonoBehaviour:
  m_GameObject: {fileID: 1}
  m_Enabled: 1
  m_Script: {fileID: 11500000, guid: 0cd44c1031e13a943bb63640046fad76, type: 3}
  m_UiScaleMode: 1
  m_ReferenceResolution: {x: 800, y: 600}
  m_ScreenMatchMode: 0
  m_MatchWidthOrHeight: 0.5
--- !u!1 &20
GameObject:
  m_Name: ScoreLabel
  m_Layer: 5
--- !u!4 &21
Transform:
  m_GameObject: {fileID: 20}
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalPosition: {x: 0, y: 0, z: 0}
  m_LocalScale: {x: 1, y: 1, z: 1}
  m_Children: []
  m_Father: {fileID: 2}
--- !u!224 &22
RectTransform:
  m_GameObject: {fileID: 20}
  m_Father: {fileID: 3}
  m_AnchorMin: {x: 0, y: 1}
  m_AnchorMax: {x: 0, y: 1}
  m_AnchoredPosition: {x: 16, y: -12}
  m_SizeDelta: {x: 320, y: 28}
  m_Pivot: {x: 0, y: 1}
--- !u!114 &23
MonoBehaviour:
  m_GameObject: {fileID: 20}
  m_Enabled: 1
  m_Script: {fileID: 11500000, guid: 5f7201a12d95ffc409449d95f23cf332, type: 3}
  m_Text: Score 0
  m_FontSize: 18
  m_Color: {r: 1, g: 1, b: 1, a: 1}
--- !u!1 &30
GameObject:
  m_Name: PlayButton
  m_Layer: 5
--- !u!4 &31
Transform:
  m_GameObject: {fileID: 30}
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalPosition: {x: 0, y: 0, z: 0}
  m_LocalScale: {x: 1, y: 1, z: 1}
  m_Children: []
  m_Father: {fileID: 2}
--- !u!224 &32
RectTransform:
  m_GameObject: {fileID: 30}
  m_Father: {fileID: 3}
  m_AnchorMin: {x: 0.5, y: 0.5}
  m_AnchorMax: {x: 0.5, y: 0.5}
  m_AnchoredPosition: {x: 0, y: 0}
  m_SizeDelta: {x: 200, y: 60}
  m_Pivot: {x: 0.5, y: 0.5}
--- !u!114 &33
MonoBehaviour:
  m_GameObject: {fileID: 30}
  m_Enabled: 1
  m_Script: {fileID: 11500000, guid: fe87c0e1cc204ed48ad3b37840f39efc, type: 3}
  m_Color: {r: 0.2, g: 0.5, b: 0.9, a: 1}
  m_Sprite: {fileID: 0}
--- !u!114 &34
MonoBehaviour:
  m_GameObject: {fileID: 30}
  m_Enabled: 1
  m_Script: {fileID: 11500000, guid: 4e29b1a8efbd4b44bb3f3716e73f07ff, type: 3}
  m_TargetGraphic: {fileID: 33}
  m_Interactable: 1
--- !u!1 &40
GameObject:
  m_Name: LockedButton
  m_Layer: 5
--- !u!4 &41
Transform:
  m_GameObject: {fileID: 40}
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalPosition: {x: 0, y: 0, z: 0}
  m_LocalScale: {x: 1, y: 1, z: 1}
  m_Children: []
  m_Father: {fileID: 2}
--- !u!224 &42
RectTransform:
  m_GameObject: {fileID: 40}
  m_Father: {fileID: 3}
  m_AnchorMin: {x: 0.5, y: 0.5}
  m_AnchorMax: {x: 0.5, y: 0.5}
  m_AnchoredPosition: {x: 0, y: 0}
  m_SizeDelta: {x: 200, y: 60}
  m_Pivot: {x: 0.5, y: 0.5}
--- !u!114 &43
MonoBehaviour:
  m_GameObject: {fileID: 40}
  m_Enabled: 1
  m_Script: {fileID: 11500000, guid: fe87c0e1cc204ed48ad3b37840f39efc, type: 3}
  m_Color: {r: 0.2, g: 0.5, b: 0.9, a: 1}
  m_Sprite: {fileID: 0}
--- !u!114 &44
MonoBehaviour:
  m_GameObject: {fileID: 40}
  m_Enabled: 1
  m_Script: {fileID: 11500000, guid: 4e29b1a8efbd4b44bb3f3716e73f07ff, type: 3}
  m_TargetGraphic: {fileID: 43}
  m_Interactable: 0
";
    }
}
