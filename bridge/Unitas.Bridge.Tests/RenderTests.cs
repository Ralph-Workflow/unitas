using System;
using System.Collections.Generic;
using System.IO;
using UnityEngine;

namespace Unitas.Bridge.Tests
{
    public sealed class RenderCameraScript : MonoBehaviour
    {
    }

    public sealed class RenderQuadScript : MonoBehaviour
    {
    }

    public sealed class RenderLightScript : MonoBehaviour
    {
    }

    public static class RenderTests
    {
        private const string CameraGuid = "11112222333344445555666677778888";
        private const string QuadGuid = "22222222333344445555666677778888";
        private const string LightGuid = "33332222333344445555666677778888";
        private const int FrameWidth = 64;
        private const int FrameHeight = 48;
        private const float SceneDepth = 5f;
        private const float HalfQuadSize = 0.5f;
        private const float CentreTolerance = 0.05f;
        private const float AxisTolerance = 0.001f;

        private const string RenderSceneYaml = @"%YAML 1.1
%TAG !u! tag:unity3d.com,2011:
--- !u!1 &100
GameObject:
  m_Name: RenderCamera
  m_Layer: 0
--- !u!4 &101
Transform:
  m_GameObject: {fileID: 100}
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalPosition: {x: 0, y: 0, z: 0}
  m_LocalScale: {x: 1, y: 1, z: 1}
  m_Children: []
  m_Father: {fileID: 0}
--- !u!114 &102
MonoBehaviour:
  m_GameObject: {fileID: 100}
  m_Enabled: 1
  m_Script: {fileID: 11500000, guid: 11112222333344445555666677778888, type: 3}
--- !u!1 &200
GameObject:
  m_Name: RenderQuad
  m_Layer: 0
--- !u!4 &201
Transform:
  m_GameObject: {fileID: 200}
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalPosition: {x: 0, y: 0, z: 0}
  m_LocalScale: {x: 1, y: 1, z: 1}
  m_Children: []
  m_Father: {fileID: 0}
--- !u!114 &202
MonoBehaviour:
  m_GameObject: {fileID: 200}
  m_Enabled: 1
  m_Script: {fileID: 11500000, guid: 22222222333344445555666677778888, type: 3}
--- !u!1 &300
GameObject:
  m_Name: RenderLight
  m_Layer: 0
--- !u!4 &301
Transform:
  m_GameObject: {fileID: 300}
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalPosition: {x: 0, y: 0, z: 0}
  m_LocalScale: {x: 1, y: 1, z: 1}
  m_Children: []
  m_Father: {fileID: 0}
--- !u!114 &302
MonoBehaviour:
  m_GameObject: {fileID: 300}
  m_Enabled: 1
  m_Script: {fileID: 11500000, guid: 33332222333344445555666677778888, type: 3}
";

        public static void Run()
        {
            TestNullProviderSafety();
            TestSharedMaterialAliasing();
            TestUnsupportedLightType();
            TestEndToEndRender();
        }

