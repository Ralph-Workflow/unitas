using System;
using System.Collections.Generic;
using System.IO;
using System.Text.Json;
using System.Text.Json.Nodes;
using Unitas.Bridge.Playtest;
using Unitas.Bridge.Scripting;
using UnityEngine;

namespace Unitas.Bridge.Tests
{
    internal static class RollABallGameState
    {
        internal const int TotalPickups = 4;
        internal const string WinMessage = "All 4 pickups collected, press R to play again.";

        internal static readonly HashSet<string> Collected = new HashSet<string>(StringComparer.Ordinal);

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

    public sealed class RollABallPlayerScript : MonoBehaviour
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

    public sealed class RollABallPickupScript : MonoBehaviour
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

    public sealed class RollABallScoreScript : MonoBehaviour
    {
    }

    public static class RollABallScenarioTests
    {
        private const string ScenarioDirectory = "samples/roll-a-ball/Scenarios/";
        private const string SceneRelativePath = "samples/roll-a-ball/Assets/Scenes/RollABall.unity";
        private const string PlayerGuid = "1a2b3c4d5e6f708192a3b4c5d6e7f809";
        private const string RotatorGuid = "2b3c4d5e6f708192a3b4c5d6e7f8091a";
        private const string ScoreGuid = "3c4d5e6f708192a3b4c5d6e7f8091a2b";

        public static void Run()
        {
            ScriptGlueRegistry.Clear();
            ScriptGlueRegistry.Register(PlayerGuid, () => new RollABallPlayerScript());
            ScriptGlueRegistry.Register(RotatorGuid, () => new RollABallPickupScript());
            ScriptGlueRegistry.Register(ScoreGuid, () => new RollABallScoreScript());
            try
            {
                CollectAllPickupsScenario();
                RestartRecoveryScenario();
            }
            finally
            {
                PurgeStalePlayerBalls();
                ScriptGlueRegistry.Clear();
            }
        }

        private static void CollectAllPickupsScenario()
        {
            ScenarioResult result = RunScenario("collect_all_pickups.scenario.json");

            Check.Equal("Collect-all-pickups status is passed", "passed", result.Status.ToString().ToLowerInvariant());
            Check.That("Collect-all-pickups setup succeeded", result.SetupOk);
            AssertAllCheckpointsPassed(result);
        }

        private static void RestartRecoveryScenario()
        {
            ScenarioResult result = RunScenario("restart_recovery.scenario.json");

            Check.Equal("Restart-recovery status is passed", "passed", result.Status.ToString().ToLowerInvariant());
            Check.That("Restart-recovery setup succeeded", result.SetupOk);
            AssertAllCheckpointsPassed(result);
        }

        private static ScenarioResult RunScenario(string fileName)
        {
            // GameObject.Find returns the first live match process-wide, so objects from
            // an earlier scenario run would shadow this run's scene objects; purge them.
            PurgeStalePlayerBalls();
            ScenarioDocument document = ScenarioDocument.Parse(LoadScenarioJson(ScenarioDirectory + fileName));
            return new ScenarioRunner().Run(document);
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

        private static void AssertAllCheckpointsPassed(ScenarioResult result)
        {
            JsonNode report = ParseReport(result);
            JsonArray checkpoints = report["checkpoints"] as JsonArray
                ?? throw new InvalidOperationException("Scenario report has no checkpoints array.");
            Check.That(result.Scenario + " reports at least one checkpoint", checkpoints.Count > 0);
            for (int index = 0; index < checkpoints.Count; index++)
            {
                JsonNode checkpoint = checkpoints[index]!;
                string id = checkpoint["id"]?.GetValue<string>() ?? string.Empty;
                string status = checkpoint["status"]?.GetValue<string>() ?? string.Empty;
                if (status != "passed")
                {
                    Console.WriteLine(
                        "DIAG " + result.Scenario + "/" + id + " status=" + status
                        + " expected=" + (checkpoint["expected"]?.ToJsonString() ?? "<none>")
                        + " observed=" + (checkpoint["observed"]?.ToJsonString() ?? "<none>")
                        + " reason=" + (checkpoint["reason"]?.ToJsonString() ?? "<none>"));
                }

                Check.Equal(result.Scenario + " checkpoint " + id + " passed", "passed", status);
            }
        }

        private static string LoadScenarioJson(string relativePath)
        {
            string text = File.ReadAllText(ResolveFixture(relativePath));
            string sceneFullPath = JsonSerializer.Serialize(ResolveFixture(SceneRelativePath));
            text = text.Replace(JsonSerializer.Serialize(SceneRelativePath), sceneFullPath, StringComparison.Ordinal);
            return text;
        }

        private static JsonNode ParseReport(ScenarioResult result)
        {
            JsonNode? report = JsonNode.Parse(result.ToReportJson());
            return report ?? throw new InvalidOperationException("Scenario report JSON did not parse.");
        }

        private static string ResolveFixture(string relativePath)
        {
            return File.Exists(relativePath)
                ? Path.GetFullPath(relativePath)
                : Path.GetFullPath("../../../../../" + relativePath);
        }
    }
}
