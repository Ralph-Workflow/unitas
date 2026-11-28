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
            PolicyDefaultsRoundTripAndTolerateMalformedValues();
            HoldStagesItemsUnderTheDefaultPolicy();
            AcceptAndRejectItemsThenApproveAppliesOnlyAccepted();
            AutoApplyPolicyAppliesThroughTheRegisteredContinuation();
            AutoApplyWithoutAContinuationIsLoud();
            RiskClassificationCoversTheFourClasses();
            DiffNormalizationMatchesTheOperationShape();
            ItemIdsAreStableAcrossItemsAndSets();
            SetPolicyPersistsToTheProjectFile();
        }

        private static void PolicyDefaultsRoundTripAndTolerateMalformedValues()
        {
            ReviewPolicy policy = ReviewPolicy.Default;
            Check.That("policy: the default requires approval", policy.RequireApproval);
            Check.That("policy: the default does not hold agent writes", !policy.HoldAgentWrites);
            ReviewPolicy absent = ReviewPolicy.FromValue(null);
            Check.That("policy: an absent value falls back to the default", absent.RequireApproval && !absent.HoldAgentWrites);
            ReviewPolicy malformed = ReviewPolicy.FromValue(new JsonObject { ["requireApproval"] = "yes" });
            Check.That("policy: a malformed value falls back to the default", malformed.RequireApproval && !malformed.HoldAgentWrites);
            JsonObject value = new ReviewPolicy(requireApproval: false, holdAgentWrites: true).ToValue();
            Check.That("policy: to-value records requireApproval", !value["requireApproval"]!.GetValue<bool>());
            Check.That("policy: to-value records holdAgentWrites", value["holdAgentWrites"]!.GetValue<bool>());
            ReviewPolicy roundTripped = ReviewPolicy.FromValue(value);
            Check.That(
                "policy: to-value and from-value round-trip",
                !roundTripped.RequireApproval && roundTripped.HoldAgentWrites);
        }

        private static void HoldStagesItemsUnderTheDefaultPolicy()
        {
            FixedClock clock = new FixedClock(5_000);
            using McpHost host = new McpHost(clock);
            ChangeSetStore store = new ChangeSetStore(host);
            ChangeReviewTools.Register(host, store);
            RegisterSyntheticMutator(host, store);
            Driver driver = new Driver();
            RegisterDriver(host, driver);
            int applied = 0;
            store.RegisterApplier("test.mutate", _ =>
            {
                applied++;
                return new JsonObject();
            });
            string agentSession = host.CreateSession(new SessionIdentity("agent-a", "stdio", "person-1"));
            host.CallTool(agentSession, "test.mutate", new JsonObject { ["value"] = "staged" });
            ChangeSet set = store.Get("cs-1");
            Check.Equal("staged: one item per held operation", 1, set.Items.Count);
            ChangeItem item = set.Items[0];
            Check.Equal("staged: the item id is the set id plus a one-based index", "cs-1.1", item.Id);
            Check.That("staged: the item is pending", item.Status == ChangeItemStatus.Pending);
            Check.Equal("staged: the item names the tool", "test.mutate", item.Tool);
            Check.Equal("staged: the continuation is not invoked", 0, applied);
            JsonArray items = (JsonArray)set.ToValue()["items"]!;
            Check.Equal("staged: the payload lists the item", 1, items.Count);
            Check.Equal(
                "staged: the payload reports the item pending",
                "pending",
                ((JsonObject)items[0]!)["status"]!.GetValue<string>());
            Drive(host, agentSession, driver, () => store.AcceptItem("cs-1", "cs-1.1").ToValue());
            Check.That("staged: accept marks the item accepted", item.Status == ChangeItemStatus.Accepted);
            Drive(host, agentSession, driver, () => store.Approve("cs-1").ToValue());
            Check.Equal("staged: approve applies the accepted item", 1, applied);
            Check.That("staged: the approved set is applied", set.Status == ChangeSetStatus.Applied);
        }

        private static void AcceptAndRejectItemsThenApproveAppliesOnlyAccepted()
        {
            FixedClock clock = new FixedClock(6_000);
            string project = CreateProject("item-review");
            try
            {
                using McpHost host = new McpHost(clock);
                ChangeSetStore store = new ChangeSetStore(host);
                AssetTools.Register(host, store);
                Driver driver = new Driver();
                RegisterDriver(host, driver);
                host.OpenProject(project);
                string agentSession = host.CreateSession(new SessionIdentity("agent-a", "stdio", "person-1"));
                host.GrantScope(agentSession, Scope.EditContent, GrantAuthority.Person);
                List<string> appliedPaths = new List<string>();
                store.RegisterApplier("assets.import", operation =>
                {
                    appliedPaths.Add(operation["path"]!.GetValue<string>());
                    return new JsonObject();
                });
                host.CallTool(
                    agentSession,
                    "assets.import",
                    new JsonObject
                    {
                        ["projectPath"] = project,
                        ["path"] = "Assets/Notes.txt",
                        ["text"] = "hello\n",
                    });
                ChangeSet set = store.Get("cs-1");
                Check.Equal("items: the import holds the asset and meta operations", 2, set.Items.Count);
                Check.Equal("items: the first item id", "cs-1.1", set.Items[0].Id);
                Check.Equal("items: the second item id", "cs-1.2", set.Items[1].Id);
                Check.Equal("items: nothing is applied while staged", 0, appliedPaths.Count);
                Drive(host, agentSession, driver, () => store.AcceptItem("cs-1", "cs-1.1").ToValue());
                Drive(host, agentSession, driver, () => store.RejectItem("cs-1", "cs-1.2").ToValue());
                Check.That("items: the accepted item is accepted", set.Items[0].Status == ChangeItemStatus.Accepted);
                Check.That("items: the rejected item is rejected", set.Items[1].Status == ChangeItemStatus.Rejected);
                bool doubleTransitionRefused = false;
                try
                {
                    Drive(host, agentSession, driver, () => store.AcceptItem("cs-1", "cs-1.2").ToValue());
                }
                catch (ChangeItemNotPending)
                {
                    doubleTransitionRefused = true;
                }

                Check.That("items: accepting a rejected item is refused", doubleTransitionRefused);
                bool unknownItemRefused = false;
                try
                {
                    Drive(host, agentSession, driver, () => store.AcceptItem("cs-1", "cs-1.9").ToValue());
                }
                catch (UnknownChangeItem)
                {
                    unknownItemRefused = true;
                }

                Check.That("items: an unknown item id is refused", unknownItemRefused);
                Drive(host, agentSession, driver, () => store.Approve("cs-1").ToValue());
                Check.Equal("items: approve applies only the accepted item", 1, appliedPaths.Count);
                Check.Equal("items: the applied item is the asset", "Assets/Notes.txt", appliedPaths[0]);
                Check.That("items: the approved set is applied", set.Status == ChangeSetStatus.Applied);
                Check.That("items: the rejected item stays rejected", set.Items[1].Status == ChangeItemStatus.Rejected);
            }
            finally
            {
                DeleteDirectory(project);
            }
        }

        private static void AutoApplyPolicyAppliesThroughTheRegisteredContinuation()
        {
            FixedClock clock = new FixedClock(7_000);
            using McpHost host = new McpHost(clock);
            ChangeSetStore store = new ChangeSetStore(host);
            ChangeReviewTools.Register(host, store);
            RegisterSyntheticMutator(host, store);
            store.SetPolicy(new ReviewPolicy(requireApproval: false, holdAgentWrites: false));
            string? appliedValue = null;
            store.RegisterApplier("test.mutate", operation =>
            {
                appliedValue = ((JsonObject)operation["arguments"]!)["value"]!.GetValue<string>();
                return new JsonObject();
            });
            string agentSession = host.CreateSession(new SessionIdentity("agent-a", "stdio", "person-1"));
            host.CallTool(agentSession, "test.mutate", new JsonObject { ["value"] = "automatic" });
            Check.Equal("auto: the continuation ran at hold time", "automatic", appliedValue!);
            ChangeItem item = store.Get("cs-1").Items[0];
            Check.That("auto: the item ends accepted", item.Status == ChangeItemStatus.Accepted);
        }

        private static void AutoApplyWithoutAContinuationIsLoud()
        {
            FixedClock clock = new FixedClock(8_000);
            using McpHost host = new McpHost(clock);
            ChangeSetStore store = new ChangeSetStore(host);
            ChangeReviewTools.Register(host, store);
            RegisterSyntheticMutator(host, store);
            store.SetPolicy(new ReviewPolicy(requireApproval: false, holdAgentWrites: false));
            string agentSession = host.CreateSession(new SessionIdentity("agent-a", "stdio", "person-1"));
            bool refused = false;
            try
            {
                host.CallTool(agentSession, "test.mutate", new JsonObject { ["value"] = "automatic" });
            }
            catch (McpError)
            {
                refused = true;
            }

            Check.That("auto: a missing continuation is a loud misconfiguration", refused);
        }

        private static void RiskClassificationCoversTheFourClasses()
        {
            FixedClock clock = new FixedClock(9_000);
            using McpHost host = new McpHost(clock);
            ChangeSetStore store = new ChangeSetStore(host);
            Driver driver = new Driver();
            RegisterDriver(host, driver);
            string agentSession = host.CreateSession(new SessionIdentity("agent-a", "stdio", "person-1"));
            Check.Equal(
                "risk: emptying a tests file is test-deletion",
                "test-deletion",
                HoldRisk(driver, store, agentSession, new JsonObject
                {
                    ["tool"] = "project.write_file",
                    ["path"] = "Assets/Editor/FooTests.cs",
                    ["before"] = "class FooTests {}\n",
                    ["after"] = "",
                }));
            Check.Equal(
                "risk: deleting under a tests directory is test-deletion",
                "test-deletion",
                HoldRisk(driver, store, agentSession, new JsonObject
                {
                    ["tool"] = "asset.delete",
                    ["path"] = "Assets/Tests/Helper.cs",
                    ["before"] = "class Helper {}\n",
                }));
            Check.Equal(
                "risk: widening a tolerance property is tolerance-widening",
                "tolerance-widening",
                HoldRisk(driver, store, agentSession, new JsonObject
                {
                    ["tool"] = "scene.set_property",
                    ["scene"] = "Assets/Scenes/Main.unity",
                    ["path"] = "m_Tolerance",
                    ["before"] = "0.01",
                    ["after"] = "0.5",
                }));
            Check.Equal(
                "risk: editing a script is script-edit",
                "script-edit",
                HoldRisk(driver, store, agentSession, new JsonObject
                {
                    ["tool"] = "project.write_file",
                    ["path"] = "Assets/Scripts/Mover.cs",
                    ["before"] = "class Mover {}\n",
                    ["after"] = "class Mover { }\n",
                }));
            Check.Equal(
                "risk: a scene value edit is content-edit",
                "content-edit",
                HoldRisk(driver, store, agentSession, new JsonObject
                {
                    ["tool"] = "scene.set_property",
                    ["scene"] = "Assets/Scenes/Main.unity",
                    ["path"] = "m_Name",
                    ["before"] = "Old",
                    ["after"] = "New",
                }));
            Check.Equal(
                "risk: the severity order lists test-deletion first",
                "test-deletion",
                ChangeRisks.AsString(ChangeRisks.SeverityOrder[0]));
            Check.Equal(
                "risk: the severity order lists tolerance-widening second",
                "tolerance-widening",
                ChangeRisks.AsString(ChangeRisks.SeverityOrder[1]));
        }

        private static void DiffNormalizationMatchesTheOperationShape()
        {
            FixedClock clock = new FixedClock(10_000);
            using McpHost host = new McpHost(clock);
            ChangeSetStore store = new ChangeSetStore(host);
            Driver driver = new Driver();
            RegisterDriver(host, driver);
            string agentSession = host.CreateSession(new SessionIdentity("agent-a", "stdio", "person-1"));
            JsonObject fileDiff = HoldDiff(driver, store, agentSession, new JsonObject
            {
                ["tool"] = "project.write_file",
                ["path"] = "Assets/Notes.md",
                ["before"] = "first\n",
                ["after"] = "second\n",
            });
            Check.Equal("diff: a file write normalizes to text", "text", fileDiff["kind"]!.GetValue<string>());
            Check.Equal("diff: the text diff names the path", "Assets/Notes.md", fileDiff["path"]!.GetValue<string>());
            Check.Equal("diff: the text diff keeps before", "first\n", fileDiff["before"]!.GetValue<string>());
            Check.Equal("diff: the text diff keeps after", "second\n", fileDiff["after"]!.GetValue<string>());
            JsonObject propertyDiff = HoldDiff(driver, store, agentSession, new JsonObject
            {
                ["tool"] = "scene.set_property",
                ["scene"] = "Assets/Scenes/Main.unity",
                ["path"] = "m_Name",
                ["before"] = "Old",
                ["after"] = "New",
            });
            Check.Equal("diff: a property op normalizes to properties", "properties", propertyDiff["kind"]!.GetValue<string>());
            Check.Equal(
                "diff: the properties diff names the scene",
                "Assets/Scenes/Main.unity",
                propertyDiff["path"]!.GetValue<string>());
            JsonObject property = (JsonObject)((JsonArray)propertyDiff["properties"]!)[0]!;
            Check.Equal("diff: the property entry names the property", "m_Name", property["name"]!.GetValue<string>());
            Check.Equal("diff: the property entry keeps before", "Old", property["before"]!.GetValue<string>());
            Check.Equal("diff: the property entry keeps after", "New", property["after"]!.GetValue<string>());
        }

        private static void ItemIdsAreStableAcrossItemsAndSets()
        {
            FixedClock clock = new FixedClock(11_000);
            using McpHost host = new McpHost(clock);
            ChangeSetStore store = new ChangeSetStore(host);
            Driver driver = new Driver();
            RegisterDriver(host, driver);
            string agentSession = host.CreateSession(new SessionIdentity("agent-a", "stdio", "person-1"));
            Drive(host, agentSession, driver, () => store.Hold("agent-a", new JsonObject { ["tool"] = "test.mutate" }).ToValue());
            Drive(host, agentSession, driver, () => store.Hold("agent-a", new JsonObject { ["tool"] = "test.mutate" }).ToValue());
            Check.Equal("ids: the first set numbers its item from one", "cs-1.1", store.Get("cs-1").Items[0].Id);
            Check.Equal("ids: the second set restarts at one", "cs-2.1", store.Get("cs-2").Items[0].Id);
        }

        private static void SetPolicyPersistsToTheProjectFile()
        {
            FixedClock clock = new FixedClock(12_000);
            string project = CreateProject("policy");
            try
            {
                using McpHost host = new McpHost(clock);
                host.OpenProject(project);
                ChangeSetStore store = new ChangeSetStore(host);
                Check.That("persist: an absent policy file loads the default", store.Policy.RequireApproval);
                store.SetPolicy(new ReviewPolicy(requireApproval: false, holdAgentWrites: true));
                string policyFile = Path.Combine(project, ".unitas", "review-policy.json");
                Check.That("persist: the policy file is written under the project", File.Exists(policyFile));
                JsonObject persisted = (JsonObject)JsonNode.Parse(File.ReadAllText(policyFile))!;
                Check.That("persist: the file holds requireApproval", !persisted["requireApproval"]!.GetValue<bool>());
                Check.That("persist: the file holds holdAgentWrites", persisted["holdAgentWrites"]!.GetValue<bool>());
                ChangeSetStore reopened = new ChangeSetStore(host);
                Check.That(
                    "persist: a new store over the same project loads the saved policy",
                    !reopened.Policy.RequireApproval && reopened.Policy.HoldAgentWrites);
            }
            finally
            {
                DeleteDirectory(project);
            }
        }

        private static string HoldRisk(Driver driver, ChangeSetStore store, string session, JsonObject operation)
        {
            return (string)HoldValue(driver, store, session, operation, "risk")!;
        }

        private static JsonObject HoldDiff(Driver driver, ChangeSetStore store, string session, JsonObject operation)
        {
            return (JsonObject)HoldValue(driver, store, session, operation, "diff");
        }

        private static JsonNode HoldValue(Driver driver, ChangeSetStore store, string session, JsonObject operation, string field)
        {
            JsonObject? payload = null;
            Drive(driver.Host, session, driver, () =>
            {
                payload = store.Hold("agent-a", operation).ToValue();
                return payload;
            });
            return ((JsonObject)((JsonArray)payload!["items"]!)[0]!)[field]!;
        }

        private sealed class Driver
        {
            public Func<ToolOutput>? Next;

            public McpHost Host = null!;
        }

        private static void RegisterDriver(McpHost host, Driver driver)
        {
            driver.Host = host;
            host.RegisterTool(
                new ToolDefinition(
                    "test.drive",
                    "Synthetic driver that runs store calls inside a session",
                    new JsonObject
                    {
                        ["type"] = "object",
                    },
                    Scope.ReadProject),
                _ => driver.Next!());
        }

        private static void Drive(McpHost host, string session, Driver driver, Func<JsonObject> action)
        {
            driver.Next = () => ToolOutput.Untrusted("test://driver", action());
            host.CallTool(session, "test.drive", new JsonObject());
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
            ChangeSetStore store = new ChangeSetStore(host);
            Driver driver = new Driver();
            RegisterDriver(host, driver);
            store.RegisterApplier("test.mutate", operation =>
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

            Drive(host, personSession, driver, () => store.AcceptItem("cs-1", "cs-1.1").ToValue());
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
            Check.Equal("approve: hold, accept, approve, apply transitions audited", 4, transitions.Count);
            Check.Equal("approve: first transition is hold", "changes.hold", transitions[0].Target);
            Check.Equal("approve: second transition is accept-item", "changes.accept_item", transitions[1].Target);
            Check.Equal("approve: third transition is approve", "changes.approve", transitions[2].Target);
            Check.Equal("approve: fourth transition is apply", "changes.apply", transitions[3].Target);
            Check.That(
                "approve: transitions by the person identity",
                transitions[2].Identity.Client == "http" && transitions[3].Identity.Client == "http");
            Check.That(
                "approve: refused agent attempt audited as failed tool call",
                HasOutcomeFor(host, "changes.approve", "failed"));
        }

        private static void PersonRejectRefusesHeldMutationAndClosesIt()
        {
            FixedClock clock = new FixedClock(3_000);
            using McpHost host = new McpHost(clock, PersonCredential.FromToken("person-token"));
            bool applied = false;
            ChangeSetStore store = new ChangeSetStore(host);
            store.RegisterApplier("test.mutate", _ =>
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

        private static void UseAutoApplyPolicyForPreReviewGateRestoreExpectations(string project)
        {
            string directory = Path.Combine(project, ".unitas");
            Directory.CreateDirectory(directory);
            File.WriteAllText(
                Path.Combine(directory, "review-policy.json"),
                new ReviewPolicy(requireApproval: false, holdAgentWrites: false).ToValue().ToJsonString());
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
                store.RegisterApplier("test.mutate", operation => operation);
                host.OpenProject(project);
                string agentSession = host.CreateSession(new SessionIdentity("agent-a", "stdio", "person-1"));
                host.GrantScope(agentSession, Scope.EditContent, GrantAuthority.Person);
                string personSession = host.CreateSession(new SessionIdentity("person-ui", "http", "person-1"));

                File.WriteAllText(Path.Combine(project, "Assets", "Existing.txt"), "original\n");
                UseAutoApplyPolicyForPreReviewGateRestoreExpectations(project);
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
