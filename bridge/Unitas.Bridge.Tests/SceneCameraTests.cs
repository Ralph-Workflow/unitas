using System;
using System.Globalization;
using System.IO;
using System.Text.Json;
using Unitas.Bridge.Playtest;
using UnityEngine;

namespace Unitas.Bridge.Tests
{
    public static class SceneCameraTests
    {
        private const string SceneRelativePath = "samples/marble-rush/Assets/Scenes/MarbleRush.unity";
        private const string CameraName = "Main Camera";
        private const int CaptureSize = 64;

        public static void Run()
        {
            SceneCameraIsMaterialized();
            RenderCaptureWritesABmp();
        }

        private static void SceneCameraIsMaterialized()
        {
            PurgeStaleCameras();
            using (BridgeHost host = new BridgeHost())
            {
                host.LoadScene(ResolveFixture(SceneRelativePath));

                GameObject? cameraObject = GameObject.Find(CameraName);
                Check.That("The scene camera object exists once the scene loads", cameraObject != null);
                Camera? camera = cameraObject != null ? cameraObject.GetComponent<Camera>() : null;
                Check.That("The scene camera object carries a camera component", camera != null);
                if (camera == null)
                {
                    return;
                }

                Check.Equal("Scene camera field of view", 60f, camera.fieldOfView);
                Check.Equal("Scene camera near clip plane", 0.1f, camera.nearClipPlane);
                Check.Equal("Scene camera far clip plane", 200f, camera.farClipPlane);
                Check.Equal("Scene camera orthographic size", 5f, camera.orthographicSize);
                Check.Equal("Scene camera orthographic", false, camera.orthographic);
                Check.Equal("Scene camera background red", 0.08f, camera.backgroundColor.r);
                Check.Equal("Scene camera background green", 0.11f, camera.backgroundColor.g);
                Check.Equal("Scene camera background blue", 0.16f, camera.backgroundColor.b);
            }
        }

        private static void RenderCaptureWritesABmp()
        {
            PurgeStaleCameras();
            string size = CaptureSize.ToString(CultureInfo.InvariantCulture);
            string json = "{\"name\":\"scene-camera-capture\",\"scene\":"
                + JsonSerializer.Serialize(ResolveFixture(SceneRelativePath))
                + ",\"checkpoints\":[{\"id\":\"shot\",\"after_step\":0,\"kind\":\"render_capture\",\"camera\":\"Main Camera\",\"width\":"
                + size
                + ",\"height\":"
                + size
                + "}]}";
            ScenarioDocument document = ScenarioDocument.Parse(json);
            ScenarioResult result = new ScenarioRunner().Run(document);

            Check.Equal("Scene camera capture scenario passed", "passed", result.Status.ToString().ToLowerInvariant());
            Check.That("Scene camera capture reported one checkpoint", result.Checkpoints.Count == 1);
            if (result.Checkpoints.Count == 1)
            {
                Check.Equal(
                    "Scene camera capture checkpoint passed",
                    "passed",
                    result.Checkpoints[0].Status.ToString().ToLowerInvariant());
            }

            Check.That("Scene camera capture wrote one capture file", result.Captures.Count == 1);
            if (result.Captures.Count != 1)
            {
                return;
            }

            string capturePath = result.Captures[0];
            Check.That("Scene camera capture file exists", File.Exists(capturePath));
            Check.That("Scene camera capture file is not empty", new FileInfo(capturePath).Length > 0);
            BmpImage? image = LoadBmp(capturePath);
            Check.That("Scene camera capture file is a readable bitmap", image != null);
            if (image == null)
            {
                return;
            }

            Check.Equal("Scene camera capture width", CaptureSize, image.Width);
            Check.Equal("Scene camera capture height", CaptureSize, image.Height);
        }

        private static BmpImage? LoadBmp(string path)
        {
            try
            {
                return BmpImage.Load(path);
            }
            catch (InvalidDataException ex)
            {
                Console.WriteLine("DIAG capture file could not be read: " + ex.Message);
                return null;
            }
        }

        // GameObject.Find returns the first live match process-wide, so a camera object left
        // behind by an earlier run would shadow the one this test loads.
        private static void PurgeStaleCameras()
        {
            for (int guard = 0; guard < 100; guard++)
            {
                GameObject? stale = GameObject.Find(CameraName);
                if (stale == null)
                {
                    return;
                }

                UnityEngine.Object.Destroy(stale);
            }
        }

        private static string ResolveFixture(string relativePath)
        {
            return File.Exists(relativePath)
                ? Path.GetFullPath(relativePath)
                : Path.GetFullPath("../../../../../" + relativePath);
        }
    }
}
