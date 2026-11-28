using System;
using System.Collections.Generic;
using System.Globalization;
using System.Text.Json.Nodes;

namespace Unitas.Editor.Mcp
{
    public enum ChangeSetStatus
    {
        Pending,
        Approved,
        Rejected,
        Applied,
    }

    public static class ChangeSetStatuses
    {
        public static string AsString(ChangeSetStatus status)
        {
            switch (status)
            {
                case ChangeSetStatus.Pending: return "pending";
                case ChangeSetStatus.Approved: return "approved";
                case ChangeSetStatus.Rejected: return "rejected";
                case ChangeSetStatus.Applied: return "applied";
                default: return status.ToString();
            }
        }
    }

    public sealed class UnknownChangeSet : McpError
    {
        public UnknownChangeSet(string changeSetId)
            : base("unknown change-set " + changeSetId)
        {
            ChangeSetId = changeSetId;
        }

        public string ChangeSetId { get; }
    }

    public sealed class ChangeSetNotPending : McpError
    {
        public ChangeSetNotPending(string changeSetId)
            : base("change-set " + changeSetId + " is no longer pending review")
        {
            ChangeSetId = changeSetId;
        }

        public string ChangeSetId { get; }
    }

    public enum ChangeItemStatus
    {
        Pending,
        Accepted,
        Rejected,
    }

    public static class ChangeItemStatuses
    {
        public static string AsString(ChangeItemStatus status)
        {
            switch (status)
            {
                case ChangeItemStatus.Pending: return "pending";
                case ChangeItemStatus.Accepted: return "accepted";
                case ChangeItemStatus.Rejected: return "rejected";
                default: return status.ToString();
            }
        }
    }

    public enum ChangeRisk
    {
        TestDeletion,
        ToleranceWidening,
        ScriptEdit,
        ContentEdit,
    }

    public static class ChangeRisks
    {
        private static readonly ChangeRisk[] severityOrder =
        {
            ChangeRisk.TestDeletion,
            ChangeRisk.ToleranceWidening,
            ChangeRisk.ScriptEdit,
            ChangeRisk.ContentEdit,
        };

        public static IReadOnlyList<ChangeRisk> SeverityOrder => severityOrder;

        public static string AsString(ChangeRisk risk)
        {
            switch (risk)
            {
                case ChangeRisk.TestDeletion: return "test-deletion";
                case ChangeRisk.ToleranceWidening: return "tolerance-widening";
                case ChangeRisk.ScriptEdit: return "script-edit";
                case ChangeRisk.ContentEdit: return "content-edit";
                default: return risk.ToString();
            }
        }
    }

    public sealed class ChangeItem
    {
        internal ChangeItem(string id, string tool, string? path, ChangeRisk risk, JsonObject diff)
        {
            Id = id;
            Tool = tool;
            Path = path;
            Risk = risk;
            Diff = diff;
            Status = ChangeItemStatus.Pending;
        }

        public string Id { get; }

        public string Tool { get; }

        public string? Path { get; }

        public ChangeItemStatus Status { get; internal set; }

        public ChangeRisk Risk { get; }

        public JsonObject Diff { get; }

        public JsonObject ToValue()
        {
            return new JsonObject
            {
                ["id"] = Id,
                ["tool"] = Tool,
                ["path"] = Path,
                ["status"] = ChangeItemStatuses.AsString(Status),
                ["risk"] = ChangeRisks.AsString(Risk),
                ["diff"] = Diff.DeepClone(),
            };
        }

        internal static ChangeItem FromOperation(string id, JsonObject operation)
        {
            ArgumentNullException.ThrowIfNull(operation);
            string tool = StringOf(operation, "tool") ?? string.Empty;
            string? path = StringOf(operation, "path");
            return new ChangeItem(id, tool, path, ClassifyRisk(operation, tool, path), BuildDiff(operation, path));
        }

        private static JsonObject BuildDiff(JsonObject operation, string? path)
        {
            string? scene = StringOf(operation, "scene");
            JsonNode? before = operation["before"];
            JsonNode? after = operation["after"];
            if (operation["properties"] is JsonArray properties)
            {
                return new JsonObject
                {
                    ["kind"] = "properties",
                    ["path"] = path ?? scene,
                    ["properties"] = properties.DeepClone(),
                };
            }

            if (scene != null && path != null && (before != null || after != null))
            {
                return new JsonObject
                {
                    ["kind"] = "properties",
                    ["path"] = scene,
                    ["properties"] = new JsonArray
                    {
                        new JsonObject
                        {
                            ["name"] = path,
                            ["before"] = before?.DeepClone(),
                            ["after"] = after?.DeepClone(),
                        },
                    },
                };
            }

            if (path != null && (before != null || after != null))
            {
                return new JsonObject
                {
                    ["kind"] = "text",
                    ["path"] = path,
                    ["before"] = before?.DeepClone(),
                    ["after"] = after?.DeepClone(),
                };
            }

            JsonObject opaque = new JsonObject
            {
                ["kind"] = "opaque",
            };
            if (before != null)
            {
                opaque["before"] = before.DeepClone();
            }

            if (after != null)
            {
                opaque["after"] = after.DeepClone();
            }

            return opaque;
        }

