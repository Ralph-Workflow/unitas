using System;
using System.Collections.Generic;
using System.IO;
using System.Reflection;
using System.Text.Json.Nodes;
using Unitas.Editor.Mcp;

namespace Unitas.Editor.Tests
{
    // Registry-level EDT-008 review-gate tests against the U2b tool surface
    // (changes.accept_item, changes.reject_item, changes.revert,
    // changes.policy.get/set, risk-ordered changes.show). These encode the
    // pinned contracts and stay red until U2b lands.
    public static class ReviewPolicyToolsTests
    {
        private const string PersonToken = "person-token";

        public static void Run()
        {
            DefaultPolicyHoldsAgentWriteUntilPersonAcceptsAndApproves();
            PersonRejectsOneOfTwoWritesAndApproveAppliesOnlyTheAccepted();
            ShowListsRiskFlaggedItemsFirst();
            ShowPresentsPropertyShapedItemsAsPerPropertyDiffs();
            HoldAgentWritesRefusesAgentWritesAndNamesThePolicyTool();
            PersonOnlyReviewToolsRefuseAgentSessions();
            PolicySetPersistsAndAFreshHostReadsItBack();
            RevertRestoresTheFilesTheApprovedSetChanged();
        }

        // Case 1: default policy — an agent write is held, a person accepts the
        // item and approves the set, and only then does the file land on disk.
        private static void DefaultPolicyHoldsAgentWriteUntilPersonAcceptsAndApproves()
        {
            string project = CreateProject("default-policy");
            try
            {
                using McpHost host = new McpHost(new FixedClock(20_000), PersonCredential.FromToken(PersonToken));
                host.OpenProject(project);
                ChangeSetStore store = new ChangeSetStore(host);
                ProjectTools.Register(host, store);
                ChangeReviewTools.Register(host, store);
                string agentSession = host.CreateSession(new SessionIdentity("agent-a", "stdio", "person-1"));
                host.GrantScope(agentSession, Scope.EditContent, GrantAuthority.Person);
                string personSession = host.CreateSession(new SessionIdentity("person-ui", "http", "person-1"));

                JsonObject held = ContentOf(host.CallTool(
                    agentSession,
                    "project.write_file",
                    new JsonObject
                    {
                        ["projectPath"] = project,
                        ["path"] = "Assets/Notes.txt",
                        ["content"] = "hello\n",
                    }));
                Check.Equal(
                    "default: the agent write is held for review",
                    "pending-review",
                    held["status"]!.GetValue<string>());
                string changeId = held["changeId"]!.GetValue<string>();
                Check.That("default: the held write names its change set", changeId.Length > 0);
                Check.That("default: the held write carries the change set", held["changeSet"] is JsonObject);
                Check.That(
                    "default: the file is not written while held",
                    !File.Exists(Path.Combine(project, "Assets", "Notes.txt")));

                JsonObject shown = ContentOf(host.CallTool(
                    personSession,
                    "changes.show",
                    new JsonObject { ["id"] = changeId }));
                JsonArray items = (JsonArray)shown["items"]!;
                Check.Equal("default: show lists the one held item", 1, items.Count);
                JsonObject item = (JsonObject)items[0]!;
                Check.Equal("default: the item names the writing tool", "project.write_file", item["tool"]!.GetValue<string>());
                Check.Equal("default: the item names the path", "Assets/Notes.txt", item["path"]!.GetValue<string>());
                Check.Equal("default: the item is pending", "pending", item["status"]!.GetValue<string>());
                Check.Equal("default: a plain text write is a content edit", "content-edit", item["risk"]!.GetValue<string>());
                JsonObject diff = (JsonObject)item["diff"]!;
                Check.Equal("default: the diff is a text diff", "text", diff["kind"]!.GetValue<string>());
                Check.Equal("default: the text diff keeps the after content", "hello\n", diff["after"]!.GetValue<string>());

                host.CallTool(
                    personSession,
                    "changes.accept_item",
                    new JsonObject
                    {
                        ["id"] = changeId,
                        ["itemId"] = changeId + ".1",
                        ["personCredential"] = PersonToken,
                    });
                JsonObject approved = ContentOf(host.CallTool(
                    personSession,
                    "changes.approve",
                    new JsonObject
                    {
                        ["id"] = changeId,
                        ["personCredential"] = PersonToken,
                    }));
                Check.Equal("default: approve applies the set", "applied", approved["status"]!.GetValue<string>());
                Check.Equal(
                    "default: the approved write lands on disk",
                    "hello\n",
                    File.ReadAllText(Path.Combine(project, "Assets", "Notes.txt")));
            }
            finally
            {
                DeleteDirectory(project);
            }
        }

