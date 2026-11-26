using System;
using System.Collections.Generic;
using System.IO;
using System.Text.Json.Nodes;
using Unitas.Editor.Mcp;

namespace Unitas.Editor.Tests
{
    public static class ChangeReviewTests
    {
        public static void Run()
        {
            AgentMutationIsHeldUnderReviewPolicy();
            PersonApproveAppliesHeldMutationAndAuditsTransitions();
            PersonRejectRefusesHeldMutationAndClosesIt();
            RejectRestoresTheFilesTheChangeSetChanged();
        }

        private static void AgentMutationIsHeldUnderReviewPolicy()
        {
            FixedClock clock = new FixedClock(1_000);
            using McpHost host = new McpHost(clock);
            ChangeSetStore store = new ChangeSetStore(host);
            ChangeReviewTools.Register(host, store);
            RegisterSyntheticMutator(host, store);
            string agentSession = host.CreateSession(new SessionIdentity("agent-a", "stdio", "person-1"));

            JsonObject content = ContentOf(host.CallTool(
                agentSession,
                "test.mutate",
                new JsonObject
                {
                    ["value"] = "new-value",
                }));
            Check.Equal("held: held outcome status", "held-for-review", content["status"]!.GetValue<string>());
            Check.Equal("held: held outcome names the change set", "cs-1", content["changeId"]!.GetValue<string>());

            JsonObject listed = ContentOf(host.CallTool(agentSession, "changes.list", new JsonObject()));
            JsonArray changeSets = (JsonArray)listed["changeSets"]!;
            Check.Equal("held: one change set listed", 1, changeSets.Count);
            JsonObject changeSet = (JsonObject)changeSets[0]!;
            Check.Equal("held: change set id", "cs-1", changeSet["id"]!.GetValue<string>());
            Check.Equal("held: author is the agent identity", "agent-a", changeSet["author"]!.GetValue<string>());
            Check.Equal(
                "held: createdUtc from the clock",
                "1970-01-01T00:00:01.0000000Z",
                changeSet["createdUtc"]!.GetValue<string>());
            Check.Equal("held: status pending", "pending", changeSet["status"]!.GetValue<string>());
            JsonArray operations = (JsonArray)changeSet["operations"]!;
            Check.Equal("held: one operation recorded", 1, operations.Count);
            JsonObject operation = (JsonObject)operations[0]!;
            Check.Equal("held: operation names the tool", "test.mutate", operation["tool"]!.GetValue<string>());
            Check.Equal(
                "held: operation keeps the arguments",
                "new-value",
                ((JsonObject)operation["arguments"]!)["value"]!.GetValue<string>());

            JsonObject shown = ContentOf(host.CallTool(
                agentSession,
                "changes.show",
                new JsonObject
                {
                    ["id"] = "cs-1",
                }));
            Check.Equal("held: show returns the same change set", "cs-1", shown["id"]!.GetValue<string>());
            Check.Equal("held: show reports pending", "pending", shown["status"]!.GetValue<string>());

            JsonObject pendingResource = ContentOf(host.ReadResource(agentSession, ChangeReviewTools.PendingOrigin));
            Check.Equal(
                "held: pending resource lists the held change",
                1,
                ((JsonArray)pendingResource["changeSets"]!).Count);

            bool unknownRefused = false;
            try
            {
                host.CallTool(agentSession, "changes.show", new JsonObject { ["id"] = "cs-missing" });
            }
            catch (UnknownChangeSet)
            {
                unknownRefused = true;
            }

            Check.That("held: unknown change set id is refused", unknownRefused);

            List<AuditRecord> transitions = TransitionRecords(host);
            Check.Equal("held: one transition recorded", 1, transitions.Count);
            Check.Equal("held: hold transition audited", "changes.hold", transitions[0].Target);
            Check.That("held: transition scoped to review", transitions[0].Scope == Scope.Review);
            Check.That(
                "held: transition names the change set",
                transitions[0].AffectedIdentities.Count == 1
                && transitions[0].AffectedIdentities[0] == "cs-1");
        }

