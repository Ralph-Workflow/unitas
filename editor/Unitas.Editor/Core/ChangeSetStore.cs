using System;
using System.Collections.Generic;
using System.Text.Json.Nodes;

namespace Unitas.Editor.Mcp
{
    // Holds agent-identity mutations under the project review policy (EDT-008):
    // a held mutation stays pending until a person approves it (then it is
    // applied through the applier) or rejects it. Every transition appends an
    // audit record, so the store is the single choke point for transitions.
    public sealed class ChangeSetStore
    {
        private readonly McpHost host;
        private readonly Func<JsonObject, JsonObject>? applier;
        private readonly List<ChangeSet> order = new List<ChangeSet>();
        private readonly Dictionary<string, ChangeSet> sets =
            new Dictionary<string, ChangeSet>(StringComparer.Ordinal);
        private long nextId;

        public ChangeSetStore(McpHost host, Func<JsonObject, JsonObject>? applier = null)
        {
            ArgumentNullException.ThrowIfNull(host);
            this.host = host;
            this.applier = applier;
        }

        public IReadOnlyList<ChangeSet> All => order;

        public IReadOnlyList<ChangeSet> Pending
        {
            get
            {
                List<ChangeSet> pending = new List<ChangeSet>();
                foreach (ChangeSet set in order)
                {
                    if (set.Status == ChangeSetStatus.Pending)
                    {
                        pending.Add(set);
                    }
                }

                return pending;
            }
        }

        public ChangeSet Get(string id)
        {
            if (!sets.TryGetValue(id, out ChangeSet? set))
            {
                throw new UnknownChangeSet(id);
            }

            return set;
        }

        public ChangeSet Hold(string author, JsonObject operation)
        {
            if (string.IsNullOrWhiteSpace(author))
            {
                throw new McpError.InvalidParams("a held change-set requires an author");
            }

            nextId++;
            string id = "cs-" + nextId;
            DateTime createdUtc = DateTimeOffset.FromUnixTimeMilliseconds(host.Clock.NowMs()).UtcDateTime;
            ChangeSet set = new ChangeSet(id, author, createdUtc, (JsonObject)operation.DeepClone());
            sets.Add(id, set);
            order.Add(set);
            RecordTransition(set, "changes.hold");
            return set;
        }

        public ChangeSet Approve(string id)
        {
            ChangeSet set = PendingSet(id);
            set.Status = ChangeSetStatus.Approved;
            RecordTransition(set, "changes.approve");
            if (applier != null)
            {
                foreach (JsonObject operation in set.Operations)
                {
                    applier(operation);
                }

                set.Status = ChangeSetStatus.Applied;
                RecordTransition(set, "changes.apply");
            }

            return set;
        }

        public ChangeSet Reject(string id, string reason)
        {
            ChangeSet set = PendingSet(id);
            set.Status = ChangeSetStatus.Rejected;
            set.RejectionReason = reason;
            RecordTransition(set, "changes.reject");
            return set;
        }

        private ChangeSet PendingSet(string id)
        {
            ChangeSet set = Get(id);
            if (set.Status != ChangeSetStatus.Pending)
            {
                throw new ChangeSetNotPending(id);
            }

            return set;
        }

        private void RecordTransition(ChangeSet set, string action)
        {
            string sessionId = host.CurrentSessionId
                ?? throw new McpError.InvalidRequest("change-set transitions require an active MCP session");
            SessionIdentity identity = host.GetSessionIdentity(sessionId);
            host.Audit.Record(new AuditEntry(
                host.Clock.NowMs(),
                sessionId,
                identity,
                AuditTargetKind.Operation,
                action,
                Scope.Review,
                new AuditOutcome.Allowed(),
                new[] { set.Id }));
        }
    }
}