        // Case 2: per-item reject — of two held writes, the person accepts one
        // and rejects the other; approving both sets applies only the accepted one.
        private static void PersonRejectsOneOfTwoWritesAndApproveAppliesOnlyTheAccepted()
        {
            string project = CreateProject("per-item-reject");
            try
            {
                using McpHost host = new McpHost(new FixedClock(21_000), PersonCredential.FromToken(PersonToken));
                host.OpenProject(project);
                ChangeSetStore store = new ChangeSetStore(host);
                ProjectTools.Register(host, store);
                ChangeReviewTools.Register(host, store);
                string agentSession = host.CreateSession(new SessionIdentity("agent-a", "stdio", "person-1"));
                host.GrantScope(agentSession, Scope.EditContent, GrantAuthority.Person);
                string personSession = host.CreateSession(new SessionIdentity("person-ui", "http", "person-1"));

                string accepted = HoldWrite(host, agentSession, project, "Assets/Kept.txt", "alpha\n");
                string rejected = HoldWrite(host, agentSession, project, "Assets/Dropped.txt", "beta\n");

                host.CallTool(
                    personSession,
                    "changes.accept_item",
                    new JsonObject
                    {
                        ["id"] = accepted,
                        ["itemId"] = accepted + ".1",
                        ["personCredential"] = PersonToken,
                    });
                host.CallTool(
                    personSession,
                    "changes.reject_item",
                    new JsonObject
                    {
                        ["id"] = rejected,
                        ["itemId"] = rejected + ".1",
                        ["personCredential"] = PersonToken,
                    });
                host.CallTool(
                    personSession,
                    "changes.approve",
                    new JsonObject { ["id"] = accepted, ["personCredential"] = PersonToken });
                host.CallTool(
                    personSession,
                    "changes.approve",
                    new JsonObject { ["id"] = rejected, ["personCredential"] = PersonToken });

                Check.Equal(
                    "reject: the accepted write is applied",
                    "alpha\n",
                    File.ReadAllText(Path.Combine(project, "Assets", "Kept.txt")));
                Check.That(
                    "reject: the rejected write never lands",
                    !File.Exists(Path.Combine(project, "Assets", "Dropped.txt")));
            }
            finally
            {
                DeleteDirectory(project);
            }
        }

