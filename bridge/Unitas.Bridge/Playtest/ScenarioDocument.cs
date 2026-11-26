using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Text.Json;
using UnityEngine;
using UnityEngine.SceneManagement;

namespace Unitas.Bridge.Playtest
{
    public sealed class ScenarioDocument
    {
        private const double DefaultDeltaSeconds = 0.02;
        private const double DefaultTolerance = 0.001;
        private const int DefaultCaptureSize = 256;
        private const string AnySeverity = "any";
        private const string LogSeverityName = "log";

        private static readonly HashSet<string> StepKindNames = new HashSet<string>(StringComparer.Ordinal)
        {
            "load_scene",
            "tick",
            "inject_key",
            "inject_mouse_button",
            "inject_pointer",
            "inject_scroll",
            "set_time_scale",
            "inject_gamepad_axis",
        };

        public ScenarioDocument(
            string name,
            string scene,
            IReadOnlyList<ScenarioStep> steps,
            IReadOnlyList<ScenarioCheckpoint> checkpoints)
        {
            Name = name;
            Scene = scene;
            Steps = steps;
            Checkpoints = checkpoints;
        }

        public string Name { get; }

        public string Scene { get; }

        public IReadOnlyList<ScenarioStep> Steps { get; }

        public IReadOnlyList<ScenarioCheckpoint> Checkpoints { get; }

        public static ScenarioDocument Parse(string json, string? baseDirectory = null)
        {
            ArgumentNullException.ThrowIfNull(json);
            string referenceBase = string.IsNullOrEmpty(baseDirectory) ? Directory.GetCurrentDirectory() : Path.GetFullPath(baseDirectory);

            JsonDocument document;
            try
            {
                document = JsonDocument.Parse(json);
            }
            catch (JsonException exception)
            {
                throw Fail("$", "invalid JSON document (" + exception.Message + ")");
            }

            using (document)
            {
                JsonElement root = document.RootElement;
                if (root.ValueKind != JsonValueKind.Object)
                {
                    throw Fail("$", "document root must be a JSON object");
                }

                string name = ReadRequiredString(root, "name", "name");
                string scene = ReadOptionalString(root, "scene", "scene") ?? string.Empty;

                List<ScenarioStep> steps = new List<ScenarioStep>();
                bool hasLoadSceneStep = false;
                if (root.TryGetProperty("steps", out JsonElement stepsElement))
                {
                    if (stepsElement.ValueKind != JsonValueKind.Array)
                    {
                        throw Fail("steps", "'steps' must be an array");
                    }

                    int index = 0;
                    foreach (JsonElement stepElement in stepsElement.EnumerateArray())
                    {
                        string path = "steps[" + index.ToString(CultureInfo.InvariantCulture) + "]";
                        ScenarioStep step = ParseStep(stepElement, path);
                        if (step.Kind == ScenarioStepKind.LoadScene)
                        {
                            hasLoadSceneStep = true;
                        }

                        steps.Add(step);
                        index++;
                    }
                }

                if (scene.Length == 0 && !hasLoadSceneStep)
                {
                    throw Fail("scene", "scenario requires a non-empty 'scene' or a 'load_scene' step");
                }

                List<ScenarioCheckpoint> checkpoints = new List<ScenarioCheckpoint>();
                if (root.TryGetProperty("checkpoints", out JsonElement checkpointsElement))
                {
                    if (checkpointsElement.ValueKind != JsonValueKind.Array)
                    {
                        throw Fail("checkpoints", "'checkpoints' must be an array");
                    }

                    HashSet<string> ids = new HashSet<string>(StringComparer.Ordinal);
                    int index = 0;
                    foreach (JsonElement checkpointElement in checkpointsElement.EnumerateArray())
                    {
                        string path = "checkpoints[" + index.ToString(CultureInfo.InvariantCulture) + "]";
                        checkpoints.Add(ParseCheckpoint(checkpointElement, path, ids, referenceBase));
                        index++;
                    }
                }

                return new ScenarioDocument(name, scene, steps.ToArray(), checkpoints.ToArray());
            }
        }