        private static void PersonApproveAppliesHeldMutationAndAuditsTransitions()
        {
            FixedClock clock = new FixedClock(2_000);
            using McpHost host = new McpHost(clock, PersonCredential.FromToken("person-token"));
            string? appliedValue = null;
            ChangeSetStore store = new ChangeSetStore(host, operation =>
            {
                appliedValue = ((JsonObject)operation["arguments"]!)["value"]!.GetValue<string>();
                return new JsonObject
                {
                    ["applied"] = true,
                };
            });
            ChangeReviewTools.Register(host, store);
            RegisterSyntheticMutator(host, store);
            string agentSession = host.CreateSession(new SessionIdentity("agent-a", "stdio", "person-1"));
            string personSession = host.CreateSession(new SessionIdentity("person-ui", "http", "person-1"));
            host.CallTool(agentSession, "test.mutate", new JsonObject { ["value"] = "agent-proposal" });
            Check.That("approve: mutation not applied while held", appliedValue == null);

            McpError.PersonCredentialRequired? stdioError = null;
            try
            {
                host.CallTool(
                    agentSession,
                    "changes.approve",
                    new JsonObject
                    {
                        ["id"] = "cs-1",
                        ["personCredential"] = "person-token",
                    });
            }
            catch (McpError.PersonCredentialRequired error)
            {
                stdioError = error;
            }

            Check.That(
                "approve: agent over stdio can never approve, even with the right token",
                stdioError != null
                && stdioError.Message.Contains(
                    "requests over stdio are never treated as a person's",
                    StringComparison.Ordinal));
            Check.Equal(
                "approve: refused attempt leaves the change pending",
                "pending",
                ContentOf(host.CallTool(personSession, "changes.show", new JsonObject { ["id"] = "cs-1" }))["status"]!
                    .GetValue<string>());

            bool missingTokenRefused = false;
            try
            {
                host.CallTool(personSession, "changes.approve", new JsonObject { ["id"] = "cs-1" });
            }
            catch (McpError.PersonCredentialRequired)
            {
                missingTokenRefused = true;
            }

            Check.That("approve: person without a token is refused", missingTokenRefused);

            bool wrongTokenRefused = false;
            try
            {
                host.CallTool(
                    personSession,
                    "changes.approve",
                    new JsonObject
                    {
                        ["id"] = "cs-1",
                        ["personCredential"] = "wrong-token",
                    });
            }
            catch (McpError.PersonCredentialRequired)
            {
                wrongTokenRefused = true;
            }

            Check.That("approve: person with a wrong token is refused", wrongTokenRefused);

            JsonObject approved = ContentOf(host.CallTool(
                personSession,
                "changes.approve",
                new JsonObject
                {
                    ["id"] = "cs-1",
                    ["personCredential"] = "person-token",
                }));
            Check.Equal("approve: change set is applied", "applied", approved["status"]!.GetValue<string>());
            Check.Equal("approve: approved operation executed", "agent-proposal", appliedValue!);

            List<AuditRecord> transitions = TransitionRecords(host);
            Check.Equal("approve: hold, approve, apply transitions audited", 3, transitions.Count);
            Check.Equal("approve: first transition is hold", "changes.hold", transitions[0].Target);
            Check.Equal("approve: second transition is approve", "changes.approve", transitions[1].Target);
            Check.Equal("approve: third transition is apply", "changes.apply", transitions[2].Target);
            Check.That(
                "approve: transitions by the person identity",
                transitions[1].Identity.Client == "http" && transitions[2].Identity.Client == "http");
            Check.That(
                "approve: refused agent attempt audited as failed tool call",
                HasOutcomeFor(host, "changes.approve", "failed"));
        }

