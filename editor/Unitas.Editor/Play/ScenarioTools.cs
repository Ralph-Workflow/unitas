using System;
using System.Collections.Generic;
using System.IO;
using System.Text.Json;
using System.Text.Json.Nodes;
using Unitas.Bridge.Playtest;
using Unitas.Bridge.Scripting;
using UnityEngine;

namespace Unitas.Editor.Mcp
{
    public static class ScenarioTools
    {
        public const string ScenarioRunOrigin = "unitas://scenario/run";

        private const string PlayerGuid = "1a2b3c4d5e6f708192a3b4c5d6e7f809";
        private const string RotatorGuid = "2b3c4d5e6f708192a3b4c5d6e7f8091a";
        private const string ScoreGuid = "3c4d5e6f708192a3b4c5d6e7f8091a2b";
        private const int PathSearchDepth = 8;

        public static void Register(McpHost host)
        {
            ArgumentNullException.ThrowIfNull(host);
            RegisterRollABallGlue();
            host.RegisterTool(
                new ToolDefinition(
                    "scenario.run",
                    "Run a declared scenario document (untrusted project text) and return structured per-step and per-checkpoint results",
                    new JsonObject
                    {
                        ["type"] = "object",
                        ["properties"] = new JsonObject
                        {
                            ["scenarioPath"] = new JsonObject
                            {
                                ["type"] = "string",
                            },
                            ["projectPath"] = new JsonObject
                            {
                                ["type"] = "string",
                            },
                        },
                        ["required"] = new JsonArray("scenarioPath"),
                    },
                    Scope.ExecuteProjectCode),
                arguments => HandleRun(host, arguments));
        }

        private static ToolOutput HandleRun(McpHost host, JsonNode arguments)
        {
            string callerSessionId = RequireCallerSession(host);
            string scenarioPath = RequireScenarioPath(arguments);
            string? projectPath = ExtractOptionalString(arguments, "projectPath");
            string operationId = host.StartOperation(callerSessionId, "scenario.run", Scope.ExecuteProjectCode);
            host.Operations.UpdateStage(operationId, "running");
            System.Threading.SynchronizationContext? previousContext = System.Threading.SynchronizationContext.Current;
            try
            {
                ScenarioDocument document = LoadDocument(scenarioPath, projectPath);
                PurgeStalePlayerBalls();
                ScenarioResult result = new ScenarioRunner().Run(document);
                JsonObject payload = BuildPayload(result);
                host.CompleteOperation(operationId, ScenarioRunOrigin, payload);
                return ToolOutput.Untrusted(ScenarioRunOrigin, payload)
                    .WithOperation(operationId);
            }
            catch (McpError error)
            {
                host.FailOperation(operationId, error.Message);
                throw;
            }
            catch (Exception error)
            {
                host.FailOperation(operationId, error.Message);
                throw new McpError.InvalidRequest(error.Message);
            }
            finally
            {
                System.Threading.SynchronizationContext.SetSynchronizationContext(previousContext);
            }
        }

        private static ScenarioDocument LoadDocument(string scenarioPath, string? projectPath)
        {
            string root = string.IsNullOrWhiteSpace(projectPath)
                ? Directory.GetCurrentDirectory()
                : Path.GetFullPath(projectPath);
            string? resolved = ResolveExistingPath(scenarioPath, root);
            if (resolved == null)
            {
                throw new McpError.InvalidRequest("scenario file does not exist: " + scenarioPath);
            }

            string text = File.ReadAllText(resolved);
            string scenarioDirectory = Path.GetDirectoryName(resolved) ?? root;
            string json = ResolveEmbeddedPaths(text, scenarioDirectory);
            try
            {
                return ScenarioDocument.Parse(json);
            }
            catch (ScenarioFormatException error)
            {
                throw new McpError.InvalidRequest("scenario document is invalid: " + error.Message);
            }
        }