        private static ScenarioStep ParseStep(JsonElement element, string path)
        {
            if (element.ValueKind != JsonValueKind.Object)
            {
                throw Fail(path, "step must be a JSON object");
            }

            string kind = string.Empty;
            int count = 0;
            foreach (JsonProperty property in element.EnumerateObject())
            {
                kind = property.Name;
                count++;
            }

            if (count != 1)
            {
                throw Fail(path, "step must name exactly one step kind");
            }

            string kindPath = path + "." + kind;
            if (!StepKindNames.Contains(kind))
            {
                throw Fail(kindPath, "unknown step kind '" + kind + "'");
            }

            JsonElement body = element.GetProperty(kind);
            if (body.ValueKind != JsonValueKind.Object)
            {
                throw Fail(kindPath, "step '" + kind + "' must be a JSON object");
            }

            switch (kind)
            {
                case "load_scene":
                    return ParseLoadSceneStep(body, kindPath);
                case "tick":
                    return ParseTickStep(body, kindPath);
                case "inject_key":
                    return ParseInjectKeyStep(body, kindPath);
                case "inject_mouse_button":
                    return ParseInjectMouseButtonStep(body, kindPath);
                case "inject_pointer":
                    return ParseInjectPointerStep(body, kindPath);
                case "inject_scroll":
                    return ParseInjectScrollStep(body, kindPath);
                case "set_time_scale":
                    return ParseSetTimeScaleStep(body, kindPath);
                case "inject_gamepad_axis":
                    return ParseInjectGamepadAxisStep(body, kindPath);
                default:
                    throw Fail(kindPath, "unknown step kind '" + kind + "'");
            }
        }

        private static LoadSceneStep ParseLoadSceneStep(JsonElement body, string path)
        {
            RejectUnknownProperties(body, path, "path", "mode");
            string scenePath = ReadRequiredString(body, "path", path + ".path");
            LoadSceneMode mode = LoadSceneMode.Single;
            if (body.TryGetProperty("mode", out JsonElement modeElement))
            {
                if (modeElement.ValueKind != JsonValueKind.String)
                {
                    throw Fail(path + ".mode", "'mode' must be a string");
                }

                string modeText = modeElement.GetString() ?? string.Empty;
                if (string.Equals(modeText, "single", StringComparison.OrdinalIgnoreCase))
                {
                    mode = LoadSceneMode.Single;
                }
                else if (string.Equals(modeText, "additive", StringComparison.OrdinalIgnoreCase))
                {
                    mode = LoadSceneMode.Additive;
                }
                else
                {
                    throw Fail(path + ".mode", "'mode' must be 'single' or 'additive'");
                }
            }

            return new LoadSceneStep(scenePath, mode);
        }

        private static TickStep ParseTickStep(JsonElement body, string path)
        {
            RejectUnknownProperties(body, path, "frames", "delta_seconds");
            int frames = ReadOptionalInt(body, "frames", path + ".frames", 1);
            if (frames < 0)
            {
                throw Fail(path + ".frames", "'frames' must be >= 0");
            }

            double deltaSeconds = ReadOptionalNumber(body, "delta_seconds", path + ".delta_seconds", DefaultDeltaSeconds);
            if (deltaSeconds <= 0)
            {
                throw Fail(path + ".delta_seconds", "'delta_seconds' must be > 0");
            }

            return new TickStep(frames, deltaSeconds);
        }

        private static InjectKeyStep ParseInjectKeyStep(JsonElement body, string path)
        {
            RejectUnknownProperties(body, path, "key", "pressed");
            string key = ReadRequiredString(body, "key", path + ".key");
            if (!Enum.TryParse(key, true, out KeyCode _))
            {
                throw Fail(path + ".key", "unknown key '" + key + "'");
            }

            bool pressed = ReadOptionalBool(body, "pressed", path + ".pressed", false);
            return new InjectKeyStep(key, pressed);
        }

        private static InjectMouseButtonStep ParseInjectMouseButtonStep(JsonElement body, string path)
        {
            RejectUnknownProperties(body, path, "button", "pressed");
            int button = ReadRequiredInt(body, "button", path + ".button");
            if (button < 0)
            {
                throw Fail(path + ".button", "'button' must be >= 0");
            }

            bool pressed = ReadOptionalBool(body, "pressed", path + ".pressed", false);
            return new InjectMouseButtonStep(button, pressed);
        }