        private static void PersonRejectRefusesHeldMutationAndClosesIt()
        {
            FixedClock clock = new FixedClock(3_000);
            using McpHost host = new McpHost(clock, PersonCredential.FromToken("person-token"));
            bool applied = false;
            ChangeSetStore store = new ChangeSetStore(host, _ =>
            {
                applied = true;
                return new JsonObject();
            });
            ChangeReviewTools.Register(host, store);
            RegisterSyntheticMutator(host, store);
            string agentSession = host.CreateSession(new SessionIdentity("agent-a", "stdio", "person-1"));
            string personSession = host.CreateSession(new SessionIdentity("person-ui", "http", "person-1"));
            host.CallTool(agentSession, "test.mutate", new JsonObject { ["value"] = "risky" });

            McpError.PersonCredentialRequired? stdioError = null;
            try
            {
                host.CallTool(
                    agentSession,
                    "changes.reject",
                    new JsonObject
                    {
                        ["id"] = "cs-1",
                        ["reason"] = "not wanted",
                        ["personCredential"] = "person-token",
                    });
            }
            catch (McpError.PersonCredentialRequired error)
            {
                stdioError = error;
            }

            Check.That("reject: agent over stdio can never reject", stdioError != null);

            bool missingReasonRefused = false;
            try
            {
                host.CallTool(
                    personSession,
                    "changes.reject",
                    new JsonObject
                    {
                        ["id"] = "cs-1",
                        ["personCredential"] = "person-token",
                    });
            }
            catch (McpError.InvalidParams)
            {
                missingReasonRefused = true;
            }

            Check.That("reject: missing reason is refused", missingReasonRefused);

            JsonObject rejected = ContentOf(host.CallTool(
                personSession,
                "changes.reject",
                new JsonObject
                {
                    ["id"] = "cs-1",
                    ["reason"] = "breaks the scene",
                    ["personCredential"] = "person-token",
                }));
            Check.Equal("reject: change set is rejected", "rejected", rejected["status"]!.GetValue<string>());
            Check.Equal("reject: reason kept on the change set", "breaks the scene", store.Get("cs-1").RejectionReason!);
            Check.That("reject: rejected operation never applied", !applied);

            bool approveAfterRejectRefused = false;
            try
            {
                host.CallTool(
                    personSession,
                    "changes.approve",
                    new JsonObject
                    {
                        ["id"] = "cs-1",
                        ["personCredential"] = "person-token",
                    });
            }
            catch (ChangeSetNotPending)
            {
                approveAfterRejectRefused = true;
            }

            Check.That("reject: approving a rejected change set is refused", approveAfterRejectRefused);

            JsonObject pendingResource = ContentOf(host.ReadResource(personSession, ChangeReviewTools.PendingOrigin));
            Check.Equal(
                "reject: pending resource no longer lists the rejected change",
                0,
                ((JsonArray)pendingResource["changeSets"]!).Count);

            List<AuditRecord> transitions = TransitionRecords(host);
            Check.Equal("reject: hold and reject transitions audited", 2, transitions.Count);
            Check.Equal("reject: reject transition audited", "changes.reject", transitions[1].Target);
        }