        private static void TestNullProviderSafety()
        {
            IRenderProvider? savedCamera = Camera.Provider;
            IRenderProvider? savedMesh = Mesh.Provider;
            IRenderProvider? savedMaterial = Material.Provider;
            IRenderProvider? savedFilter = MeshFilter.Provider;
            IRenderProvider? savedRenderer = MeshRenderer.Provider;
            IRenderProvider? savedLight = Light.Provider;
            Camera.Provider = null;
            Mesh.Provider = null;
            Material.Provider = null;
            MeshFilter.Provider = null;
            MeshRenderer.Provider = null;
            Light.Provider = null;
            try
            {
                GameObject cameraObject = new GameObject("ProviderlessCamera");
                Camera camera = cameraObject.AddComponent<Camera>();
                Check.Equal("Camera keeps its default field of view without a provider", 60f, camera.fieldOfView);
                camera.fieldOfView = 45f;
                Check.Equal("Camera caches the field of view without a provider", 45f, camera.fieldOfView);
                camera.backgroundColor = new Color(0.25f, 0.5f, 0.75f, 1f);
                Check.Equal("Camera caches the background colour without a provider", 0.25f, camera.backgroundColor.r);
                Check.That("Camera projection is zero without a provider", camera.WorldToScreenPoint(new Vector3(1f, 2f, 3f)) == Vector3.zero);
                Ray ray = camera.ScreenPointToRay(new Vector3(1f, 2f, 3f));
                Check.That("Camera ray is empty without a provider", ray.origin == Vector3.zero && ray.direction == Vector3.zero);
                Check.That("Camera viewport point is zero without a provider", camera.ViewportToWorldPoint(new Vector3(0.5f, 0.5f, 1f)) == Vector3.zero);

                Mesh mesh = new Mesh();
                Vector3[] meshVertices = { Vector3.zero, Vector3.up };
                Vector3[] meshNormals = { Vector3.up, Vector3.up };
                int[] meshTriangles = { 0, 1, 0 };
                mesh.SetVertices(meshVertices);
                mesh.SetNormals(meshNormals);
                mesh.SetTriangles(meshTriangles);
                Check.Equal("Mesh caches vertices without a provider", 2, mesh.vertices.Length);
                Check.Equal("Mesh caches normals without a provider", 2, mesh.normals.Length);
                Check.Equal("Mesh caches triangles without a provider", 3, mesh.triangles.Length);

                Material material = new Material();
                material.SetColor(new Color(1f, 0f, 0f, 1f));
                Check.Equal("Material caches the colour without a provider", 1f, material.color.r);
                Check.Equal("Material caches the shader without a provider", "Unitas/Standard", material.shader);

                GameObject quadObject = new GameObject("ProviderlessQuad");
                MeshFilter filter = quadObject.AddComponent<MeshFilter>();
                MeshRenderer renderer = quadObject.AddComponent<MeshRenderer>();
                filter.sharedMesh = mesh;
                renderer.sharedMaterial = material;
                renderer.enabled = false;
                Check.That("Mesh filter keeps the mesh without a provider", filter.sharedMesh == mesh);
                Check.That("Mesh renderer keeps the material without a provider", renderer.sharedMaterial == material);
                Check.Equal("Mesh renderer keeps the disabled state without a provider", false, renderer.enabled);

                GameObject lightObject = new GameObject("ProviderlessLight");
                Light light = lightObject.AddComponent<Light>();
                light.intensity = 2f;
                light.color = new Color(0f, 1f, 0f, 1f);
                Check.Equal("Light caches the intensity without a provider", 2f, light.intensity);
                Check.Equal("Light caches the colour without a provider", 1f, light.color.g);
                Check.That("Light keeps the directional type without a provider", light.type == LightType.Directional);

                string path = Path.Combine(Path.GetTempPath(), "unitas-render-no-provider.bmp");
                if (File.Exists(path))
                {
                    File.Delete(path);
                }

                camera.CaptureToFile(path);
                Check.That("Capture without a provider writes no file", !File.Exists(path));
            }
            finally
            {
                Camera.Provider = savedCamera;
                Mesh.Provider = savedMesh;
                Material.Provider = savedMaterial;
                MeshFilter.Provider = savedFilter;
                MeshRenderer.Provider = savedRenderer;
                Light.Provider = savedLight;
            }
        }