        private static JsonObject BuildPayload(ScenarioResult result)
        {
            JsonObject payload = JsonNode.Parse(result.ToReportJson()) as JsonObject
                ?? new JsonObject();
            bool passed = result.Status == ScenarioStatus.Passed;
            payload["ok"] = passed;
            if (!passed)
            {
                payload["error"] = result.SetupError ?? FirstFailureReason(result) ?? "scenario failed";
            }

            return payload;
        }

        private static string? FirstFailureReason(ScenarioResult result)
        {
            foreach (CheckpointResult checkpoint in result.Checkpoints)
            {
                if (checkpoint.Status != CheckpointStatus.Passed)
                {
                    return checkpoint.Reason ?? ("checkpoint failed: " + checkpoint.Id);
                }
            }

            return null;
        }

        private static string ResolveEmbeddedPaths(string json, string scenarioDirectory)
        {
            JsonNode? node;
            try
            {
                node = JsonNode.Parse(json);
            }
            catch (JsonException)
            {
                return json;
            }

            if (node != null)
            {
                ResolveNode(node, scenarioDirectory);
                return node.ToJsonString();
            }

            return json;
        }

        private static void ResolveNode(JsonNode node, string scenarioDirectory)
        {
            if (node is JsonObject map)
            {
                List<KeyValuePair<string, JsonNode?>> pairs = new List<KeyValuePair<string, JsonNode?>>();
                foreach (KeyValuePair<string, JsonNode?> pair in map)
                {
                    pairs.Add(pair);
                }

                foreach (KeyValuePair<string, JsonNode?> pair in pairs)
                {
                    if (TryResolvePathValue(pair.Value, scenarioDirectory, out JsonValue? resolved))
                    {
                        map[pair.Key] = resolved;
                    }
                    else if (pair.Value != null)
                    {
                        ResolveNode(pair.Value, scenarioDirectory);
                    }
                }

                return;
            }

            if (node is JsonArray array)
            {
                for (int index = 0; index < array.Count; index++)
                {
                    if (TryResolvePathValue(array[index], scenarioDirectory, out JsonValue? resolved))
                    {
                        array[index] = resolved;
                    }
                    else if (array[index] != null)
                    {
                        ResolveNode(array[index]!, scenarioDirectory);
                    }
                }
            }
        }

        private static bool TryResolvePathValue(JsonNode? node, string scenarioDirectory, out JsonValue? resolved)
        {
            resolved = null;
            if (!(node is JsonValue value) || !value.TryGetValue(out string? text))
            {
                return false;
            }

            if (string.IsNullOrWhiteSpace(text)
                || (text.IndexOf('/', StringComparison.Ordinal) < 0 && text.IndexOf('\\', StringComparison.Ordinal) < 0)
                || Path.IsPathRooted(text))
            {
                return false;
            }

            string? fullPath = ResolveExistingPath(text, scenarioDirectory);
            if (fullPath == null)
            {
                return false;
            }

            resolved = JsonValue.Create(fullPath);
            return true;
        }

        private static string? ResolveExistingPath(string path, string root)
        {
            string candidate = Path.GetFullPath(Path.Combine(root, path));
            if (File.Exists(candidate))
            {
                return candidate;
            }

            DirectoryInfo? directory = new DirectoryInfo(root);
            for (int depth = 0; depth < PathSearchDepth && directory != null; depth++)
            {
                candidate = Path.GetFullPath(Path.Combine(directory.FullName, path));
                if (File.Exists(candidate))
                {
                    return candidate;
                }

                directory = directory.Parent;
            }

            return null;
        }

        private static void PurgeStalePlayerBalls()
        {
            for (int guard = 0; guard < 100; guard++)
            {
                GameObject? stale = GameObject.Find("PlayerBall");
                if (stale == null)
                {
                    return;
                }

                UnityEngine.Object.Destroy(stale);
            }
        }

        private static string RequireCallerSession(McpHost host)
        {
            string? sessionId = host.CurrentSessionId;
            if (string.IsNullOrEmpty(sessionId))
            {
                throw new McpError.InvalidRequest("tool invocation requires an active MCP session");
            }

            return sessionId!;
        }

