using System;
using System.Globalization;
using System.IO;
using Unitas.Bridge.Scripting;
using UnityEngine;
using UnityEngine.UI;

namespace Unitas.Bridge.Tests
{
    /// <summary>
    /// A scenario-injected pointer click over a scene button must reach the
    /// managed uGUI button and fire its <c>onClick</c> listeners.
    ///
    /// The scene here is shaped like the title screen of the roll-a-ball
    /// sample: a canvas with a scaler, a panel that sits off the middle of the
    /// canvas and a button that sits off the middle of the panel. The sample
    /// injects the pointer at the button and expects the button's handler to
    /// run, so the native raycast has to place the button where the managed
    /// side believes it is.
    /// </summary>
    public static class UiPointerClickTests
    {
        private const string CanvasObjectName = "HudCanvas";
        private const string PanelObjectName = "TitlePanel";
        private const string ButtonObjectName = "PlayButton";
        private const double FrameSeconds = 0.016;

        // The button of the panel below: the centre of the play button of the
        // scene this test drives, in screen pixels.
        private const double ButtonCenterX = 510.0;
        private const double ButtonCenterY = 340.0;

        public static void Run()
        {
            string tempDir = Path.Combine(
                Path.GetTempPath(),
                "unitas_bridge_ui_pointer_" + Guid.NewGuid().ToString("N"));
            Directory.CreateDirectory(tempDir);
            string scenePath = Path.Combine(tempDir, "pointer_ui.unity");
            File.WriteAllText(scenePath, TitleScreenSceneYaml);
            try
            {
                ScriptGlueRegistry.Clear();
                PointerClickReachesTheButton(scenePath);
                SecondSceneTakesTheClick(scenePath);
            }
            finally
            {
                ScriptGlueRegistry.Clear();
                Directory.Delete(tempDir, true);
            }
        }

        private static void PointerClickReachesTheButton(string scenePath)
        {
            int clicks = 0;
            using (BridgeHost host = new BridgeHost())
            {
                Check.That("The bridge host installs the native uGUI provider", Ui.Provider != null);
                host.LoadScene(scenePath);
                host.Tick(FrameSeconds);

                GameObject? canvas = GameObject.Find(CanvasObjectName);
                Check.That("The scene canvas object exists once the scene loads", canvas != null);
                GameObject? panel = GameObject.Find(PanelObjectName);
                Check.That("The scene panel object exists once the scene loads", panel != null);
                GameObject? owner = GameObject.Find(ButtonObjectName);
                Check.That("The scene button object exists once the scene loads", owner != null);
                Button? button = owner != null ? owner.GetComponent<Button>() : null;
                Check.That("The scene button object carries a managed uGUI button", button != null);
                if (button == null)
                {
                    return;
                }

                Check.That("The managed uGUI button is bound to the native scene object", button.NativeObjectId != 0UL);
                Check.That("The managed uGUI button stays interactable", button.interactable);
                button.onClick.AddListener(() =>
                {
                    clicks++;
                    Debug.Log("Title play clicked");
                });

                // The pointer has to reach the button before the press, so the
                // native raycast sees the button under the pointer.
                host.InjectPointerPosition(ButtonCenterX, ButtonCenterY);
                host.Tick(FrameSeconds);
                Check.That("The pointer over the button counts as pointer over the interface", Ui.Provider != null && Ui.Provider.IsPointerOver());

                // A press alone is not a click.
                host.InjectMouseButton(0, true);
                host.Tick(FrameSeconds);
                Check.Equal("The press alone takes no click", 0, clicks);

                // The release is the frame that turns the press into a click.
                host.InjectMouseButton(0, false);
                host.Tick(FrameSeconds);
                Check.Equal("The pumped click reaches the managed onClick handler", 1, clicks);
            }
        }

        private static void SecondSceneTakesTheClick(string scenePath)
        {
            int firstClicks = 0;
            int secondClicks = 0;
            using (BridgeHost host = new BridgeHost())
            {
                host.LoadScene(scenePath);
                host.Tick(FrameSeconds);
                Button? first = GameObject.Find(ButtonObjectName)?.GetComponent<Button>();
                Check.That("The first scene button exists", first != null);
                if (first != null)
                {
                    first.onClick.AddListener(() => firstClicks++);
                }

                host.LoadScene(scenePath);
                host.Tick(FrameSeconds);
                Button? second = GameObject.Find(ButtonObjectName)?.GetComponent<Button>();
                Check.That("The second scene button exists", second != null);
                if (second != null)
                {
                    second.onClick.AddListener(() => secondClicks++);
                }

                host.InjectPointerPosition(ButtonCenterX, ButtonCenterY);
                host.Tick(FrameSeconds);
                host.InjectMouseButton(0, true);
                host.Tick(FrameSeconds);
                host.InjectMouseButton(0, false);
                host.Tick(FrameSeconds);
                Check.Equal("The first scene button takes no click", 0, firstClicks);
                Check.Equal("The second scene button takes the click", 1, secondClicks);
            }
        }

        // A canvas of 800 by 600 reference pixels that holds a panel off the
        // middle of the canvas and a button off the middle of the panel: the
        // shape the title screen of the roll-a-ball sample states.
        private const string TitleScreenSceneYaml = @"%YAML 1.1
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
  m_Name: TitlePanel
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
  m_AnchoredPosition: {x: 100, y: -50}
  m_SizeDelta: {x: 360, y: 220}
  m_Pivot: {x: 0.5, y: 0.5}
--- !u!114 &23
MonoBehaviour:
  m_GameObject: {fileID: 20}
  m_Enabled: 1
  m_Script: {fileID: 11500000, guid: fe87c0e1cc204ed48ad3b37840f39efc, type: 3}
  m_Color: {r: 0.2, g: 0.2, b: 0.2, a: 1}
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
  m_Father: {fileID: 21}
--- !u!224 &32
RectTransform:
  m_GameObject: {fileID: 30}
  m_Father: {fileID: 22}
  m_AnchorMin: {x: 0.5, y: 0.5}
  m_AnchorMax: {x: 0.5, y: 0.5}
  m_AnchoredPosition: {x: 0, y: 10}
  m_SizeDelta: {x: 200, y: 48}
  m_Pivot: {x: 0.5, y: 0.5}
--- !u!114 &33
MonoBehaviour:
  m_GameObject: {fileID: 30}
  m_Enabled: 1
  m_Script: {fileID: 11500000, guid: fe87c0e1cc204ed48ad3b37840f39efc, type: 3}
  m_Color: {r: 0.2, g: 0.5, b: 0.9, a: 1}
--- !u!114 &34
MonoBehaviour:
  m_GameObject: {fileID: 30}
  m_Enabled: 1
  m_Script: {fileID: 11500000, guid: 4e29b1a8efbd4b44bb3f3716e73f07ff, type: 3}
  m_TargetGraphic: {fileID: 33}
  m_Interactable: 1
";
    }
}