        // Case 3: risk-first ordering — a set holding a content edit, a
        // tolerance widening, and a test deletion lists the risky items first.
        private static void ShowListsRiskFlaggedItemsFirst()
        {
            string project = CreateProject("risk-order");
            try
            {
                using McpHost host = new McpHost(new FixedClock(22_000), PersonCredential.FromToken(PersonToken));
                host.OpenProject(project);
                ChangeSetStore store = new ChangeSetStore(host);
                ChangeReviewTools.Register(host, store);
                Driver driver = new Driver();
                RegisterDriver(host, driver);
                string agentSession = host.CreateSession(new SessionIdentity("agent-a", "stdio", "person-1"));
                string personSession = host.CreateSession(new SessionIdentity("person-ui", "http", "person-1"));

                ChangeSet? held = null;
                Drive(host, agentSession, driver, () =>
                {
                    held = store.Hold("agent-a", new JsonObject
                    {
                        ["tool"] = "project.write_file",
                        ["path"] = "Assets/Notes.md",
                        ["before"] = "first\n",
                        ["after"] = "second\n",
                    });
                    return held.ToValue();
                });
                // The test assembly has no InternalsVisibleTo, and no public tool
                // can stage a riskier item after a content edit inside one set
                // (AssetTools' two-item import always puts the riskiest shape
                // first), so the fixture appends operations the way AssetTools does.
                AddOperation(held!, new JsonObject
                {
                    ["tool"] = "scene.set_property",
                    ["scene"] = "Assets/Scenes/Main.unity",
                    ["path"] = "m_Tolerance",
                    ["before"] = "0.01",
                    ["after"] = "0.5",
                });
                AddOperation(held!, new JsonObject
                {
                    ["tool"] = "project.write_file",
                    ["path"] = "Assets/Editor/FooTests.cs",
                    ["before"] = "class FooTests {}\n",
                    ["after"] = "",
                });

                JsonObject shown = ContentOf(host.CallTool(
                    personSession,
                    "changes.show",
                    new JsonObject { ["id"] = held!.Id }));
                JsonArray items = (JsonArray)shown["items"]!;
                Check.Equal("order: show lists every item", 3, items.Count);
                Check.Equal(
                    "order: the test deletion is listed first",
                    "test-deletion",
                    ((JsonObject)items[0]!)["risk"]!.GetValue<string>());
                Check.Equal(
                    "order: the tolerance widening is listed second",
                    "tolerance-widening",
                    ((JsonObject)items[1]!)["risk"]!.GetValue<string>());
                Check.Equal(
                    "order: the content edit is listed last",
                    "content-edit",
                    ((JsonObject)items[2]!)["risk"]!.GetValue<string>());
                Check.Equal(
                    "order: item ids stay stable through the reordering",
                    held.Id + ".3",
                    ((JsonObject)items[0]!)["id"]!.GetValue<string>());
            }
            finally
            {
                DeleteDirectory(project);
            }
        }

        // Case 4: a held scene.set_property-shaped operation shows a
        // per-property before/after diff through changes.show.
        private static void ShowPresentsPropertyShapedItemsAsPerPropertyDiffs()
        {
            string project = CreateProject("property-diff");
            try
            {
                using McpHost host = new McpHost(new FixedClock(23_000), PersonCredential.FromToken(PersonToken));
                host.OpenProject(project);
                ChangeSetStore store = new ChangeSetStore(host);
                ChangeReviewTools.Register(host, store);
                Driver driver = new Driver();
                RegisterDriver(host, driver);
                string agentSession = host.CreateSession(new SessionIdentity("agent-a", "stdio", "person-1"));
                string personSession = host.CreateSession(new SessionIdentity("person-ui", "http", "person-1"));

                string changeId = string.Empty;
                Drive(host, agentSession, driver, () =>
                {
                    ChangeSet set = store.Hold("agent-a", new JsonObject
                    {
                        ["tool"] = "scene.set_property",
                        ["scene"] = "Assets/Scenes/Main.unity",
                        ["path"] = "m_Name",
                        ["before"] = "Old",
                        ["after"] = "New",
                    });
                    changeId = set.Id;
                    return set.ToValue();
                });

                JsonObject shown = ContentOf(host.CallTool(
                    personSession,
                    "changes.show",
                    new JsonObject { ["id"] = changeId }));
                JsonObject item = (JsonObject)((JsonArray)shown["items"]!)[0]!;
                JsonObject diff = (JsonObject)item["diff"]!;
                Check.Equal("property: the diff is property-shaped", "properties", diff["kind"]!.GetValue<string>());
                Check.Equal(
                    "property: the diff names the scene",
                    "Assets/Scenes/Main.unity",
                    diff["path"]!.GetValue<string>());
                JsonObject property = (JsonObject)((JsonArray)diff["properties"]!)[0]!;
                Check.Equal("property: the entry names the property", "m_Name", property["name"]!.GetValue<string>());
                Check.Equal("property: the entry keeps the before value", "Old", property["before"]!.GetValue<string>());
                Check.Equal("property: the entry keeps the after value", "New", property["after"]!.GetValue<string>());
            }
            finally
            {
                DeleteDirectory(project);
            }
        }

