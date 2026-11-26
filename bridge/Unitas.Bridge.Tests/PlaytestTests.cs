using System;
using System.Globalization;
using System.IO;
using System.Text.Json;
using System.Text.Json.Nodes;
using Unitas.Bridge.Playtest;
using Unitas.Bridge.Scripting;
using UnityEngine;

namespace Unitas.Bridge.Tests
{
    public sealed class PlaytestBallScript : MonoBehaviour
    {
        private static readonly Vector3 SettledPosition = new Vector3(1f, 0.5f, 0f);

        public PlaytestBallScript()
        {
            name = "Ball";
        }

        public override void Awake()
        {
            transform.position = Vector3.zero;
        }

        public override void Start()
        {
            transform.position = SettledPosition;
        }

        public override void Update()
        {
            transform.position = SettledPosition;
        }
    }

    public sealed class PlaytestRecorderScript : MonoBehaviour
    {
        private bool _logged;

        public PlaytestRecorderScript()
        {
            name = "Recorder";
        }

        public override void Update()
        {
            if (_logged)
            {
                return;
            }

            _logged = true;
            Debug.Log("recorder started");
        }
    }

    public sealed class PlaytestCameraScript : MonoBehaviour
    {
        public PlaytestCameraScript()
        {
            name = "Main Camera";
        }

        public override void Awake()
        {
            Camera camera = gameObject.AddComponent<Camera>();
            camera.orthographic = false;
            camera.fieldOfView = 60f;
            camera.nearClipPlane = 0.3f;
            camera.farClipPlane = 1000f;
            camera.backgroundColor = new Color(0f, 0f, 0f, 1f);
            camera.pixelWidth = 64;
            camera.pixelHeight = 64;

            Light light = gameObject.AddComponent<Light>();
            light.direction = new Vector3(0f, 0f, -1f);
            light.color = new Color(1f, 1f, 1f, 1f);
            light.intensity = 1f;
        }
    }

    public sealed class PlaytestQuadScript : MonoBehaviour
    {
        private static readonly int[] QuadTriangles = { 0, 1, 2, 0, 2, 3 };

        public PlaytestQuadScript()
        {
            name = "Quad";
        }

        public override void Awake()
        {
            Vector3 centre = new Vector3(0f, 0f, 5f);
            Vector3 right = new Vector3(-1f, 0f, 0f);
            Vector3 up = new Vector3(0f, 1f, 0f);
            Vector3 normal = new Vector3(0f, 0f, -1f);
            const float halfSize = 20f;

            Mesh mesh = new Mesh();
            mesh.SetVertices(new Vector3[]
            {
                centre + (right * -halfSize) + (up * -halfSize),
                centre + (right * halfSize) + (up * -halfSize),
                centre + (right * halfSize) + (up * halfSize),
                centre + (right * -halfSize) + (up * halfSize),
            });
            mesh.SetNormals(new Vector3[] { normal, normal, normal, normal });
            mesh.SetTriangles(QuadTriangles);

            Material material = new Material();
            material.SetColor(new Color(1f, 1f, 1f, 1f));

            MeshFilter filter = gameObject.AddComponent<MeshFilter>();
            filter.sharedMesh = mesh;
            MeshRenderer renderer = gameObject.AddComponent<MeshRenderer>();
            renderer.sharedMaterial = material;
        }
    }

    public static class PlaytestTests
    {
        private const string FixtureDirectory = "bridge/Unitas.Bridge.Tests/Fixtures/playtest/";
        private const string SceneFileName = "Assets/Scenes/playtest_scene.unity";
        private const string BallGuid = "a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1";
        private const string RecorderGuid = "b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2";
        private const string CameraGuid = "c3c3c3c3c3c3c3c3c3c3c3c3c3c3c3c3";
        private const string QuadGuid = "d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4";

        private static readonly string[] ExpectedStepKinds =
        {
            "load_scene",
            "tick",
            "inject_key",
            "inject_mouse_button",
            "inject_pointer",
            "inject_scroll",
            "set_time_scale",
        };

        public static void Run()
        {
            ScriptGlueRegistry.Clear();
            ScriptGlueRegistry.Register(BallGuid, () => new PlaytestBallScript());
            ScriptGlueRegistry.Register(RecorderGuid, () => new PlaytestRecorderScript());
            ScriptGlueRegistry.Register(CameraGuid, () => new PlaytestCameraScript());
            ScriptGlueRegistry.Register(QuadGuid, () => new PlaytestQuadScript());
            try
            {
                FullPassScenario();
                FailingCheckpointScenario();
                UnevaluableCheckpointScenario();
                SetupFailureScenario();
                RepeatabilityScenario();
                ReloadedSceneKeepsCameraRenderable();
                VisualReferenceTests.Run();
            }
            finally
            {
                ScriptGlueRegistry.Clear();
            }
        }