        private static ChangeRisk ClassifyRisk(JsonObject operation, string tool, string? path)
        {
            if (path != null && IsTestPath(path) && EmptiesOrDeletes(operation, tool))
            {
                return ChangeRisk.TestDeletion;
            }

            if (WidensTolerance(operation, path))
            {
                return ChangeRisk.ToleranceWidening;
            }

            if (path != null && path.EndsWith(".cs", StringComparison.OrdinalIgnoreCase))
            {
                return ChangeRisk.ScriptEdit;
            }

            return ChangeRisk.ContentEdit;
        }

        private static bool IsTestPath(string path)
        {
            if (path.EndsWith("Tests.cs", StringComparison.OrdinalIgnoreCase))
            {
                return true;
            }

            foreach (string segment in path.Split('/', '\\'))
            {
                if (string.Equals(segment, "tests", StringComparison.OrdinalIgnoreCase))
                {
                    return true;
                }
            }

            return false;
        }

        private static bool EmptiesOrDeletes(JsonObject operation, string tool)
        {
            JsonNode? after = operation["after"];
            if (after == null)
            {
                return true;
            }

            if (after is JsonValue value && value.TryGetValue(out string? text) && text.Length == 0)
            {
                return true;
            }

            return tool.Contains("delete", StringComparison.OrdinalIgnoreCase)
                || tool.Contains("remove", StringComparison.OrdinalIgnoreCase);
        }

        private static bool WidensTolerance(JsonObject operation, string? path)
        {
            if (path != null && Widened(path, operation["before"], operation["after"]))
            {
                return true;
            }

            if (operation["properties"] is JsonArray properties)
            {
                foreach (JsonNode? entry in properties)
                {
                    if (entry is JsonObject property
                        && StringOf(property, "name") is string name
                        && Widened(name, property["before"], property["after"]))
                    {
                        return true;
                    }
                }
            }

            return false;
        }

        private static bool Widened(string name, JsonNode? before, JsonNode? after)
        {
            return name.Contains("tolerance", StringComparison.OrdinalIgnoreCase)
                && TryNumber(before, out double beforeNumber)
                && TryNumber(after, out double afterNumber)
                && afterNumber > beforeNumber;
        }

        private static bool TryNumber(JsonNode? node, out double number)
        {
            number = 0;
            if (node is not JsonValue value)
            {
                return false;
            }

            if (value.TryGetValue(out double numeric))
            {
                number = numeric;
                return true;
            }

            return value.TryGetValue(out string? text)
                && text != null
                && double.TryParse(text, NumberStyles.Float, CultureInfo.InvariantCulture, out number);
        }

        private static string? StringOf(JsonObject operation, string propertyName)
        {
            if (operation[propertyName] is JsonValue value && value.TryGetValue(out string? text))
            {
                return text;
            }

            return null;
        }
    }

    public sealed class ChangeSet
    {
        private readonly List<JsonObject> operations = new List<JsonObject>();
        private readonly List<ChangeItem> items = new List<ChangeItem>();

        internal ChangeSet(string id, string author, DateTime createdUtc, JsonObject operation)
        {
            Id = id;
            Author = author;
            CreatedUtc = createdUtc;
            Status = ChangeSetStatus.Pending;
            operations.Add(operation);
            items.Add(ChangeItem.FromOperation(id + ".1", operation));
        }

        internal void AddOperation(JsonObject operation)
        {
            ArgumentNullException.ThrowIfNull(operation);
            JsonObject clone = (JsonObject)operation.DeepClone();
            operations.Add(clone);
            items.Add(ChangeItem.FromOperation(Id + "." + (items.Count + 1).ToString(CultureInfo.InvariantCulture), clone));
        }

        public string Id { get; }

        public string Author { get; }

        public DateTime CreatedUtc { get; }

        public ChangeSetStatus Status { get; internal set; }

        public string? RejectionReason { get; internal set; }

        public IReadOnlyList<JsonObject> Operations => operations;

        public IReadOnlyList<ChangeItem> Items => items;

        public JsonObject ToValue()
        {
            JsonArray serializedOperations = new JsonArray();
            foreach (JsonObject operation in operations)
            {
                serializedOperations.Add(operation.DeepClone());
            }

            JsonArray serializedItems = new JsonArray();
            foreach (ChangeItem item in items)
            {
                serializedItems.Add(item.ToValue());
            }

            return new JsonObject
            {
                ["id"] = Id,
                ["author"] = Author,
                ["createdUtc"] = CreatedUtc.ToString("O", CultureInfo.InvariantCulture),
                ["status"] = ChangeSetStatuses.AsString(Status),
                ["operations"] = serializedOperations,
                ["items"] = serializedItems,
            };
        }
    }
}
