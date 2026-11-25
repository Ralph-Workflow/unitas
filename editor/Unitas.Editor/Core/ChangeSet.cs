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

    // A held agent-identity mutation under the project review policy (EDT-008,
    // contract C1). The JSON shape is pinned: {id, author, createdUtc, status,
    // operations}.
    public sealed class ChangeSet
    {
        private readonly List<JsonObject> operations = new List<JsonObject>();

        internal ChangeSet(string id, string author, DateTime createdUtc, JsonObject operation)
        {
            Id = id;
            Author = author;
            CreatedUtc = createdUtc;
            Status = ChangeSetStatus.Pending;
            operations.Add(operation);
        }

        public string Id { get; }

        public string Author { get; }

        public DateTime CreatedUtc { get; }

        public ChangeSetStatus Status { get; internal set; }

        public string? RejectionReason { get; internal set; }

        public IReadOnlyList<JsonObject> Operations => operations;

        public JsonObject ToValue()
        {
            JsonArray serializedOperations = new JsonArray();
            foreach (JsonObject operation in operations)
            {
                serializedOperations.Add(operation.DeepClone());
            }

            return new JsonObject
            {
                ["id"] = Id,
                ["author"] = Author,
                ["createdUtc"] = CreatedUtc.ToString("O", CultureInfo.InvariantCulture),
                ["status"] = ChangeSetStatuses.AsString(Status),
                ["operations"] = serializedOperations,
            };
        }
    }
}