        private static void FullPassScenario()
        {
            ScenarioDocument document = ScenarioDocument.Parse(LoadScenarioJson("full_pass.json"));
            ScenarioResult result = new ScenarioRunner().Run(document);
            JsonNode report = ParseReport(result);

            Check.Equal("Full pass reports the scenario name", "full-pass", GetString(report, "scenario"));
            Check.Equal("Full pass status is passed", "passed", GetString(report, "status"));
            Check.Equal("Full pass setup succeeded", true, GetBool(GetNode(report, "setup"), "ok"));

            JsonArray steps = GetArray(report, "steps");
            Check.Equal("Full pass reports one entry per executed step", ExpectedStepKinds.Length, steps.Count);
            for (int index = 0; index < ExpectedStepKinds.Length && index < steps.Count; index++)
            {
                Check.Equal(
                    "Full pass step " + index.ToString(CultureInfo.InvariantCulture) + " kind is " + ExpectedStepKinds[index],
                    ExpectedStepKinds[index],
                    GetString(steps[index]!, "kind"));
            }

            JsonArray checkpoints = GetArray(report, "checkpoints");
            Check.Equal("Full pass reports four checkpoints", 4, checkpoints.Count);
            for (int index = 0; index < checkpoints.Count; index++)
            {
                JsonNode checkpoint = checkpoints[index]!;
                string id = GetString(checkpoint, "id");
                string status = GetString(checkpoint, "status");
                string reason = GetString(checkpoint, "reason");
                Check.Equal(
                    "Full pass checkpoint " + id + " passed"
                        + (status == "passed" || reason.Length == 0 ? string.Empty : " (" + reason + ")"),
                    "passed",
                    status);
            }

            Check.That("Full pass collected log messages", GetArray(report, "messages").Count > 0);

            JsonArray captures = GetArray(report, "captures");
            Check.Equal("Full pass captured one frame", 1, captures.Count);
            if (captures.Count == 1)
            {
                string capturePath = GetText(captures[0]!);
                Check.That("Full pass capture file exists on disk", File.Exists(capturePath));
            }
        }

        private static void ReloadedSceneKeepsCameraRenderable()
        {
            ScenarioDocument document = ScenarioDocument.Parse(ResolveScenePaths(ReloadedSceneScenarioJson()));
            ScenarioResult result = new ScenarioRunner().Run(document);
            JsonNode report = ParseReport(result);

            Check.Equal("Reloaded scene scenario status is passed", "passed", GetString(report, "status"));
            JsonArray checkpoints = GetArray(report, "checkpoints");
            Check.Equal("Reloaded scene reports one checkpoint", 1, checkpoints.Count);
            if (checkpoints.Count == 1)
            {
                JsonNode checkpoint = checkpoints[0]!;
                string status = GetString(checkpoint, "status");
                string reason = GetString(checkpoint, "reason");
                Check.Equal(
                    "Reloaded scene checkpoint shot passed"
                        + (status == "passed" || reason.Length == 0 ? string.Empty : " (" + reason + ")"),
                    "passed",
                    status);
            }

            Check.Equal("Reloaded scene captured one frame", 1, GetArray(report, "captures").Count);
        }

        private static string ReloadedSceneScenarioJson()
        {
            string scene = "\"" + FixtureDirectory + SceneFileName + "\"";
            return "{"
                + "\"name\": \"reloaded-scene\","
                + "\"scene\": " + scene + ","
                + "\"steps\": ["
                + "{ \"load_scene\": { \"path\": " + scene + ", \"mode\": \"single\" } },"
                + "{ \"load_scene\": { \"path\": " + scene + ", \"mode\": \"single\" } },"
                + "{ \"tick\": { \"frames\": 30, \"delta_seconds\": 0.02 } }"
                + "],"
                + "\"checkpoints\": ["
                + "{ \"id\": \"shot\", \"after_step\": 3, \"kind\": \"render_capture\", \"camera\": \"Main Camera\","
                + " \"width\": 64, \"height\": 64, \"expected_non_background_pixels\": 4096 }"
                + "]"
                + "}";
        }

        private static void FailingCheckpointScenario()
        {
            ScenarioDocument document = ScenarioDocument.Parse(LoadScenarioJson("failing_checkpoint.json"));
            ScenarioResult result = new ScenarioRunner().Run(document);
            JsonNode report = ParseReport(result);

            Check.Equal("Failing checkpoint scenario status is failed", "failed", GetString(report, "status"));
            JsonNode checkpoint = FindCheckpoint(report, "moved");
            Check.Equal("Failing checkpoint is reported as failed", "failed", GetString(checkpoint, "status"));
            string expected = GetString(checkpoint, "expected");
            string observed = GetString(checkpoint, "observed");
            Check.That("Failing checkpoint records the expected value", expected.Length > 0);
            Check.That("Failing checkpoint records the observed value", observed.Length > 0);
            Check.That("Failing checkpoint expected and observed differ", expected != observed);
        }

