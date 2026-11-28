using System;
using System.Collections.Generic;
using System.IO;
using System.Text.Json.Nodes;

namespace Unitas.Editor.Mcp
{
    public sealed class ChangeSetStore
    {
        public const string PolicyFileRelativePath = ".unitas/review-policy.json";

        private const string Base64EncodingPrefix = "base64:";
        private readonly McpHost host;
        private readonly Dictionary<string, Func<JsonObject, JsonObject>> appliers =
            new Dictionary<string, Func<JsonObject, JsonObject>>(StringComparer.Ordinal);
        private readonly List<ChangeSet> order = new List<ChangeSet>();
        private readonly Dictionary<string, ChangeSet> sets =
            new Dictionary<string, ChangeSet>(StringComparer.Ordinal);
        private readonly Dictionary<string, JsonObject> restores =
            new Dictionary<string, JsonObject>(StringComparer.Ordinal);
        private readonly Dictionary<string, ReviewPolicy> policiesByRoot =
            new Dictionary<string, ReviewPolicy>(StringComparer.Ordinal);
        private long nextId;

        public ChangeSetStore(McpHost host)
        {
            ArgumentNullException.ThrowIfNull(host);
            this.host = host;
        }

        // Loaded once per project root on first access: later file edits must
        // not bypass an in-memory hold without a person-only changes.policy.set.
        public ReviewPolicy Policy => PolicyForRoot(null);

        // Tools that resolve their root from a projectPath argument (the same
        // resolution ProjectAccess uses) get that root's policy.
        public ReviewPolicy PolicyForArguments(JsonNode? arguments)
        {
            return PolicyForRoot(ArgumentString(arguments, "projectPath"));
        }

        private ReviewPolicy PolicyForRoot(string? requestedRoot)
        {
            string root = ResolvePolicyRoot(requestedRoot);
            if (!policiesByRoot.TryGetValue(root, out ReviewPolicy? cached))
            {
                cached = LoadPolicy(root, host.ProjectPath);
                policiesByRoot[root] = cached;
            }

            return cached;
        }

        private string ResolvePolicyRoot(string? requestedRoot)
        {
            string? root = requestedRoot;
            if (string.IsNullOrWhiteSpace(root))
            {
                root = host.ProjectPath;
            }

            if (string.IsNullOrWhiteSpace(root))
            {
                root = Directory.GetCurrentDirectory();
            }

            try
            {
                return Path.GetFullPath(root!);
            }
            catch (Exception error) when (error is ArgumentException || error is NotSupportedException)
            {
                return Path.GetFullPath(Directory.GetCurrentDirectory());
            }
        }

        public void RegisterApplier(string tool, Func<JsonObject, JsonObject> applier)
        {
            ArgumentException.ThrowIfNullOrWhiteSpace(tool);
            ArgumentNullException.ThrowIfNull(applier);
            appliers[tool] = applier;
        }

        public void SetPolicy(ReviewPolicy policy)
        {
            ArgumentNullException.ThrowIfNull(policy);
            policiesByRoot[ResolvePolicyRoot(null)] = policy;
            string? project = host.ProjectPath;
            if (string.IsNullOrWhiteSpace(project))
            {
                return;
            }

            string path = Path.Combine(project, PolicyFileRelativePath);
            string? directory = Path.GetDirectoryName(path);
            if (!string.IsNullOrEmpty(directory))
            {
                Directory.CreateDirectory(directory);
            }

            File.WriteAllText(path, policy.ToValue().ToJsonString());
        }

        // Walks up from the requested root to the nearest policy file so a
        // projectPath pointing at a subdirectory cannot escape the hold set
        // on the project tree; the walk never leaves the host project.
        private static ReviewPolicy LoadPolicy(string root, string? hostProject)
        {
            DirectoryInfo? directory = new DirectoryInfo(root);
            while (directory != null)
            {
                string candidate = Path.Combine(directory.FullName, PolicyFileRelativePath);
                try
                {
                    if (File.Exists(candidate))
                    {
                        return ReviewPolicy.FromValue(JsonNode.Parse(File.ReadAllText(candidate)) as JsonObject);
                    }
                }
                catch (Exception error) when (
                    error is IOException
                    || error is UnauthorizedAccessException
                    || error is System.Text.Json.JsonException
                    || error is FormatException)
                {
                    return ReviewPolicy.Default;
                }

                if (hostProject != null
                    && string.Equals(
                        directory.FullName.TrimEnd(Path.DirectorySeparatorChar, Path.AltDirectorySeparatorChar),
                        Path.GetFullPath(hostProject).TrimEnd(Path.DirectorySeparatorChar, Path.AltDirectorySeparatorChar),
                        StringComparison.Ordinal))
                {
                    break;
                }

                directory = directory.Parent;
            }

            return ReviewPolicy.Default;
        }

        // Every agent write tool calls this first: under holdAgentWrites the
        // refusal names the person-only unlock, and nothing is staged.
        public void EnsureAgentWritesAllowed(string toolName)
        {
            EnsureAgentWritesAllowed(toolName, null);
        }