        private static void RejectRestoresTheFilesTheChangeSetChanged()
        {
            FixedClock clock = new FixedClock(4_000);
            string project = CreateProject("reject-restore");
            try
            {
                using McpHost host = new McpHost(clock, PersonCredential.FromToken("person-token"));
                ChangeSetStore store = new ChangeSetStore(host);
                ProjectTools.Register(host, store);
                ChangeReviewTools.Register(host, store);
                RegisterSyntheticMutator(host, store);
                host.OpenProject(project);
                string agentSession = host.CreateSession(new SessionIdentity("agent-a", "stdio", "person-1"));
                host.GrantScope(agentSession, Scope.EditContent, GrantAuthority.Person);
                string personSession = host.CreateSession(new SessionIdentity("person-ui", "http", "person-1"));

                File.WriteAllText(Path.Combine(project, "Assets", "Existing.txt"), "original\n");
                host.CallTool(
                    agentSession,
                    "project.write_file",
                    new JsonObject
                    {
                        ["projectPath"] = project,
                        ["path"] = "Assets/Added.txt",
                        ["content"] = "added\n",
                    });
                host.CallTool(
                    agentSession,
                    "project.write_file",
                    new JsonObject
                    {
                        ["projectPath"] = project,
                        ["path"] = "Assets/Existing.txt",
                        ["content"] = "changed\n",
                    });
                host.CallTool(
                    agentSession,
                    "test.mutate",
                    new JsonObject
                    {
                        ["value"] = "risky",
                    });

                JsonObject added = ContentOf(host.CallTool(
                    personSession,
                    "changes.reject",
                    new JsonObject
                    {
                        ["id"] = "cs-1",
                        ["reason"] = "not wanted",
                        ["personCredential"] = "person-token",
                    }));
                Check.That(
                    "reject: the file that did not exist before is gone",
                    !File.Exists(Path.Combine(project, "Assets", "Added.txt")));
                JsonObject addedRestore = (JsonObject)added["restore"]!;
                Check.Equal(
                    "reject: the restore report counts the restored file",
                    1,
                    addedRestore["restoredCount"]!.GetValue<int>());
                Check.Equal(
                    "reject: the restore report names the restored path",
                    "Assets/Added.txt",
                    ((JsonArray)addedRestore["restoredPaths"]!)[0]!.GetValue<string>());

                JsonObject existing = ContentOf(host.CallTool(
                    personSession,
                    "changes.reject",
                    new JsonObject
                    {
                        ["id"] = "cs-2",
                        ["reason"] = "not wanted",
                        ["personCredential"] = "person-token",
                    }));
                Check.Equal(
                    "reject: the pre-existing file holds its original content",
                    "original\n",
                    File.ReadAllText(Path.Combine(project, "Assets", "Existing.txt")));
                JsonObject existingRestore = (JsonObject)existing["restore"]!;
                Check.Equal(
                    "reject: the restore report counts the rewritten file",
                    1,
                    existingRestore["restoredCount"]!.GetValue<int>());
                Check.Equal(
                    "reject: the restore report names the rewritten path",
                    "Assets/Existing.txt",
                    ((JsonArray)existingRestore["restoredPaths"]!)[0]!.GetValue<string>());

                JsonObject held = ContentOf(host.CallTool(
                    personSession,
                    "changes.reject",
                    new JsonObject
                    {
                        ["id"] = "cs-3",
                        ["reason"] = "not wanted",
                        ["personCredential"] = "person-token",
                    }));
                Check.Equal(
                    "reject: a change set without a recorded path restores nothing",
                    0,
                    ((JsonObject)held["restore"]!)["restoredCount"]!.GetValue<int>());
            }
            finally
            {
                DeleteDirectory(project);
            }
        }

        private static string CreateProject(string label)
        {
            string root = Path.Combine(
                Path.GetTempPath(),
                "unitas-u3-" + label + "-" + Guid.NewGuid().ToString("N"));
            Directory.CreateDirectory(Path.Combine(root, "Assets"));
            return root;
        }

        private static void DeleteDirectory(string directory)
        {
            if (Directory.Exists(directory))
            {
                Directory.Delete(directory, true);
            }
        }

        private static void RegisterSyntheticMutator(McpHost host, ChangeSetStore store)
        {
            host.RegisterTool(
                new ToolDefinition(
                    "test.mutate",
                    "Synthetic mutating command held under the review policy",
                    new JsonObject
                    {
                        ["type"] = "object",
                        ["properties"] = new JsonObject
                        {
                            ["value"] = new JsonObject
                            {
                                ["type"] = "string",
                            },
                        },
                    },
                    Scope.ReadProject),
                arguments => ChangeReviewTools.HoldMutation(host, store, "test.mutate", arguments));
        }

        private static JsonObject ContentOf(JsonNode envelope)
        {
            return (JsonObject)((JsonObject)envelope)["content"]!;
        }

        private static List<AuditRecord> TransitionRecords(McpHost host)
        {
            List<AuditRecord> transitions = new List<AuditRecord>();
            foreach (AuditRecord record in host.Audit.Records)
            {
                if (record.TargetKind == AuditTargetKind.Operation && record.Scope == Scope.Review)
                {
                    transitions.Add(record);
                }
            }

            return transitions;
        }

        private static bool HasOutcomeFor(McpHost host, string target, string wanted)
        {
            foreach (AuditRecord record in host.Audit.Records)
            {
                if (record.Target == target && record.TargetKind == AuditTargetKind.Tool
                    && record.Outcome.StatusString == wanted)
                {
                    return true;
                }
            }

            return false;
        }
    }
}
