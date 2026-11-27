using System;
using System.Globalization;
using System.Text.Json;
using System.Text.Json.Nodes;

namespace Unitas.Editor.Shell
{
    public sealed class ShellEvent
    {
        public const string TypeInit = "init";
        public const string TypeSelect = "select";
        public const string TypeSelectNext = "select_next";
        public const string TypeSelectProperty = "select_property";
        public const string TypeStepProperty = "step_property";
        public const string TypeToggleProperty = "toggle_property";
        public const string TypeToggleActive = "toggle_active";
        public const string TypeCommand = "command";

        public ShellEvent(
            string type,
            string? targetId = null,
            int? index = null,
            string? command = null,
            JsonObject? arguments = null,
            string? projectPath = null,
            double? delta = null)
        {
            ArgumentNullException.ThrowIfNull(type);
            Type = type;
            TargetId = targetId;
            Index = index;
            Command = command;
            Arguments = arguments;
            ProjectPath = projectPath;
            Delta = delta;
        }

        public string Type { get; }

        public string? TargetId { get; }

        public int? Index { get; }

        public string? Command { get; }

        public JsonObject? Arguments { get; }

        public string? ProjectPath { get; }

        public double? Delta { get; }

        public static ShellEvent Init(string? projectPath = null)
        {
            return new ShellEvent(TypeInit, projectPath: projectPath);
        }

        public static ShellEvent SelectByIndex(int index)
        {
            return new ShellEvent(TypeSelect, index: index);
        }

        public static ShellEvent SelectById(string targetId)
        {
            ArgumentNullException.ThrowIfNull(targetId);
            return new ShellEvent(TypeSelect, targetId: targetId);
        }

        public static ShellEvent SelectNext()
        {
            return new ShellEvent(TypeSelectNext);
        }

        public static ShellEvent SelectProperty(int index)
        {
            return new ShellEvent(TypeSelectProperty, index: index);
        }

        public static ShellEvent StepProperty(double delta)
        {
            return new ShellEvent(TypeStepProperty, delta: delta);
        }

        public static ShellEvent ToggleProperty()
        {
            return new ShellEvent(TypeToggleProperty);
        }

        public static ShellEvent ToggleActive()
        {
            return new ShellEvent(TypeToggleActive);
        }

        public static ShellEvent InvokeCommand(string command, JsonObject? arguments = null)
        {
            ArgumentNullException.ThrowIfNull(command);
            return new ShellEvent(TypeCommand, command: command, arguments: arguments);
        }

        public static ShellEvent Parse(string eventJson)
        {
            if (string.IsNullOrWhiteSpace(eventJson))
            {
                return Init();
            }

            JsonNode? parsed;
            try
            {
                parsed = JsonNode.Parse(eventJson);
            }
            catch (JsonException)
            {
                return Init();
            }

            if (!(parsed is JsonObject obj))
            {
                return Init();
            }

            string type = ReadString(obj, "type") ?? TypeInit;
            string? targetId = ReadString(obj, "targetId") ?? ReadString(obj, "fileId");
            int? index = ReadInt(obj, "index");
            double? delta = ReadDouble(obj, "delta");
            string? command = ReadString(obj, "command");
            string? projectPath = ReadString(obj, "projectPath");
            JsonObject? arguments = (obj["arguments"] as JsonObject)?.DeepClone() as JsonObject;
            return new ShellEvent(type, targetId, index, command, arguments, projectPath, delta);
        }

        public JsonObject ToJsonObject()
        {
            JsonObject obj = new JsonObject
            {
                ["type"] = Type,
            };

            if (!string.IsNullOrEmpty(TargetId))
            {
                obj["targetId"] = TargetId;
            }

            if (Index.HasValue)
            {
                obj["index"] = Index.Value;
            }

            if (Delta.HasValue)
            {
                obj["delta"] = Delta.Value;
            }

            if (!string.IsNullOrEmpty(Command))
            {
                obj["command"] = Command;
            }

            if (Arguments != null)
            {
                obj["arguments"] = Arguments.DeepClone();
            }

            if (!string.IsNullOrEmpty(ProjectPath))
            {
                obj["projectPath"] = ProjectPath;
            }

            return obj;
        }

        public string ToJsonString()
        {
            return ToJsonObject().ToJsonString();
        }

        private static string? ReadString(JsonObject obj, string propertyName)
        {
            JsonNode? node = obj[propertyName];
            if (node is JsonValue value)
            {
                if (value.TryGetValue(out string? text) && !string.IsNullOrWhiteSpace(text))
                {
                    return text;
                }

                if (value.TryGetValue(out long number))
                {
                    return number.ToString(CultureInfo.InvariantCulture);
                }

                if (value.TryGetValue(out int small))
                {
                    return small.ToString(CultureInfo.InvariantCulture);
                }
            }

            return null;
        }

        private static int? ReadInt(JsonObject obj, string propertyName)
        {
            JsonNode? node = obj[propertyName];
            if (node is JsonValue value)
            {
                if (value.TryGetValue(out int intVal))
                {
                    return intVal;
                }

                if (value.TryGetValue(out long longVal)
                    && longVal >= int.MinValue
                    && longVal <= int.MaxValue)
                {
                    return (int)longVal;
                }
            }

            return null;
        }

        private static double? ReadDouble(JsonObject obj, string propertyName)
        {
            JsonNode? node = obj[propertyName];
            if (node is JsonValue value)
            {
                if (value.TryGetValue(out double doubleVal))
                {
                    return doubleVal;
                }

                if (value.TryGetValue(out long longVal))
                {
                    return longVal;
                }

                if (value.TryGetValue(out int intVal))
                {
                    return intVal;
                }
            }

            return null;
        }
    }
}
