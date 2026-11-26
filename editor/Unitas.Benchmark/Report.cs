using System;
using System.Collections.Generic;
using System.Text.Json;
using System.Text.Json.Nodes;

namespace Unitas.Benchmark
{
    public sealed record StepReport(
        string Role,
        string Tool,
        JsonObject Arguments,
        bool Ok,
        string? Error,
        JsonNode? Result);

    public sealed class BenchmarkReport
    {
        public const string RedactedMarker = "[redacted]";

        private readonly IReadOnlyList<StepReport> steps;
        private readonly IReadOnlyList<string> tools;

        public BenchmarkReport(IReadOnlyList<StepReport> steps, IReadOnlyList<string> tools)
        {
            this.steps = steps;
            this.tools = tools;
        }

        public IReadOnlyList<StepReport> Steps => steps;

        public IReadOnlyList<string> Tools => tools;

        public bool AllStepsSucceeded
        {
            get
            {
                foreach (StepReport step in steps)
                {
                    if (!step.Ok)
                    {
                        return false;
                    }
                }

                return true;
            }
        }

        public string ToJson()
        {
            JsonArray stepArray = new JsonArray();
            int succeeded = 0;
            foreach (StepReport step in steps)
            {
                if (step.Ok)
                {
                    succeeded++;
                }

                stepArray.Add(new JsonObject
                {
                    ["role"] = step.Role,
                    ["tool"] = step.Tool,
                    ["arguments"] = step.Arguments.DeepClone(),
                    ["ok"] = step.Ok,
                    ["error"] = step.Error == null ? null : JsonValue.Create(step.Error),
                    ["result"] = step.Result?.DeepClone(),
                });
            }

            JsonArray toolArray = new JsonArray();
            foreach (string tool in tools)
            {
                toolArray.Add(JsonValue.Create(tool));
            }

            JsonObject root = new JsonObject
            {
                ["steps"] = stepArray,
                ["tools"] = toolArray,
                ["summary"] = new JsonObject
                {
                    ["steps"] = steps.Count,
                    ["succeeded"] = succeeded,
                    ["failed"] = steps.Count - succeeded,
                    ["ok"] = AllStepsSucceeded,
                },
            };

            return root.ToJsonString(new JsonSerializerOptions { WriteIndented = true });
        }

        public static JsonNode? Redact(JsonNode? node, string? credential)
        {
            if (node is JsonObject map)
            {
                JsonObject copy = new JsonObject();
                foreach (KeyValuePair<string, JsonNode?> pair in map)
                {
                    if (string.Equals(pair.Key, "personCredential", StringComparison.OrdinalIgnoreCase))
                    {
                        copy[pair.Key] = JsonValue.Create(RedactedMarker);
                        continue;
                    }

                    copy[pair.Key] = Redact(pair.Value, credential);
                }

                return copy;
            }

            if (node is JsonArray array)
            {
                JsonArray copy = new JsonArray();
                foreach (JsonNode? item in array)
                {
                    copy.Add(Redact(item, credential));
                }

                return copy;
            }

            if (node is JsonValue value
                && value.TryGetValue(out string? text)
                && credential != null
                && (string.Equals(text, credential, StringComparison.Ordinal)
                    || string.Equals(text, "$UNITAS_PERSON_CREDENTIAL", StringComparison.Ordinal)))
            {
                return JsonValue.Create(RedactedMarker);
            }

            return node?.DeepClone();
        }
    }
}