        private static void TestSharedMaterialAliasing()
        {
            IRenderProvider? savedCamera = Camera.Provider;
            IRenderProvider? savedMesh = Mesh.Provider;
            IRenderProvider? savedMaterial = Material.Provider;
            IRenderProvider? savedFilter = MeshFilter.Provider;
            IRenderProvider? savedRenderer = MeshRenderer.Provider;
            IRenderProvider? savedLight = Light.Provider;
            RecordingRenderProvider provider = new RecordingRenderProvider();
            Camera.Provider = provider;
            Mesh.Provider = provider;
            Material.Provider = provider;
            MeshFilter.Provider = provider;
            MeshRenderer.Provider = provider;
            Light.Provider = provider;
            try
            {
                Material material = new Material();
                material.SetColor(new Color(1f, 0f, 0f, 1f));

                Mesh mesh = new Mesh();
                Vector3[] meshVertices = { Vector3.zero, Vector3.up };
                Vector3[] meshNormals = { Vector3.up, Vector3.up };
                int[] meshTriangles = { 0, 1, 0 };
                mesh.SetVertices(meshVertices);
                mesh.SetNormals(meshNormals);
                mesh.SetTriangles(meshTriangles);

                GameObject firstObject = new GameObject("FirstQuad");
                MeshFilter firstFilter = firstObject.AddComponent<MeshFilter>();
                MeshRenderer firstRenderer = firstObject.AddComponent<MeshRenderer>();
                firstFilter.sharedMesh = mesh;
                firstRenderer.sharedMaterial = material;

                GameObject secondObject = new GameObject("SecondQuad");
                MeshFilter secondFilter = secondObject.AddComponent<MeshFilter>();
                MeshRenderer secondRenderer = secondObject.AddComponent<MeshRenderer>();
                secondFilter.sharedMesh = mesh;
                secondRenderer.sharedMaterial = material;

                Check.Equal("Both renderers bind the shared material", 2, provider.RendererMaterialHandles.Count);
                Check.That("Shared material resolves to one native handle", provider.RendererMaterialHandles[0] == provider.RendererMaterialHandles[1]);
                Check.That("Shared material handle is valid", provider.RendererMaterialHandles[0] != 0UL);
                Check.That("Both renderers bind the shared mesh", provider.RendererMeshHandles[0] == provider.RendererMeshHandles[1]);

                Check.Equal("Shared material colour reaches the provider once", 1, provider.MaterialColorHandles.Count);
                Check.Equal("Shared material colour is the first colour", 0f, provider.MaterialColors[0].g);

                material.SetColor(new Color(0f, 1f, 0f, 1f));
                Check.Equal("Shared material colour update reaches the provider", 2, provider.MaterialColorHandles.Count);
                Check.That("Shared material colour updates the shared handle", provider.MaterialColorHandles[1] == provider.RendererMaterialHandles[0]);
                Check.Equal("Shared material colour is the new colour", 1f, provider.MaterialColors[1].g);
            }
            finally
            {
                Camera.Provider = savedCamera;
                Mesh.Provider = savedMesh;
                Material.Provider = savedMaterial;
                MeshFilter.Provider = savedFilter;
                MeshRenderer.Provider = savedRenderer;
                Light.Provider = savedLight;
            }
        }

        private static void TestUnsupportedLightType()
        {
            IRenderProvider? savedLight = Light.Provider;
            Light.Provider = null;
            try
            {
                GameObject lightObject = new GameObject("UnsupportedLight");
                Light light = lightObject.AddComponent<Light>();
                try
                {
                    light.type = LightType.Point;
                    Check.That("Point light type is rejected", false);
                }
                catch (NotSupportedException exception)
                {
                    Check.That("Unsupported light message names the type", exception.Message.Contains("Point", StringComparison.Ordinal));
                    Check.That("Unsupported light message names the renderer", exception.Message.Contains("directional", StringComparison.Ordinal));
                }

                Check.That("Light keeps the directional type after a rejected change", light.type == LightType.Directional);
            }
            finally
            {
                Light.Provider = savedLight;
            }
        }