        // Case 5: holdAgentWrites — under the strict policy an agent write is
        // refused outright, the refusal names the person-only policy tool, and
        // nothing is staged.
        private static void HoldAgentWritesRefusesAgentWritesAndNamesThePolicyTool()
        {
            string project = CreateProject("hold-agent-writes");
            try
            {
                using McpHost host = new McpHost(new FixedClock(24_000), PersonCredential.FromToken(PersonToken));
                host.OpenProject(project);
                ChangeSetStore store = new ChangeSetStore(host);
                ProjectTools.Register(host, store);
                ChangeReviewTools.Register(host, store);
                string agentSession = host.CreateSession(new SessionIdentity("agent-a", "stdio", "person-1"));
                host.GrantScope(agentSession, Scope.EditContent, GrantAuthority.Person);
                string personSession = host.CreateSession(new SessionIdentity("person-ui", "http", "person-1"));

                host.CallTool(
                    personSession,
                    "changes.policy.set",
                    new JsonObject
                    {
                        ["holdAgentWrites"] = true,
                        ["personCredential"] = PersonToken,
                    });

                McpError? refused = Catch(() => host.CallTool(
                    agentSession,
                    "project.write_file",
                    new JsonObject
                    {
                        ["projectPath"] = project,
                        ["path"] = "Assets/Blocked.txt",
                        ["content"] = "blocked\n",
                    }));
                Check.That(
                    "hold: the agent write is refused under holdAgentWrites",
                    refused != null
                    && refused.Message.Contains("changes.policy.set", StringComparison.Ordinal));
                Check.That(
                    "hold: the refused write never lands",
                    !File.Exists(Path.Combine(project, "Assets", "Blocked.txt")));
                JsonObject listed = ContentOf(host.CallTool(agentSession, "changes.list", new JsonObject()));
                Check.Equal(
                    "hold: nothing is staged under holdAgentWrites",
                    0,
                    ((JsonArray)listed["changeSets"]!).Count);
            }
            finally
            {
                DeleteDirectory(project);
            }
        }