        private static void UnevaluableCheckpointScenario()
        {
            ScenarioDocument document = ScenarioDocument.Parse(LoadScenarioJson("unevaluable_checkpoint.json"));
            ScenarioResult result = new ScenarioRunner().Run(document);
            JsonNode report = ParseReport(result);

            Check.Equal("Unevaluable checkpoint scenario status is failed", "failed", GetString(report, "status"));
            JsonNode checkpoint = FindCheckpoint(report, "missing");
            Check.Equal("Unevaluable checkpoint is reported as unevaluable", "unevaluable", GetString(checkpoint, "status"));
            Check.That("Unevaluable checkpoint records a reason", GetString(checkpoint, "reason").Length > 0);
        }

        private static void SetupFailureScenario()
        {
            ScenarioDocument document = ScenarioDocument.Parse(LoadScenarioJson("setup_failure.json"));
            ScenarioResult result = new ScenarioRunner().Run(document);
            JsonNode report = ParseReport(result);

            Check.Equal("Setup failure status is failed_setup", "failed_setup", GetString(report, "status"));
            JsonNode setup = GetNode(report, "setup");
            Check.Equal("Setup failure reports setup as not ok", false, GetBool(setup, "ok"));
            Check.That("Setup failure reports an error", GetString(setup, "error").Length > 0);
        }

        private static void RepeatabilityScenario()
        {
            ScenarioDocument document = ScenarioDocument.Parse(LoadScenarioJson("repeatability.json"));
            ScenarioRunner runner = new ScenarioRunner();
            ScenarioResult first = runner.Run(document, () => new BridgeHost());
            ScenarioResult second = runner.Run(document, () => new BridgeHost());

            bool repeatable = ScenarioRunner.CompareForRepeatability(first, second, out string? difference);
            Check.That("Repeatability comparison reports the runs as repeatable", repeatable);
            Check.That("Repeatability comparison reports no difference", difference == null);

            second.Repeatable = true;
            JsonNode report = ParseReport(second);
            Check.Equal("Repeatability report marks the run repeatable", true, GetBool(report, "repeatable"));
        }

        private static string LoadScenarioJson(string fileName)
        {
            return ResolveScenePaths(File.ReadAllText(ResolveFixture(FixtureDirectory + fileName)));
        }

        private static string ResolveScenePaths(string text)
        {
            string marker = "\"" + FixtureDirectory + SceneFileName + "\"";
            if (text.Contains(marker, StringComparison.Ordinal))
            {
                text = text.Replace(marker, JsonSerializer.Serialize(ResolveFixture(FixtureDirectory + SceneFileName)), StringComparison.Ordinal);
            }

            return text;
        }

        private static JsonNode ParseReport(ScenarioResult result)
        {
            JsonNode? report = JsonNode.Parse(result.ToReportJson());
            return report ?? throw new InvalidOperationException("Scenario report JSON did not parse.");
        }

        private static JsonNode FindCheckpoint(JsonNode report, string id)
        {
            JsonArray checkpoints = GetArray(report, "checkpoints");
            for (int index = 0; index < checkpoints.Count; index++)
            {
                JsonNode? checkpoint = checkpoints[index];
                if (checkpoint != null && GetString(checkpoint, "id") == id)
                {
                    return checkpoint;
                }
            }

            throw new InvalidOperationException("Scenario report has no checkpoint '" + id + "'.");
        }

        private static JsonNode GetNode(JsonNode parent, string property)
        {
            JsonNode? value = parent[property];
            return value ?? throw new InvalidOperationException("Scenario report property '" + property + "' is missing.");
        }

        private static JsonArray GetArray(JsonNode parent, string property)
        {
            JsonNode? value = parent[property];
            return value as JsonArray
                ?? throw new InvalidOperationException("Scenario report property '" + property + "' is not an array.");
        }

        private static string GetString(JsonNode parent, string property)
        {
            JsonNode? value = parent[property];
            return value == null ? string.Empty : GetText(value);
        }

        private static string GetText(JsonNode node)
        {
            return node.GetValue<string>();
        }

        private static bool GetBool(JsonNode parent, string property)
        {
            JsonNode? value = parent[property];
            return value != null && value.GetValue<bool>();
        }

        private static string ResolveFixture(string relativePath)
        {
            return File.Exists(relativePath)
                ? Path.GetFullPath(relativePath)
                : Path.GetFullPath("../../../../../" + relativePath);
        }
    }
}