        public void EnsureAgentWritesAllowed(string toolName, JsonNode? arguments)
        {
            if (PolicyForArguments(arguments).HoldAgentWrites)
            {
                throw new McpError.InvalidRequest(
                    toolName + " is disabled by the project review policy (holdAgentWrites); "
                        + "a person can relax it with changes.policy.set");
            }
        }

        // Reverts an applied set through the same file-restore path as rejection.
        public JsonObject Revert(string id)
        {
            ChangeSet set = Get(id);
            if (set.Status != ChangeSetStatus.Applied)
            {
                throw new ChangeSetNotApplied(id);
            }

            RecordTransition(set, "changes.revert");
            JsonObject report = RestoreFiles(set);
            restores[id] = report;
            return report;
        }

        public IReadOnlyList<ChangeSet> All => order;

        // Adds an operation to an existing set, mirroring Hold: under an
        // auto-apply root policy the new item is applied immediately.
        public ChangeItem AddOperation(ChangeSet set, JsonObject operation)
        {
            ArgumentNullException.ThrowIfNull(set);
            ArgumentNullException.ThrowIfNull(operation);
            set.AddOperation(operation);
            ChangeItem item = set.Items[set.Items.Count - 1];
            if (!PolicyForRoot(OperationString(operation, "projectPath")).RequireApproval)
            {
                try
                {
                    Apply(item, set.Operations[set.Operations.Count - 1]);
                    item.Status = ChangeItemStatus.Accepted;
                }
                catch (McpError)
                {
                    item.Status = ChangeItemStatus.Rejected;
                    throw;
                }
            }

            return item;
        }

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
            if (!PolicyForRoot(OperationString(operation, "projectPath")).RequireApproval)
            {
                for (int index = 0; index < set.Items.Count; index++)
                {
                    try
                    {
                        Apply(set.Items[index], set.Operations[index]);
                        set.Items[index].Status = ChangeItemStatus.Accepted;
                    }
                    catch (McpError)
                    {
                        set.Items[index].Status = ChangeItemStatus.Rejected;
                        throw;
                    }
                }
            }

            return set;
        }

        public ChangeSet AcceptItem(string changeSetId, string itemId)
        {
            return TransitionItem(changeSetId, itemId, ChangeItemStatus.Accepted, "changes.accept_item");
        }

        public ChangeSet RejectItem(string changeSetId, string itemId)
        {
            return TransitionItem(changeSetId, itemId, ChangeItemStatus.Rejected, "changes.reject_item");
        }

        public ChangeSet Approve(string id)
        {
            ChangeSet set = PendingSet(id);
            set.Status = ChangeSetStatus.Approved;
            RecordTransition(set, "changes.approve");
            try
            {
                for (int index = 0; index < set.Items.Count; index++)
                {
                    if (set.Items[index].Status == ChangeItemStatus.Accepted)
                    {
                        Apply(set.Items[index], set.Operations[index]);
                    }
                }
            }
            catch (Exception)
            {
                // A failed apply must not strand the set in Approved limbo:
                // back to Pending so the person can reject and restore files.
                set.Status = ChangeSetStatus.Pending;
                throw;
            }

            set.Status = ChangeSetStatus.Applied;
            RecordTransition(set, "changes.apply");
            return set;
        }

        private ChangeSet TransitionItem(string changeSetId, string itemId, ChangeItemStatus status, string action)
        {
            ChangeSet set = PendingSet(changeSetId);
            ChangeItem? item = null;
            foreach (ChangeItem candidate in set.Items)
            {
                if (string.Equals(candidate.Id, itemId, StringComparison.Ordinal))
                {
                    item = candidate;
                    break;
                }
            }

            if (item == null)
            {
                throw new UnknownChangeItem(changeSetId, itemId);
            }

            if (item.Status != ChangeItemStatus.Pending)
            {
                throw new ChangeItemNotPending(itemId);
            }

            item.Status = status;
            RecordTransition(set, action);
            return set;
        }

        private void Apply(ChangeItem item, JsonObject operation)
        {
            if (!appliers.TryGetValue(item.Tool, out Func<JsonObject, JsonObject>? applier))
            {
                throw new McpError.InvalidRequest("no applier registered for tool " + item.Tool);
            }

            applier(operation);
        }

        public ChangeSet Reject(string id, string reason)
        {
            ChangeSet set = PendingSet(id);
            set.Status = ChangeSetStatus.Rejected;
            set.RejectionReason = reason;
            RecordTransition(set, "changes.reject");
            restores[id] = RestoreFiles(set);
            return set;
        }

        public JsonObject RestoreReport(string id)
        {
            if (restores.TryGetValue(id, out JsonObject? report))
            {
                return report;
            }

            return new JsonObject
            {
                ["restoredCount"] = 0,
                ["restoredPaths"] = new JsonArray(),
                ["unrestoredPaths"] = new JsonArray(),
            };
        }

