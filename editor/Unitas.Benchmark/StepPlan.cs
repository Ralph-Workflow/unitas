using System;
using System.Collections.Generic;
using System.IO;
using System.Text.Json;
using System.Text.Json.Nodes;

namespace Unitas.Benchmark
{
    public sealed record Step(string Role, string Tool, JsonObject Arguments);

    public sealed class StepPlan
    {
        private StepPlan(IReadOnlyList<Step> steps)
        {
            Steps = steps;
        }

        public IReadOnlyList<Step> Steps { get; }

        public static StepPlan Load(string path)
        {
            string text;
            try
            {
                text = File.ReadAllText(path);
            }
            catch (Exception error) when (error is IOException || error is UnauthorizedAccessException)
            {
                throw new UsageException("the step plan could not be read: " + error.Message);
            }

            JsonNode? root;
            try
            {
                root = JsonNode.Parse(text);
            }
            catch (JsonException error)
            {
                throw new UsageException("the step plan is not valid JSON: " + error.Message);
            }

            JsonArray? steps = root switch
            {
                JsonArray array => array,
                JsonObject map => map["steps"] as JsonArray,
                _ => null,
            };
            if (steps == null)
            {
                throw new UsageException(
                    "the step plan must be a JSON array of steps or an object with a 'steps' array");
            }

            List<Step> parsed = new List<Step>();
            int index = 0;
            foreach (JsonNode? node in steps)
            {
                index++;
                if (node is not JsonObject step)
                {
                    throw new UsageException("step " + index + " is not a JSON object");
                }

                parsed.Add(new Step(
                    RequireText(step, "role", index),
                    RequireText(step, "tool", index),
                    step["arguments"] as JsonObject ?? new JsonObject()));
            }

            return new StepPlan(parsed);
        }

        private static string RequireText(JsonObject step, string name, int index)
        {
            if (step[name] is JsonValue value
                && value.TryGetValue(out string? text)
                && !string.IsNullOrWhiteSpace(text))
            {
                return text;
            }

            throw new UsageException("step " + index + " needs a non-empty '" + name + "'");
        }
    }
}
