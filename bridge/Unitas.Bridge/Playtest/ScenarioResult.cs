using System.Collections.Generic;
using System.Text.Json;
using System.Text.Json.Nodes;
using System.Text.Json.Serialization.Metadata;

namespace Unitas.Bridge.Playtest
{
    public enum CheckpointStatus
    {
        Passed,
        Failed,
        Unevaluable,
    }

    public enum ScenarioStatus
    {
        Passed,
        Failed,
        FailedSetup,
    }

    public sealed class ScenarioStepRecord
    {
        public ScenarioStepRecord(int index, string kind, int? frames, int frameStart, int frameEnd)
        {
            Index = index;
            Kind = kind;
            Frames = frames;
            FrameStart = frameStart;
            FrameEnd = frameEnd;
        }

        public int Index { get; }

        public string Kind { get; }

        public int? Frames { get; }

        public int FrameStart { get; }

        public int FrameEnd { get; }
    }

    public sealed class CheckpointResult
    {
        public CheckpointResult(string id, CheckpointStatus status, string? expected, string? observed, string? reason)
        {
            Id = id;
            Status = status;
            Expected = expected;
            Observed = observed;
            Reason = reason;
        }

        public string Id { get; }

        public CheckpointStatus Status { get; }

        public string? Expected { get; }

        public string? Observed { get; }

        public string? Reason { get; }
    }

    public sealed class ScenarioMessage
    {
        public ScenarioMessage(string severity, int frame, string message)
        {
            Severity = severity;
            Frame = frame;
            Message = message;
        }

        public string Severity { get; }

        public int Frame { get; }

        public string Message { get; }
    }

    public sealed class ScenarioResult
    {
        private static readonly JsonSerializerOptions ReportOptions = new JsonSerializerOptions
        {
            WriteIndented = true,
            TypeInfoResolver = new DefaultJsonTypeInfoResolver(),
        };

        public ScenarioResult(
            string scenario,
            ScenarioStatus status,
            bool setupOk,
            string? setupScene,
            string? setupError,
            IReadOnlyList<ScenarioStepRecord> steps,
            IReadOnlyList<CheckpointResult> checkpoints,
            IReadOnlyList<ScenarioMessage> messages,
            IReadOnlyList<string> captures,
            bool? repeatable)
        {
            Scenario = scenario;
            Status = status;
            SetupOk = setupOk;
            SetupScene = setupScene;
            SetupError = setupError;
            Steps = steps;
            Checkpoints = checkpoints;
            Messages = messages;
            Captures = captures;
            Repeatable = repeatable;
        }

        public string Scenario { get; }

        public ScenarioStatus Status { get; }

        public bool SetupOk { get; }

        public string? SetupScene { get; }

        public string? SetupError { get; }

        public IReadOnlyList<ScenarioStepRecord> Steps { get; }

        public IReadOnlyList<CheckpointResult> Checkpoints { get; }

        public IReadOnlyList<ScenarioMessage> Messages { get; }

        public IReadOnlyList<string> Captures { get; }

        public bool? Repeatable { get; set; }

        public string ToReportJson()
        {
            JsonObject root = new JsonObject();
            root["scenario"] = Scenario;
            root["status"] = StatusName(Status);

            JsonObject setup = new JsonObject();
            setup["ok"] = SetupOk;
            setup["scene"] = SetupScene;
            setup["error"] = SetupError;
            root["setup"] = setup;

            JsonArray steps = new JsonArray();
            foreach (ScenarioStepRecord record in Steps)
            {
                JsonObject step = new JsonObject();
                step["index"] = record.Index;
                step["kind"] = record.Kind;
                if (record.Frames.HasValue)
                {
                    step["frames"] = record.Frames.Value;
                }

                step["frame_start"] = record.FrameStart;
                step["frame_end"] = record.FrameEnd;
                steps.Add(step);
            }

            root["steps"] = steps;

            JsonArray checkpoints = new JsonArray();
            foreach (CheckpointResult checkpoint in Checkpoints)
            {
                JsonObject entry = new JsonObject();
                entry["id"] = checkpoint.Id;
                entry["status"] = StatusName(checkpoint.Status);
                entry["expected"] = checkpoint.Expected;
                entry["observed"] = checkpoint.Observed;
                entry["reason"] = checkpoint.Reason;
                checkpoints.Add(entry);
            }

            root["checkpoints"] = checkpoints;

            JsonArray messages = new JsonArray();
            foreach (ScenarioMessage message in Messages)
            {
                JsonObject entry = new JsonObject();
                entry["severity"] = message.Severity;
                entry["frame"] = message.Frame;
                entry["message"] = message.Message;
                messages.Add(entry);
            }

            root["messages"] = messages;

            JsonArray captures = new JsonArray();
            foreach (string capture in Captures)
            {
                captures.Add(JsonValue.Create(capture));
            }

            root["captures"] = captures;

            root["repeatable"] = Repeatable.HasValue ? Repeatable.Value : (bool?)null;

            return root.ToJsonString(ReportOptions);
        }

        private static string StatusName(ScenarioStatus status)
        {
            switch (status)
            {
                case ScenarioStatus.Passed:
                    return "passed";
                case ScenarioStatus.Failed:
                    return "failed";
                default:
                    return "failed_setup";
            }
        }

        private static string StatusName(CheckpointStatus status)
        {
            switch (status)
            {
                case CheckpointStatus.Passed:
                    return "passed";
                case CheckpointStatus.Failed:
                    return "failed";
                default:
                    return "unevaluable";
            }
        }
    }
}