        private static InjectPointerStep ParseInjectPointerStep(JsonElement body, string path)
        {
            RejectUnknownProperties(body, path, "x", "y");
            double x = ReadRequiredNumber(body, "x", path + ".x");
            double y = ReadRequiredNumber(body, "y", path + ".y");
            return new InjectPointerStep(x, y);
        }

        private static InjectScrollStep ParseInjectScrollStep(JsonElement body, string path)
        {
            RejectUnknownProperties(body, path, "x", "y");
            double x = ReadOptionalNumber(body, "x", path + ".x", 0.0);
            double y = ReadOptionalNumber(body, "y", path + ".y", 0.0);
            return new InjectScrollStep(x, y);
        }

        private static SetTimeScaleStep ParseSetTimeScaleStep(JsonElement body, string path)
        {
            RejectUnknownProperties(body, path, "value");
            double value = ReadRequiredNumber(body, "value", path + ".value");
            if (value <= 0)
            {
                throw Fail(path + ".value", "'value' must be > 0");
            }

            return new SetTimeScaleStep(value);
        }

        private static InjectGamepadAxisStep ParseInjectGamepadAxisStep(JsonElement body, string path)
        {
            RejectUnknownProperties(body, path, "axis", "value");
            string axis = ReadRequiredString(body, "axis", path + ".axis");
            double value = ReadRequiredNumber(body, "value", path + ".value");
            if (value < -1.0 || value > 1.0)
            {
                throw Fail(path + ".value", "'value' must be within [-1, 1]");
            }

            return new InjectGamepadAxisStep(axis, value);
        }

        private static ScenarioCheckpoint ParseCheckpoint(JsonElement element, string path, HashSet<string> ids, string referenceBase)
        {
            if (element.ValueKind != JsonValueKind.Object)
            {
                throw Fail(path, "checkpoint must be a JSON object");
            }

            string id = ReadRequiredString(element, "id", path + ".id");
            if (!ids.Add(id))
            {
                throw Fail(path + ".id", "duplicate checkpoint id '" + id + "'");
            }

            int afterStep = ReadOptionalInt(element, "after_step", path + ".after_step", 0);
            if (afterStep < 0)
            {
                throw Fail(path + ".after_step", "'after_step' must be >= 0");
            }

            string kind = ReadRequiredString(element, "kind", path + ".kind");
            switch (kind)
            {
                case "transform_position":
                    return ParseTransformPositionCheckpoint(element, path, id, afterStep);
                case "log":
                    return ParseLogCheckpoint(element, path, id, afterStep);
                case "frame_count":
                    return ParseFrameCountCheckpoint(element, path, id, afterStep);
                case "render_capture":
                    return ParseRenderCaptureCheckpoint(element, path, id, afterStep, referenceBase);
                default:
                    throw Fail(path + ".kind", "unknown checkpoint kind '" + kind + "'");
            }
        }

        private static ScenarioCheckpoint ParseTransformPositionCheckpoint(
            JsonElement element,
            string path,
            string id,
            int afterStep)
        {
            RejectUnknownProperties(element, path, "id", "after_step", "kind", "object", "expected", "tolerance");
            string objectName = ReadRequiredString(element, "object", path + ".object");
            double[] expected = ReadRequiredVector(element, "expected", path + ".expected");
            double tolerance = ReadOptionalNumber(element, "tolerance", path + ".tolerance", DefaultTolerance);
            if (tolerance < 0)
            {
                throw Fail(path + ".tolerance", "'tolerance' must be >= 0");
            }

            return new ScenarioCheckpoint(
                id,
                afterStep,
                CheckpointKind.TransformPosition,
                objectName,
                expected,
                tolerance,
                null,
                null,
                null,
                null,
                0,
                0,
                null);
        }

        private static ScenarioCheckpoint ParseLogCheckpoint(JsonElement element, string path, string id, int afterStep)
        {
            RejectUnknownProperties(element, path, "id", "after_step", "kind", "severity", "contains");
            string rawSeverity = ReadOptionalString(element, "severity", path + ".severity") ?? AnySeverity;
            string severity = rawSeverity.ToLowerInvariant();
            if (string.Equals(severity, "info", StringComparison.Ordinal))
            {
                severity = LogSeverityName;
            }

            if (!IsKnownSeverity(severity))
            {
                throw Fail(path + ".severity", "unknown severity '" + rawSeverity + "'");
            }

            string contains = ReadOptionalString(element, "contains", path + ".contains") ?? string.Empty;
            return new ScenarioCheckpoint(
                id,
                afterStep,
                CheckpointKind.Log,
                null,
                null,
                DefaultTolerance,
                severity,
                contains,
                null,
                null,
                0,
                0,
                null);
        }

