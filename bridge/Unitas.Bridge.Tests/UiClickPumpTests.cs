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
        // The script GUID of the uGUI Button component in UnityEngine.UI.dll.
        private const string UiButtonGuid = "4e29b1a8efbd4b44bb3f3716e73f07ff";
        private const string ButtonObjectName = "PlayButton";
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
            try
            {
                ScriptGlueRegistry.Clear();
                ScriptGlueRegistry.Register<Button>(UiButtonGuid, () => new Button());
                ClickReachesTheManagedButton(scenePath);
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
    }
}