        private static void TestEndToEndRender()
        {
            string tempDir = Path.Combine(
                Path.GetTempPath(),
                "unitas_bridge_render_" + Guid.NewGuid().ToString("N"));
            Directory.CreateDirectory(tempDir);
            string scenePath = Path.Combine(tempDir, "render.unity");
            File.WriteAllText(scenePath, RenderSceneYaml);
            ScriptRegistry.Clear();
            RenderCameraScript? cameraScript = null;
            RenderQuadScript? quadScript = null;
            RenderLightScript? lightScript = null;
            ScriptRegistry.Register(CameraGuid, () => cameraScript = new RenderCameraScript());
            ScriptRegistry.Register(QuadGuid, () => quadScript = new RenderQuadScript());
            ScriptRegistry.Register(LightGuid, () => lightScript = new RenderLightScript());
            try
            {
                using (BridgeHost host = new BridgeHost())
                {
                    host.LoadScene(scenePath);
                    host.Tick(0.0);
                    Check.That("Render camera script is created", cameraScript != null);
                    Check.That("Render quad script is created", quadScript != null);
                    Check.That("Render light script is created", lightScript != null);
                    if (cameraScript == null || quadScript == null || lightScript == null)
                    {
                        return;
                    }

                    GameObject cameraObject = cameraScript.gameObject;
                    GameObject quadObject = quadScript.gameObject;
                    GameObject lightObject = lightScript.gameObject;

                    Camera camera = cameraObject.AddComponent<Camera>();
                    camera.orthographic = false;
                    camera.fieldOfView = 60f;
                    camera.nearClipPlane = 0.3f;
                    camera.farClipPlane = 100f;
                    camera.backgroundColor = new Color(0f, 0f, 0f, 1f);
                    camera.pixelWidth = FrameWidth;
                    camera.pixelHeight = FrameHeight;

                    string capturePath = Path.Combine(tempDir, "capture.bmp");
                    camera.CaptureToFile(capturePath);

                    Vector3 cameraPosition = cameraObject.transform.position;
                    Ray centreRay = camera.ScreenPointToRay(new Vector3(FrameWidth / 2f, FrameHeight / 2f, 0f));
                    Check.That("Screen point ray is normalised", Math.Abs(centreRay.direction.magnitude - 1f) < AxisTolerance);
                    Check.That("Screen point ray starts at the camera position", Vector3.Distance(centreRay.origin, cameraPosition) < AxisTolerance);
                    Check.That(
                        "Screen point ray follows the camera axis",
                        Math.Abs(centreRay.direction.z) > 1f - AxisTolerance
                            && Math.Abs(centreRay.direction.x) < AxisTolerance
                            && Math.Abs(centreRay.direction.y) < AxisTolerance);

                    Vector3 forward = centreRay.direction.normalized;
                    Vector3 toCamera = -forward;
                    Vector3 right = Vector3.Cross(forward, Vector3.up).normalized;
                    Vector3 quadCentre = cameraPosition + (forward * SceneDepth);

                    Mesh mesh = new Mesh();
                    Vector3[] quadVertices =
                    {
                        quadCentre + (right * -HalfQuadSize) + (Vector3.up * -HalfQuadSize),
                        quadCentre + (right * HalfQuadSize) + (Vector3.up * -HalfQuadSize),
                        quadCentre + (right * HalfQuadSize) + (Vector3.up * HalfQuadSize),
                        quadCentre + (right * -HalfQuadSize) + (Vector3.up * HalfQuadSize),
                    };
                    Vector3[] quadNormals = { toCamera, toCamera, toCamera, toCamera };
                    int[] quadTriangles = { 0, 1, 2, 0, 2, 3 };
                    mesh.SetVertices(quadVertices);
                    mesh.SetNormals(quadNormals);
                    mesh.SetTriangles(quadTriangles);

                    Material material = new Material();
                    material.SetColor(new Color(1f, 1f, 1f, 1f));

                    MeshFilter filter = quadObject.AddComponent<MeshFilter>();
                    MeshRenderer renderer = quadObject.AddComponent<MeshRenderer>();
                    filter.sharedMesh = mesh;
                    renderer.sharedMaterial = material;

                    Light light = lightObject.AddComponent<Light>();
                    light.direction = toCamera;
                    light.color = new Color(1f, 1f, 1f, 1f);
                    light.intensity = 1f;

                    camera.CaptureToFile(capturePath);
                    Check.That("Capture writes a file", File.Exists(capturePath));
                    if (!File.Exists(capturePath))
                    {
                        return;
                    }

                    byte[] frame = File.ReadAllBytes(capturePath);
                    Check.That("Capture is a BMP file", frame.Length > 54 && frame[0] == (byte)'B' && frame[1] == (byte)'M');
                    Check.Equal("Capture header carries the requested width", FrameWidth, ReadBmpDimension(frame, 18));
                    Check.Equal("Capture header carries the requested height", FrameHeight, ReadBmpDimension(frame, 22));

                    Vector3 screenPoint = camera.WorldToScreenPoint(quadCentre);
                    Check.That(
                        "Point on the camera axis projects to the frame centre",
                        Math.Abs(screenPoint.x - (FrameWidth / 2f)) < CentreTolerance
                            && Math.Abs(screenPoint.y - (FrameHeight / 2f)) < CentreTolerance);

                    Vector3 viewportPoint = new Vector3(screenPoint.x / FrameWidth, screenPoint.y / FrameHeight, screenPoint.z);
                    Vector3 roundTrip = camera.ViewportToWorldPoint(viewportPoint);
                    Check.That("Viewport round trip returns the world point", Vector3.Distance(roundTrip, quadCentre) < 0.01f);

                    int foreground = FindForegroundBounds(frame, FrameWidth, FrameHeight, out float minX, out float maxX, out float minY, out float maxY);
                    Check.That("Capture contains a pixel that is not the background colour", foreground > 0);
                    Check.That(
                        "Projected point falls inside the rendered object",
                        screenPoint.x >= minX - 1f && screenPoint.x <= maxX + 1f && screenPoint.y >= minY - 1f && screenPoint.y <= maxY + 1f);

                    int[] before = ReadPixel(frame, FrameWidth, FrameHeight, screenPoint.x, screenPoint.y);
                    material.SetColor(new Color(1f, 0f, 0f, 1f));
                    camera.CaptureToFile(capturePath);
                    byte[] recoloured = File.ReadAllBytes(capturePath);
                    int[] after = ReadPixel(recoloured, FrameWidth, FrameHeight, screenPoint.x, screenPoint.y);
                    Check.That(
                        "Shared material colour reaches the rendered frame",
                        before[0] != after[0] || before[1] != after[1] || before[2] != after[2]);
                }
            }
            finally
            {
                ScriptRegistry.Clear();
                if (Directory.Exists(tempDir))
                {
                    Directory.Delete(tempDir, true);
                }
            }
        }

