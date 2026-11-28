using System;
using System.Collections.Generic;
using System.IO;
using System.Text.Json.Nodes;
using Unitas.Editor.Mcp;

namespace Unitas.Editor.Tests
{
    public static class ChangeReviewToolSurfaceTests
    {
        private const string PersonToken = "person-token";

        public static void Run()
        {
            PersonOnlySurfaceRefusesAgentSessions();
            PersonAcceptItemThenApproveAppliesOnlyThatItem();
            PolicySetMergesOverTheCurrentPolicyAndPersists();
            RevertAfterApproveRestoresTheFileContent();
            ShowAndListRenderItemsRiskFirst();
        }

        private static void PersonOnlySurfaceRefusesAgentSessions()
        {
            using McpHost host = new McpHost(new FixedClock(30_000), PersonCredential.FromToken(PersonToken));
            ChangeSetStore store = new ChangeSetStore(host);
            ChangeReviewTools.Register(host, store);
            Driver driver = new Driver();
            RegisterDriver(host, driver);
            string agentSession = host.CreateSession(new SessionIdentity("agent-a", "stdio", "person-1"));
            string personSession = host.CreateSession(new SessionIdentity("person-ui", "http", "person-1"));
            Drive(host, agentSession, driver, () => store.Hold("agent-a", new JsonObject
            {
                ["tool"] = "test.mutate",
                ["path"] = "Assets/Guarded.txt",
                ["before"] = string.Empty,
                ["after"] = "guarded\n",
            }).ToValue());

            (string Name, JsonObject Arguments)[] calls =
            {
                ("changes.accept_item", new JsonObject
                {
                    ["id"] = "cs-1",
                    ["itemId"] = "cs-1.1",
                }),
                ("changes.reject_item", new JsonObject
                {
                    ["id"] = "cs-1",
                    ["itemId"] = "cs-1.1",
                }),
                ("changes.revert", new JsonObject
                {
                    ["id"] = "cs-1",
                }),
                ("changes.policy.set", new JsonObject
                {
                    ["requireApproval"] = false,
                }),
            };
            foreach ((string name, JsonObject arguments) in calls)
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
                    "surface: an agent session without a person credential cannot call " + name,
                    refusal != null);
            }

            McpError.PersonCredentialRequired? missingToken = null;
            try
            {
                host.CallTool(
                    personSession,
                    "changes.accept_item",
                    new JsonObject
                    {
                        ["id"] = "cs-1",
                        ["itemId"] = "cs-1.1",
                    });
            }
            catch (McpError.PersonCredentialRequired error)
            {
                missingToken = error;
            }