        // Case 6: the review-mutation tools are person-only — an agent session
        // over stdio is refused even with the right token, and a person session
        // without the token is refused too.
        private static void PersonOnlyReviewToolsRefuseAgentSessions()
        {
            string project = CreateProject("person-only");
            try
            {
                using McpHost host = new McpHost(new FixedClock(25_000), PersonCredential.FromToken(PersonToken));
                host.OpenProject(project);
                ChangeSetStore store = new ChangeSetStore(host);
                ProjectTools.Register(host, store);
                ChangeReviewTools.Register(host, store);
                string agentSession = host.CreateSession(new SessionIdentity("agent-a", "stdio", "person-1"));
                host.GrantScope(agentSession, Scope.EditContent, GrantAuthority.Person);
                string personSession = host.CreateSession(new SessionIdentity("person-ui", "http", "person-1"));

                string changeId = HoldWrite(host, agentSession, project, "Assets/Guarded.txt", "guarded\n");

                (string Name, JsonObject Arguments)[] personOnly =
                {
                    ("changes.accept_item", new JsonObject
                    {
                        ["id"] = changeId,
                        ["itemId"] = changeId + ".1",
                        ["personCredential"] = PersonToken,
                    }),
                    ("changes.reject_item", new JsonObject
                    {
                        ["id"] = changeId,
                        ["itemId"] = changeId + ".1",
                        ["personCredential"] = PersonToken,
                    }),
                    ("changes.revert", new JsonObject
                    {
                        ["id"] = changeId,
                        ["personCredential"] = PersonToken,
                    }),
                    ("changes.policy.set", new JsonObject
                    {
                        ["requireApproval"] = false,
                        ["personCredential"] = PersonToken,
                    }),
                };
                foreach ((string name, JsonObject arguments) in personOnly)
                {
                    McpError.PersonCredentialRequired? refusal = null;
                    try
                    {
                        host.CallTool(agentSession, name, arguments);
                    }
                    catch (McpError.PersonCredentialRequired error)
                    {
                        refusal = error;
                    }

                    Check.That(
                        "person-only: an agent over stdio can never call " + name + ", even with the right token",
                        refusal != null);
                }

                bool missingTokenRefused = false;
                try
                {
                    host.CallTool(
                        personSession,
                        "changes.accept_item",
                        new JsonObject
                        {
                            ["id"] = changeId,
                            ["itemId"] = changeId + ".1",
                        });
                }
                catch (McpError.PersonCredentialRequired)
                {
                    missingTokenRefused = true;
                }

                Check.That("person-only: a person without the token is refused", missingTokenRefused);
                JsonObject shown = ContentOf(host.CallTool(
                    personSession,
                    "changes.show",
                    new JsonObject { ["id"] = changeId }));
                Check.Equal(
                    "person-only: the refused attempts leave the item pending",
                    "pending",
                    ((JsonObject)((JsonArray)shown["items"]!)[0]!)["status"]!.GetValue<string>());
            }
            finally
            {
                DeleteDirectory(project);
            }
        }

        // Case 7: changes.policy.set persists to .unitas/review-policy.json and
        // a fresh host over the same project reads it back via changes.policy.get.
        private static void PolicySetPersistsAndAFreshHostReadsItBack()
        {
            string project = CreateProject("policy-persist");
            try
            {
                using (McpHost host = new McpHost(new FixedClock(26_000), PersonCredential.FromToken(PersonToken)))
                {
                    host.OpenProject(project);
                    ChangeSetStore store = new ChangeSetStore(host);
                    ChangeReviewTools.Register(host, store);
                    string personSession = host.CreateSession(new SessionIdentity("person-ui", "http", "person-1"));

                    JsonObject before = ContentOf(host.CallTool(personSession, "changes.policy.get", new JsonObject()));
                    Check.That("persist: the default policy requires approval", before["requireApproval"]!.GetValue<bool>());
                    Check.That("persist: the default policy does not hold agent writes", !before["holdAgentWrites"]!.GetValue<bool>());

                    host.CallTool(
                        personSession,
                        "changes.policy.set",
                        new JsonObject
                        {
                            ["requireApproval"] = false,
                            ["personCredential"] = PersonToken,
                        });
                    string policyFile = Path.Combine(project, ".unitas", "review-policy.json");
                    Check.That("persist: the policy file lands under the project", File.Exists(policyFile));
                    JsonObject persisted = (JsonObject)JsonNode.Parse(File.ReadAllText(policyFile))!;
                    Check.That("persist: the file records requireApproval false", !persisted["requireApproval"]!.GetValue<bool>());
                }

                using McpHost reopened = new McpHost(new FixedClock(27_000), PersonCredential.FromToken(PersonToken));
                reopened.OpenProject(project);
                ChangeSetStore reopenedStore = new ChangeSetStore(reopened);
                ChangeReviewTools.Register(reopened, reopenedStore);
                string reopenedPerson = reopened.CreateSession(new SessionIdentity("person-ui", "http", "person-1"));
                JsonObject loaded = ContentOf(reopened.CallTool(reopenedPerson, "changes.policy.get", new JsonObject()));
                Check.That("persist: a fresh host reads requireApproval false", !loaded["requireApproval"]!.GetValue<bool>());
                Check.That("persist: a fresh host keeps holdAgentWrites false", !loaded["holdAgentWrites"]!.GetValue<bool>());
            }
            finally
            {
                DeleteDirectory(project);
            }
        }