        private static ScenarioCheckpoint ParseFrameCountCheckpoint(JsonElement element, string path, string id, int afterStep)
        {
            RejectUnknownProperties(element, path, "id", "after_step", "kind", "expected");
            int expectedFrameCount = ReadRequiredInt(element, "expected", path + ".expected");
            if (expectedFrameCount < 0)
            {
                throw Fail(path + ".expected", "'expected' must be >= 0");
            }

            return new ScenarioCheckpoint(
                id,
                afterStep,
                CheckpointKind.FrameCount,
                null,
                null,
                DefaultTolerance,
                null,
                null,
                expectedFrameCount,
                null,
                0,
                0,
                null);
        }

        private static ScenarioCheckpoint ParseRenderCaptureCheckpoint(
            JsonElement element,
            string path,
            string id,
            int afterStep,
            string referenceBase)
        {
            RejectUnknownProperties(
                element,
                path,
                "id",
                "after_step",
                "kind",
                "camera",
                "width",
                "height",
                "expected_non_background_pixels",
                "reference",
                "tolerance");
            string cameraName = ReadRequiredString(element, "camera", path + ".camera");
            int width = ReadOptionalInt(element, "width", path + ".width", DefaultCaptureSize);
            if (width <= 0)
            {
                throw Fail(path + ".width", "'width' must be > 0");
            }

            int height = ReadOptionalInt(element, "height", path + ".height", DefaultCaptureSize);
            if (height <= 0)
            {
                throw Fail(path + ".height", "'height' must be > 0");
            }

            int? expectedNonBackgroundPixels = null;
            if (element.TryGetProperty("expected_non_background_pixels", out JsonElement pixelsElement))
            {
                expectedNonBackgroundPixels = ReadInt(pixelsElement, "expected_non_background_pixels", path + ".expected_non_background_pixels");
                if (expectedNonBackgroundPixels.Value < 0)
                {
                    throw Fail(path + ".expected_non_background_pixels", "'expected_non_background_pixels' must be >= 0");
                }
            }

            string? reference = ReadOptionalString(element, "reference", path + ".reference");
            if (reference != null && reference.Length == 0)
            {
                throw Fail(path + ".reference", "'reference' must not be empty");
            }

            double tolerance = DefaultTolerance;
            if (element.TryGetProperty("tolerance", out JsonElement toleranceElement))
            {
                if (reference == null)
                {
                    throw Fail(path + ".tolerance", "'tolerance' requires 'reference'");
                }

                tolerance = ReadNumber(toleranceElement, "tolerance", path + ".tolerance");
                if (tolerance < 0.0 || tolerance > 1.0)
                {
                    throw Fail(path + ".tolerance", "'tolerance' must be between 0 and 1");
                }
            }
            else if (reference != null)
            {
                tolerance = 0.0;
            }

            string? referencePath = reference == null ? null : ResolveReferencePath(reference, referenceBase);

            return new ScenarioCheckpoint(
                id,
                afterStep,
                CheckpointKind.RenderCapture,
                null,
                null,
                tolerance,
                null,
                null,
                null,
                cameraName,
                width,
                height,
                expectedNonBackgroundPixels,
                referencePath);
        }

        private static string ResolveReferencePath(string reference, string referenceBase)
        {
            if (Path.IsPathRooted(reference))
            {
                return Path.GetFullPath(reference);
            }

            return Path.GetFullPath(Path.Combine(referenceBase, reference));
        }

        private static bool IsKnownSeverity(string severity)
        {
            return string.Equals(severity, AnySeverity, StringComparison.Ordinal)
                || string.Equals(severity, LogSeverityName, StringComparison.Ordinal)
                || string.Equals(severity, "warning", StringComparison.Ordinal)
                || string.Equals(severity, "error", StringComparison.Ordinal);
        }