            Check.That("surface: a person session without the token is refused", missingToken != null);
            Check.That(
                "surface: the refused attempts leave the item pending",
                store.Get("cs-1").Items[0].Status == ChangeItemStatus.Pending);
        }

        private static void PersonAcceptItemThenApproveAppliesOnlyThatItem()
        {
            using McpHost host = new McpHost(new FixedClock(31_000), PersonCredential.FromToken(PersonToken));
            ChangeSetStore store = new ChangeSetStore(host);
            ChangeReviewTools.Register(host, store);
            Driver driver = new Driver();
            RegisterDriver(host, driver);
            List<string> applied = new List<string>();
            store.RegisterApplier("test.mutate", operation =>
            {
                applied.Add(operation["path"]?.GetValue<string>() ?? string.Empty);
                return new JsonObject();
            });
            string agentSession = host.CreateSession(new SessionIdentity("agent-a", "stdio", "person-1"));
            string personSession = host.CreateSession(new SessionIdentity("person-ui", "http", "person-1"));
            ChangeSet? held = null;
            Drive(host, agentSession, driver, () =>
            {
                held = store.Hold("agent-a", new JsonObject
                {
                    ["tool"] = "test.mutate",
                    ["path"] = "Assets/Kept.txt",
                    ["before"] = string.Empty,
                    ["after"] = "kept\n",
                });
                return held.ToValue();
            });
            AddOperation(held!, new JsonObject
            {
                ["tool"] = "test.mutate",
                ["path"] = "Assets/Dropped.txt",
                ["before"] = string.Empty,
                ["after"] = "dropped\n",
            });

            JsonObject accepted = ContentOf(host.CallTool(
                personSession,
                "changes.accept_item",
                new JsonObject
                {
                    ["id"] = held!.Id,
                    ["itemId"] = held.Id + ".1",
                    ["personCredential"] = PersonToken,
                }));
            JsonArray acceptedItems = (JsonArray)accepted["items"]!;
            Check.Equal(
                "surface: accept_item reports the item accepted",
                "accepted",
                ((JsonObject)acceptedItems[0]!)["status"]!.GetValue<string>());
            Check.Equal(
                "surface: accept_item leaves the other item pending",
                "pending",
                ((JsonObject)acceptedItems[1]!)["status"]!.GetValue<string>());
            Check.Equal("surface: nothing is applied before approve", 0, applied.Count);

            JsonObject approved = ContentOf(host.CallTool(
                personSession,
                "changes.approve",
                new JsonObject
                {
                    ["id"] = held.Id,
                    ["personCredential"] = PersonToken,
                }));
            Check.Equal("surface: approve applies the set", "applied", approved["status"]!.GetValue<string>());
            Check.Equal("surface: approve applies only the accepted item", 1, applied.Count);
            Check.Equal("surface: the applied item is the accepted one", "Assets/Kept.txt", applied[0]);
        }

        private static void PolicySetMergesOverTheCurrentPolicyAndPersists()
        {
            string project = CreateProject("policy-merge");
            try
            {
                using McpHost host = new McpHost(new FixedClock(32_000), PersonCredential.FromToken(PersonToken));
                host.OpenProject(project);
                ChangeSetStore store = new ChangeSetStore(host);
                ChangeReviewTools.Register(host, store);
                string personSession = host.CreateSession(new SessionIdentity("person-ui", "http", "person-1"));

                JsonObject initial = ContentOf(host.CallTool(personSession, "changes.policy.get", new JsonObject()));
                Check.That("surface: policy.get reports the default requireApproval", initial["requireApproval"]!.GetValue<bool>());
                Check.That("surface: policy.get reports the default holdAgentWrites", !initial["holdAgentWrites"]!.GetValue<bool>());

                JsonObject updated = ContentOf(host.CallTool(
                    personSession,
                    "changes.policy.set",
                    new JsonObject
                    {
                        ["holdAgentWrites"] = true,
                        ["personCredential"] = PersonToken,
                    }));
                Check.That("surface: policy.set applies the given key", updated["holdAgentWrites"]!.GetValue<bool>());
                Check.That("surface: policy.set keeps the absent key", updated["requireApproval"]!.GetValue<bool>());

                string policyFile = Path.Combine(project, ".unitas", "review-policy.json");
                Check.That("surface: policy.set persists the policy file", File.Exists(policyFile));
                JsonObject persisted = (JsonObject)JsonNode.Parse(File.ReadAllText(policyFile))!;
                Check.That(
                    "surface: the persisted policy holds the merged values",
                    persisted["holdAgentWrites"]!.GetValue<bool>() && persisted["requireApproval"]!.GetValue<bool>());

                JsonObject reloaded = ContentOf(host.CallTool(personSession, "changes.policy.get", new JsonObject()));
                Check.That(
                    "surface: policy.get returns the merged policy",
                    reloaded["holdAgentWrites"]!.GetValue<bool>() && reloaded["requireApproval"]!.GetValue<bool>());
            }
            finally
            {
                DeleteDirectory(project);
            }
        }

        private static void RevertAfterApproveRestoresTheFileContent()
        {
            string project = CreateProject("revert-restore");
            try
            {
                using McpHost host = new McpHost(new FixedClock(33_000), PersonCredential.FromToken(PersonToken));
                host.OpenProject(project);
                ChangeSetStore store = new ChangeSetStore(host);
                ChangeReviewTools.Register(host, store);
                Driver driver = new Driver();
                RegisterDriver(host, driver);
                store.RegisterApplier("test.mutate", _ => new JsonObject());
                string agentSession = host.CreateSession(new SessionIdentity("agent-a", "stdio", "person-1"));
                string personSession = host.CreateSession(new SessionIdentity("person-ui", "http", "person-1"));

                File.WriteAllText(Path.Combine(project, "Assets", "Notes.txt"), "changed\n");
                Drive(host, agentSession, driver, () => store.Hold("agent-a", new JsonObject
                {
                    ["tool"] = "test.mutate",
                    ["path"] = "Assets/Notes.txt",
                    ["before"] = "original\n",
                    ["after"] = "changed\n",
                }).ToValue());

                McpError? premature = null;
                try
                {
                    host.CallTool(
                        personSession,
                        "changes.revert",
                        new JsonObject
                        {
                            ["id"] = "cs-1",
                            ["personCredential"] = PersonToken,
                        });
                }
                catch (McpError error)
                {
                    premature = error;
                }

                Check.That(
                    "surface: reverting a set that is not applied is refused",
                    premature is ChangeSetNotApplied);

                host.CallTool(
                    personSession,
                    "changes.accept_item",
                    new JsonObject
                    {
                        ["id"] = "cs-1",
                        ["itemId"] = "cs-1.1",
                        ["personCredential"] = PersonToken,
                    });
                host.CallTool(
                    personSession,
                    "changes.approve",
                    new JsonObject
                    {
                        ["id"] = "cs-1",
                        ["personCredential"] = PersonToken,
                    });

                JsonObject reverted = ContentOf(host.CallTool(
                    personSession,
                    "changes.revert",
                    new JsonObject
                    {
                        ["id"] = "cs-1",
                        ["personCredential"] = PersonToken,
                    }));
                Check.Equal("surface: revert returns the applied set", "applied", reverted["status"]!.GetValue<string>());
                JsonObject restore = (JsonObject)reverted["restore"]!;
                Check.Equal(
                    "surface: the restore report counts the restored file",
                    1,
                    restore["restoredCount"]!.GetValue<int>());
                Check.Equal(
                    "surface: revert restores the before content",
                    "original\n",
                    File.ReadAllText(Path.Combine(project, "Assets", "Notes.txt")));
            }
            finally
            {
                DeleteDirectory(project);
            }
        }

        private static void ShowAndListRenderItemsRiskFirst()
        {
            using McpHost host = new McpHost(new FixedClock(34_000), PersonCredential.FromToken(PersonToken));
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
                    ["tool"] = "test.mutate",
                    ["path"] = "Assets/Notes.md",
                    ["before"] = "first\n",
                    ["after"] = "second\n",
                });
                return held.ToValue();
            });
            AddOperation(held!, new JsonObject
            {
                ["tool"] = "test.mutate",
                ["path"] = "Assets/Editor/FooTests.cs",
                ["before"] = "class FooTests {}\n",
                ["after"] = string.Empty,
            });

            JsonObject shown = ContentOf(host.CallTool(
                personSession,
                "changes.show",
                new JsonObject
                {
                    ["id"] = held!.Id,
                }));
            JsonArray items = (JsonArray)shown["items"]!;
            Check.Equal("surface: show lists both items", 2, items.Count);
            Check.Equal(
                "surface: the risk-flagged item is listed first",
                "test-deletion",
                ((JsonObject)items[0]!)["risk"]!.GetValue<string>());
            Check.Equal(
                "surface: the reordered item keeps its id",
                held.Id + ".2",
                ((JsonObject)items[0]!)["id"]!.GetValue<string>());
            Check.Equal(
                "surface: the content edit is listed last",
                "content-edit",
                ((JsonObject)items[1]!)["risk"]!.GetValue<string>());

            JsonObject listed = ContentOf(host.CallTool(personSession, "changes.list", new JsonObject()));
            JsonArray changeSets = (JsonArray)listed["changeSets"]!;
            JsonArray listedItems = (JsonArray)((JsonObject)changeSets[0]!)["items"]!;
            Check.Equal(
                "surface: the list payload is risk-first too",
                "test-deletion",
                ((JsonObject)listedItems[0]!)["risk"]!.GetValue<string>());
            Check.Equal(
                "surface: the stored item order is unchanged",
                ChangeRisks.AsString(ChangeRisk.ContentEdit),
                ChangeRisks.AsString(store.Get(held.Id).Items[0].Risk));
        }

        private static void AddOperation(ChangeSet set, JsonObject operation)
        {
            set.AddOperation(operation);
        }

        private static string CreateProject(string label)
        {
            string root = Path.Combine(
                Path.GetTempPath(),
                "unitas-u2b-surface-" + label + "-" + Guid.NewGuid().ToString("N"));
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
    }
}