        private static string RequireScenarioPath(JsonNode arguments)
        {
            string? scenarioPath = ExtractOptionalString(arguments, "scenarioPath");
            if (scenarioPath == null)
            {
                throw new McpError.InvalidParams("scenario.run requires a 'scenarioPath' argument");
            }

            return scenarioPath;
        }

        private static string? ExtractOptionalString(JsonNode arguments, string propertyName)
        {
            if (arguments is JsonObject obj
                && obj[propertyName] is JsonValue value
                && value.TryGetValue(out string? text)
                && !string.IsNullOrWhiteSpace(text))
            {
                return text;
            }

            return null;
        }

        private static void RegisterRollABallGlue()
        {
            ScriptGlueRegistry.Register(PlayerGuid, () => new RollABallPlayerScript());
            ScriptGlueRegistry.Register(RotatorGuid, () => new RollABallPickupScript());
            ScriptGlueRegistry.Register(ScoreGuid, () => new RollABallScoreScript());
        }

        private static class RollABallGameState
        {
            internal const int TotalPickups = 4;
            internal const string WinMessage = "All 4 pickups collected, press R to play again.";

            internal static readonly HashSet<string> Collected =
                new HashSet<string>(StringComparer.Ordinal);

            internal static int Score { get; private set; }

            internal static bool Won { get; private set; }

            internal static void Reset()
            {
                Score = 0;
                Won = false;
                Collected.Clear();
            }

            internal static void Collect(string pickupName)
            {
                if (Won || !Collected.Add(pickupName))
                {
                    return;
                }

                Score++;
                if (Score >= TotalPickups)
                {
                    Won = true;
                    Debug.Log(WinMessage);
                }
            }
        }

        private sealed class RollABallPlayerScript : MonoBehaviour
        {
            private const float MoveSpeed = 7.5f;
            private static readonly Vector3 StartPosition = new Vector3(0f, 0.5f, 0f);

            private Rigidbody? _body;

            public override void Awake()
            {
                _body = GetComponent<Rigidbody>();
                RollABallGameState.Reset();
            }

            public override void Update()
            {
                Rigidbody? body = _body;
                if (body == null)
                {
                    return;
                }

                float moveX = ReadAxis(KeyCode.D, KeyCode.RightArrow, KeyCode.A, KeyCode.LeftArrow);
                float moveZ = ReadAxis(KeyCode.W, KeyCode.UpArrow, KeyCode.S, KeyCode.DownArrow);
                body.velocity = new Vector3(moveX * MoveSpeed, body.velocity.y, moveZ * MoveSpeed);
                if (Input.GetKeyDown(KeyCode.R))
                {
                    Restart();
                }
            }

            private void Restart()
            {
                RollABallGameState.Reset();
                Rigidbody? body = _body;
                IPhysicsProvider? provider = Physics.Provider;
                if (body == null || provider == null)
                {
                    return;
                }

                if (!provider.TryGetBodyPosition(gameObject.NativeEntityId, out Vector3 current))
                {
                    current = transform.position;
                }

                float dt = Time.deltaTime > 0f ? Time.deltaTime : 0.02f;
                body.velocity = (StartPosition - current) / dt;
            }

            private static float ReadAxis(KeyCode positive, KeyCode positiveAlt, KeyCode negative, KeyCode negativeAlt)
            {
                float value = 0f;
                if (Input.GetKey(positive) || Input.GetKey(positiveAlt))
                {
                    value = Mathf.Max(value, 1f);
                }

                if (Input.GetKey(negative) || Input.GetKey(negativeAlt))
                {
                    value = Mathf.Min(value, -1f);
                }

                return value;
            }
        }

        private sealed class RollABallPickupScript : MonoBehaviour
        {
            public override void OnTriggerEnter(Collider? other)
            {
                if (other == null || !string.Equals(other.gameObject.name, "PlayerBall", StringComparison.Ordinal))
                {
                    return;
                }

                RollABallGameState.Collect(gameObject.name);
            }
        }

        private sealed class RollABallScoreScript : MonoBehaviour
        {
        }
    }
}