        private static void RejectUnknownProperties(JsonElement element, string path, params string[] allowed)
        {
            foreach (JsonProperty property in element.EnumerateObject())
            {
                if (Array.IndexOf(allowed, property.Name) < 0)
                {
                    throw Fail(path + "." + property.Name, "unknown property '" + property.Name + "'");
                }
            }
        }

        private static string ReadRequiredString(JsonElement parent, string property, string path)
        {
            if (!parent.TryGetProperty(property, out JsonElement value))
            {
                throw Fail(path, "missing required property '" + property + "'");
            }

            string text = ReadString(value, property, path);
            if (text.Length == 0)
            {
                throw Fail(path, "'" + property + "' must not be empty");
            }

            return text;
        }

        private static string? ReadOptionalString(JsonElement parent, string property, string path)
        {
            if (!parent.TryGetProperty(property, out JsonElement value))
            {
                return null;
            }

            return ReadString(value, property, path);
        }

        private static string ReadString(JsonElement value, string property, string path)
        {
            if (value.ValueKind != JsonValueKind.String)
            {
                throw Fail(path, "'" + property + "' must be a string");
            }

            return value.GetString() ?? string.Empty;
        }

        private static int ReadRequiredInt(JsonElement parent, string property, string path)
        {
            if (!parent.TryGetProperty(property, out JsonElement value))
            {
                throw Fail(path, "missing required property '" + property + "'");
            }

            return ReadInt(value, property, path);
        }

        private static int ReadOptionalInt(JsonElement parent, string property, string path, int fallback)
        {
            if (!parent.TryGetProperty(property, out JsonElement value))
            {
                return fallback;
            }

            return ReadInt(value, property, path);
        }

        private static int ReadInt(JsonElement value, string property, string path)
        {
            if (value.ValueKind != JsonValueKind.Number || !value.TryGetInt32(out int number))
            {
                throw Fail(path, "'" + property + "' must be an integer");
            }

            return number;
        }

        private static double ReadRequiredNumber(JsonElement parent, string property, string path)
        {
            if (!parent.TryGetProperty(property, out JsonElement value))
            {
                throw Fail(path, "missing required property '" + property + "'");
            }

            return ReadNumber(value, property, path);
        }

        private static double ReadOptionalNumber(JsonElement parent, string property, string path, double fallback)
        {
            if (!parent.TryGetProperty(property, out JsonElement value))
            {
                return fallback;
            }

            return ReadNumber(value, property, path);
        }

        private static double ReadNumber(JsonElement value, string property, string path)
        {
            if (value.ValueKind != JsonValueKind.Number || !value.TryGetDouble(out double number))
            {
                throw Fail(path, "'" + property + "' must be a number");
            }

            return number;
        }

        private static bool ReadOptionalBool(JsonElement parent, string property, string path, bool fallback)
        {
            if (!parent.TryGetProperty(property, out JsonElement value))
            {
                return fallback;
            }

            if (value.ValueKind != JsonValueKind.True && value.ValueKind != JsonValueKind.False)
            {
                throw Fail(path, "'" + property + "' must be a boolean");
            }

            return value.GetBoolean();
        }

        private static double[] ReadRequiredVector(JsonElement parent, string property, string path)
        {
            if (!parent.TryGetProperty(property, out JsonElement value))
            {
                throw Fail(path, "missing required property '" + property + "'");
            }

            if (value.ValueKind != JsonValueKind.Array)
            {
                throw Fail(path, "'" + property + "' must be an array of three numbers");
            }

            if (value.GetArrayLength() != 3)
            {
                throw Fail(path, "'" + property + "' must contain exactly three numbers");
            }

            double[] components = new double[3];
            int index = 0;
            foreach (JsonElement item in value.EnumerateArray())
            {
                if (item.ValueKind != JsonValueKind.Number || !item.TryGetDouble(out double number))
                {
                    throw Fail(path + "[" + index.ToString(CultureInfo.InvariantCulture) + "]", "'" + property + "' component must be a number");
                }

                components[index] = number;
                index++;
            }

            return components;
        }

        private static ScenarioFormatException Fail(string path, string reason)
        {
            return new ScenarioFormatException("Scenario format error at " + path + ": " + reason);
        }
    }
}
