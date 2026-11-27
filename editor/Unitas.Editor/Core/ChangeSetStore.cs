using System;
using System.Collections.Generic;
using System.IO;
using System.Text.Json.Nodes;

namespace Unitas.Editor.Mcp
{
    public sealed class ChangeSetStore
    {
        private const string Base64EncodingPrefix = "base64:";
        private readonly McpHost host;
        private readonly Func<JsonObject, JsonObject>? applier;
        private readonly List<ChangeSet> order = new List<ChangeSet>();
        private readonly Dictionary<string, ChangeSet> sets =
            new Dictionary<string, ChangeSet>(StringComparer.Ordinal);
        private readonly Dictionary<string, JsonObject> restores =
            new Dictionary<string, JsonObject>(StringComparer.Ordinal);
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