        private static int ReadBmpDimension(byte[] frame, int offset)
        {
            return BitConverter.ToInt32(frame, offset);
        }

        private static int FindForegroundBounds(byte[] frame, int width, int height, out float minX, out float maxX, out float minY, out float maxY)
        {
            int stride = (width * 3 + 3) & ~3;
            int minColumn = width;
            int maxColumn = -1;
            int minRow = height;
            int maxRow = -1;
            int count = 0;
            for (int row = 0; row < height; row++)
            {
                int offset = 54 + (row * stride);
                for (int column = 0; column < width; column++)
                {
                    int pixel = offset + (column * 3);
                    if (frame[pixel] == 0 && frame[pixel + 1] == 0 && frame[pixel + 2] == 0)
                    {
                        continue;
                    }

                    count++;
                    if (column < minColumn)
                    {
                        minColumn = column;
                    }

                    if (column > maxColumn)
                    {
                        maxColumn = column;
                    }

                    if (row < minRow)
                    {
                        minRow = row;
                    }

                    if (row > maxRow)
                    {
                        maxRow = row;
                    }
                }
            }

            minX = minColumn + 0.5f;
            maxX = maxColumn + 0.5f;
            minY = minRow + 0.5f;
            maxY = maxRow + 0.5f;
            return count;
        }

        private static int[] ReadPixel(byte[] frame, int width, int height, float screenX, float screenY)
        {
            int stride = (width * 3 + 3) & ~3;
            int column = Math.Max(0, Math.Min(width - 1, (int)Math.Floor(screenX - 0.5f)));
            int row = Math.Max(0, Math.Min(height - 1, (int)Math.Floor(screenY - 0.5f)));
            int offset = 54 + (row * stride) + (column * 3);
            return new int[] { frame[offset + 2], frame[offset + 1], frame[offset] };
        }

        private sealed class RecordingRenderProvider : IRenderProvider
        {
            public List<ulong> RendererMeshHandles { get; } = new List<ulong>();

            public List<ulong> RendererMaterialHandles { get; } = new List<ulong>();

            public List<ulong> MaterialColorHandles { get; } = new List<ulong>();

            public List<Color> MaterialColors { get; } = new List<Color>();

            public ulong CreateMesh(Vector3[] vertices, Vector3[] normals, int[] triangles)
            {
                return 7UL;
            }

            public void DestroyMesh(ulong meshHandle)
            {
            }

            public ulong CreateMaterial()
            {
                return 11UL;
            }

            public void SetMaterialColor(ulong materialHandle, Color color)
            {
                MaterialColorHandles.Add(materialHandle);
                MaterialColors.Add(color);
            }

            public ulong CreateCamera(ulong entityId)
            {
                return 13UL;
            }

            public void UpdateCamera(ulong cameraHandle, float orthographicSize, float fieldOfView, float nearClipPlane, float farClipPlane, Color backgroundColor)
            {
            }

            public Vector3 WorldToScreenPoint(ulong cameraHandle, Vector3 worldPoint)
            {
                return Vector3.zero;
            }

            public Ray ScreenPointToRay(ulong cameraHandle, Vector3 screenPoint)
            {
                return default;
            }

            public ulong CreateRenderer(ulong entityId, ulong meshHandle, ulong materialHandle)
            {
                RendererMeshHandles.Add(meshHandle);
                RendererMaterialHandles.Add(materialHandle);
                return 17UL;
            }

            public void SetRendererEnabled(ulong rendererHandle, bool enabled)
            {
            }

            public ulong CreateLight(ulong entityId)
            {
                return 19UL;
            }

            public void UpdateLight(ulong lightHandle, Vector3 direction, Color color, float intensity)
            {
            }

            public byte[] RenderFrame(ulong cameraHandle, uint width, uint height)
            {
                return Array.Empty<byte>();
            }
        }
    }
}