        private JsonObject RestoreFiles(ChangeSet set)
        {
            List<string> restored = new List<string>();
            List<string> unrestored = new List<string>();
            List<string> roots = CandidateRoots();

            for (int index = set.Operations.Count - 1; index >= 0; index--)
            {
                JsonObject operation = set.Operations[index];
                string? path = OperationString(operation, "path");
                if (path == null)
                {
                    continue;
                }

                string? before = OperationString(operation, "before");
                string? after = OperationString(operation, "after");
                string? target = before == null || after == null
                    ? null
                    : RecordedFile(roots, path, after);
                if (target == null || before == null)
                {
                    unrestored.Add(path);
                    continue;
                }

                try
                {
                    if (before.Length == 0)
                    {
                        File.Delete(target);
                        PruneEmptyParentDirectories(target, roots);
                    }
                    else if (before.StartsWith(Base64EncodingPrefix, StringComparison.Ordinal))
                    {
                        byte[] raw = Convert.FromBase64String(before.Substring(Base64EncodingPrefix.Length));
                        File.WriteAllBytes(target, raw);
                    }
                    else
                    {
                        File.WriteAllText(target, before);
                    }
                }
                catch (Exception error) when (error is IOException || error is UnauthorizedAccessException)
                {
                    unrestored.Add(path);
                    continue;
                }

                restored.Add(path);
            }

            JsonArray restoredPaths = new JsonArray();
            foreach (string path in restored)
            {
                restoredPaths.Add(path);
            }

            JsonArray unrestoredPaths = new JsonArray();
            foreach (string path in unrestored)
            {
                unrestoredPaths.Add(path);
            }

            return new JsonObject
            {
                ["restoredCount"] = restored.Count,
                ["restoredPaths"] = restoredPaths,
                ["unrestoredPaths"] = unrestoredPaths,
            };
        }

        private static void PruneEmptyParentDirectories(string targetFile, List<string> roots)
        {
            string? directory = Path.GetDirectoryName(targetFile);
            while (!string.IsNullOrEmpty(directory))
            {
                bool isRoot = false;
                foreach (string root in roots)
                {
                    if (string.Equals(
                        Path.GetFullPath(directory).TrimEnd(Path.DirectorySeparatorChar, Path.AltDirectorySeparatorChar),
                        Path.GetFullPath(root).TrimEnd(Path.DirectorySeparatorChar, Path.AltDirectorySeparatorChar),
                        StringComparison.Ordinal))
                    {
                        isRoot = true;
                        break;
                    }
                }

                if (isRoot || !Directory.Exists(directory))
                {
                    break;
                }

                if (Directory.GetFileSystemEntries(directory).Length > 0)
                {
                    break;
                }

                Directory.Delete(directory);
                directory = Path.GetDirectoryName(directory);
            }
        }

        private List<string> CandidateRoots()
        {
            List<string> roots = new List<string>();
            string? project = host.ProjectPath;
            if (!string.IsNullOrWhiteSpace(project))
            {
                roots.Add(project!);
            }

            string current = Directory.GetCurrentDirectory();
            if (!roots.Contains(current))
            {
                roots.Add(current);
            }

            string samples = Path.Combine(current, "samples");
            if (Directory.Exists(samples))
            {
                foreach (string child in Directory.GetDirectories(samples))
                {
                    if (!roots.Contains(child))
                    {
                        roots.Add(child);
                    }
                }
            }

            return roots;
        }

        private static string? RecordedFile(List<string> roots, string relative, string after)
        {
            foreach (string root in roots)
            {
                string? candidate = TargetPath(root, relative);
                if (candidate != null && HoldsContent(candidate, after))
                {
                    return candidate;
                }
            }

            return null;
        }

        private static string? TargetPath(string root, string relative)
        {
            try
            {
                string candidate = Path.GetFullPath(Path.Combine(root, relative));
                return AuditStore.IsPathInsideDirectory(candidate, root) ? candidate : null;
            }
            catch (Exception error) when (error is ArgumentException || error is NotSupportedException)
            {
                return null;
            }
        }

        private static bool HoldsContent(string path, string content)
        {
            try
            {
                if (!File.Exists(path))
                {
                    return false;
                }

                if (content.StartsWith(Base64EncodingPrefix, StringComparison.Ordinal))
                {
                    byte[] expected = Convert.FromBase64String(content.Substring(Base64EncodingPrefix.Length));
                    byte[] actual = File.ReadAllBytes(path);
                    if (expected.Length != actual.Length)
                    {
                        return false;
                    }

                    for (int index = 0; index < expected.Length; index++)
                    {
                        if (expected[index] != actual[index])
                        {
                            return false;
                        }
                    }

                    return true;
                }

                return File.ReadAllText(path) == content;
            }
            catch (Exception error) when (error is IOException || error is UnauthorizedAccessException || error is FormatException)
            {
                return false;
            }
        }

        private static string? ArgumentString(JsonNode? arguments, string propertyName)
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

        private static string? OperationString(JsonObject operation, string propertyName)
        {
            if (operation[propertyName] is JsonValue value && value.TryGetValue(out string? text))
            {
                return text;
            }

            return null;
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