        // Case 8: changes.revert — after approval applies a held set, a person
        // reverts it and the files return to their pre-write state (modified
        // files restored, added files deleted).
        private static void RevertRestoresTheFilesTheApprovedSetChanged()
        {
            string project = CreateProject("revert");
            try
            {
                using McpHost host = new McpHost(new FixedClock(28_000), PersonCredential.FromToken(PersonToken));
                host.OpenProject(project);
                ChangeSetStore store = new ChangeSetStore(host);
                ProjectTools.Register(host, store);
                ChangeReviewTools.Register(host, store);
                string agentSession = host.CreateSession(new SessionIdentity("agent-a", "stdio", "person-1"));
                host.GrantScope(agentSession, Scope.EditContent, GrantAuthority.Person);
                string personSession = host.CreateSession(new SessionIdentity("person-ui", "http", "person-1"));

                File.WriteAllText(Path.Combine(project, "Assets", "Existing.txt"), "original\n");
                string modified = HoldWrite(host, agentSession, project, "Assets/Existing.txt", "changed\n");
                string added = HoldWrite(host, agentSession, project, "Assets/Added.txt", "added\n");
                ApproveAll(host, personSession, modified);
                ApproveAll(host, personSession, added);
                Check.Equal(
                    "revert: the approved write lands first",
                    "changed\n",
                    File.ReadAllText(Path.Combine(project, "Assets", "Existing.txt")));
                Check.That(
                    "revert: the approved new file lands first",
                    File.Exists(Path.Combine(project, "Assets", "Added.txt")));

                host.CallTool(
                    personSession,
                    "changes.revert",
                    new JsonObject { ["id"] = modified, ["personCredential"] = PersonToken });
                host.CallTool(
                    personSession,
                    "changes.revert",
                    new JsonObject { ["id"] = added, ["personCredential"] = PersonToken });

                Check.Equal(
                    "revert: the modified file returns to its before content",
                    "original\n",
                    File.ReadAllText(Path.Combine(project, "Assets", "Existing.txt")));
                Check.That(
                    "revert: the added file is deleted again",
                    !File.Exists(Path.Combine(project, "Assets", "Added.txt")));
            }
            finally
            {
                DeleteDirectory(project);
            }
        }

        private static string HoldWrite(McpHost host, string agentSession, string project, string path, string content)
        {
            JsonObject held = ContentOf(host.CallTool(
                agentSession,
                "project.write_file",
                new JsonObject
                {
                    ["projectPath"] = project,
                    ["path"] = path,
                    ["content"] = content,
                }));
            return held["changeId"]!.GetValue<string>();
        }

        private static void ApproveAll(McpHost host, string personSession, string changeId)
        {
            host.CallTool(
                personSession,
                "changes.accept_item",
                new JsonObject
                {
                    ["id"] = changeId,
                    ["itemId"] = changeId + ".1",
                    ["personCredential"] = PersonToken,
                });
            host.CallTool(
                personSession,
                "changes.approve",
                new JsonObject
                {
                    ["id"] = changeId,
                    ["personCredential"] = PersonToken,
                });
        }

        private static void AddOperation(ChangeSet set, JsonObject operation)
        {
            typeof(ChangeSet)
                .GetMethod("AddOperation", BindingFlags.Instance | BindingFlags.NonPublic)!
                .Invoke(set, new object[] { operation });
        }

        private static string CreateProject(string label)
        {
            string root = Path.Combine(
                Path.GetTempPath(),
                "unitas-u3-review-" + label + "-" + Guid.NewGuid().ToString("N"));
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

        private static JsonObject ContentOf(JsonNode envelope)
        {
            return (JsonObject)((JsonObject)envelope)["content"]!;
        }

        private static McpError? Catch(Func<JsonNode> action)
        {
            try
            {
                action();
                return null;
            }
            catch (McpError error)
            {
                return error;
            }
        }
    }
}
