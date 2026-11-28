using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Text.Json.Nodes;
using Unitas.Editor.Scene;

namespace Unitas.Editor.Mcp
{
    public static class SceneTools
    {
        public const string HierarchyOrigin = "unitas://scene/hierarchy";
        public const string EditOrigin = "unitas://scene/edit";

        private const long KeepParentFileId = -1;

        private static readonly object Gate = new object();
        private static readonly Dictionary<string, OpenScene> Scenes = new Dictionary<string, OpenScene>(StringComparer.Ordinal);

        private static readonly Dictionary<string, string> EditStates = new Dictionary<string, string>(StringComparer.Ordinal);
        private static string? currentScene;

        public static void Register(McpHost host, ChangeSetStore store)
        {
            ArgumentNullException.ThrowIfNull(host);
            ArgumentNullException.ThrowIfNull(store);

            RegisterCoreSceneTools(host, store);
            RegisterHistorySceneTools(host, store);
            RegisterExtendedSceneTools(host, store);
            RegisterSceneAppliers(store);

            host.RegisterResource(
                new ResourceDefinition(
                    HierarchyOrigin,
                    "Scene Hierarchy",
                    "GameObject hierarchy of the open scene with the file ids of its children and components",
                    "application/json",
                    Scope.ReadProject),
                () => ResourceContent.Untrusted(HierarchyOrigin, HierarchyPayload(CurrentScene("scene.hierarchy"))));
        }

        private static void RegisterCoreSceneTools(McpHost host, ChangeSetStore store)
        {
            host.RegisterTool(
                new ToolDefinition(
                    "scene.open",
                    "Open a scene file of the open unitas project and report its hierarchy",
                    new JsonObject
                    {
                        ["type"] = "object",
                        ["properties"] = new JsonObject
                        {
                            ["projectPath"] = new JsonObject
                            {
                                ["type"] = "string",
                            },
                            ["path"] = new JsonObject
                            {
                                ["type"] = "string",
                            },
                        },
                        ["required"] = new JsonArray("path"),
                    },
                    Scope.ReadProject),
                arguments => HandleOpen(host, arguments));

            host.RegisterTool(
                new ToolDefinition(
                    "scene.hierarchy",
                    "Report the hierarchy of the open scene, or of the scene named by 'path'",
                    new JsonObject
                    {
                        ["type"] = "object",
                        ["properties"] = new JsonObject
                        {
                            ["projectPath"] = new JsonObject
                            {
                                ["type"] = "string",
                            },
                            ["path"] = new JsonObject
                            {
                                ["type"] = "string",
                            },
                        },
                    },
                    Scope.ReadProject),
                arguments => HandleHierarchy(host, arguments));

            host.RegisterTool(
                new ToolDefinition(
                    "scene.set_property",
                    "Set one property of one object of the open scene and record the edit in the agent change set",
                    new JsonObject
                    {
                        ["type"] = "object",
                        ["properties"] = new JsonObject
                        {
                            ["fileId"] = new JsonObject
                            {
                                ["type"] = "integer",
                            },
                            ["path"] = new JsonObject
                            {
                                ["type"] = "string",
                            },
                            ["value"] = new JsonObject
                            {
                                ["type"] = "string",
                            },
                        },
                        ["required"] = new JsonArray("fileId", "path", "value"),
                    },
                    Scope.EditContent),
                arguments => HandleSetProperty(host, store, arguments));

            host.RegisterTool(
                new ToolDefinition(
                    "scene.create_object",
                    "Create a GameObject in the open scene and record the edit in the agent change set",
                    new JsonObject
                    {
                        ["type"] = "object",
                        ["properties"] = new JsonObject
                        {
                            ["name"] = new JsonObject
                            {
                                ["type"] = "string",
                            },
                            ["parentFileId"] = new JsonObject
                            {
                                ["type"] = "integer",
                            },
                        },
                        ["required"] = new JsonArray("name"),
                    },
                    Scope.EditContent),
                arguments => HandleCreateObject(host, store, arguments));

            host.RegisterTool(
                new ToolDefinition(
                    "scene.destroy_object",
                    "Destroy one GameObject of the open scene and record the edit in the agent change set",
                    new JsonObject
                    {
                        ["type"] = "object",
                        ["properties"] = new JsonObject
                        {
                            ["fileId"] = new JsonObject
                            {
                                ["type"] = "integer",
                            },
                        },
                        ["required"] = new JsonArray("fileId"),
                    },
                    Scope.EditContent),
                arguments => HandleDestroyObject(host, store, arguments));
        }

        private static void RegisterHistorySceneTools(McpHost host, ChangeSetStore store)
        {
            host.RegisterTool(
                new ToolDefinition(
                    "scene.save",
                    "Write the open scene to the file it was opened from",
                    new JsonObject
                    {
                        ["type"] = "object",
                        ["properties"] = new JsonObject(),
                    },
                    Scope.EditContent),
                _ =>
                {
                    store.EnsureAgentWritesAllowed("scene.save");
                    return HandleSave();
                });

            host.RegisterTool(
                new ToolDefinition(
                    "editor.undo",
                    "Undo the last edit of the open scene",
                    new JsonObject
                    {
                        ["type"] = "object",
                        ["properties"] = new JsonObject(),
                    },
                    Scope.EditContent),
                _ =>
                {
                    store.EnsureAgentWritesAllowed("editor.undo");
                    return HandleUndo();
                });

            host.RegisterTool(
                new ToolDefinition(
                    "editor.redo",
                    "Redo the last undone edit of the open scene",
                    new JsonObject
                    {
                        ["type"] = "object",
                        ["properties"] = new JsonObject(),
                    },
                    Scope.EditContent),
                _ =>
                {
                    store.EnsureAgentWritesAllowed("editor.redo");
                    return HandleRedo();
                });
        }

        private static void RegisterExtendedSceneTools(McpHost host, ChangeSetStore store)
        {
            host.RegisterTool(
                new ToolDefinition(
                    "scene.inspect_object",
                    "Report one object of the open scene with the components and properties the scene file holds for it",
                    new JsonObject
                    {
                        ["type"] = "object",
                        ["properties"] = new JsonObject
                        {
                            ["fileId"] = new JsonObject
                            {
                                ["type"] = "integer",
                            },
                        },
                        ["required"] = new JsonArray("fileId"),
                    },
                    Scope.ReadProject),
                arguments => HandleInspectObject(host, arguments));

            host.RegisterTool(
                new ToolDefinition(
                    "scene.reparent_object",
                    "Move one GameObject of the open scene under a new parent and record the edit in the agent change set",
                    new JsonObject
                    {
                        ["type"] = "object",
                        ["properties"] = new JsonObject
                        {
                            ["fileId"] = new JsonObject
                            {
                                ["type"] = "integer",
                            },
                            ["parentFileId"] = new JsonObject
                            {
                                ["type"] = "integer",
                            },
                        },
                        ["required"] = new JsonArray("fileId"),
                    },
                    Scope.EditContent),
                arguments => HandleReparentObject(host, store, arguments));

            host.RegisterTool(
                new ToolDefinition(
                    "scene.set_active",
                    "Activate or deactivate one GameObject of the open scene and record the edit in the agent change set",
                    new JsonObject
                    {
                        ["type"] = "object",
                        ["properties"] = new JsonObject
                        {
                            ["fileId"] = new JsonObject
                            {
                                ["type"] = "integer",
                            },
                            ["active"] = new JsonObject
                            {
                                ["type"] = "boolean",
                            },
                        },
                        ["required"] = new JsonArray("fileId", "active"),
                    },
                    Scope.EditContent),
                arguments => HandleSetActive(host, store, arguments));

            host.RegisterTool(
                new ToolDefinition(
                    "scene.duplicate_object",
                    "Copy one GameObject of the open scene with its transform and activation state and record the edit in the agent change set",
                    new JsonObject
                    {
                        ["type"] = "object",
                        ["properties"] = new JsonObject
                        {
                            ["fileId"] = new JsonObject
                            {
                                ["type"] = "integer",
                            },
                            ["name"] = new JsonObject
                            {
                                ["type"] = "string",
                            },
                            ["parentFileId"] = new JsonObject
                            {
                                ["type"] = "integer",
                            },
                        },
                        ["required"] = new JsonArray("fileId"),
                    },
                    Scope.EditContent),
                arguments => HandleDuplicateObject(host, store, arguments));

            host.RegisterTool(
                new ToolDefinition(
                    "scene.snapshot_edit_state",
                    "Capture the edit-time state of the open scene so a later restore can bring it back byte for byte",
                    new JsonObject
                    {
                        ["type"] = "object",
                        ["properties"] = new JsonObject(),
                    },
                    Scope.ReadProject),
                _ => HandleSnapshotEditState());

            host.RegisterTool(
                new ToolDefinition(
                    "scene.restore_edit_state",
                    "Restore the captured edit-time state of the open scene and discard the changes made since the capture",
                    new JsonObject
                    {
                        ["type"] = "object",
                        ["properties"] = new JsonObject(),
                    },
                    Scope.EditContent),
                _ => HandleRestoreEditState());
        }

        private static ToolOutput HandleOpen(McpHost host, JsonNode arguments)
        {
            string root = ProjectAccess.ResolveRoot(host, arguments, "scene.open");
            string relative = RequireText(arguments, "path", "scene.open");
            string full = ProjectAccess.ResolveFile(root, relative, "scene.open");
            if (!File.Exists(full))
            {
                throw new McpError.InvalidRequest("scene.open: the scene file does not exist: " + relative);
            }

            SceneDocument document = OpenDocument(full, "scene.open");
            OpenScene scene = new OpenScene(relative, full, document);
            lock (Gate)
            {
                Scenes[full] = scene;
                currentScene = full;
            }

            return ToolOutput.Untrusted(HierarchyOrigin, HierarchyPayload(scene));
        }

        private static ToolOutput HandleHierarchy(McpHost host, JsonNode arguments)
        {
            string? relative = ProjectAccess.OptionalString(arguments, "path");
            if (relative == null)
            {
                return ToolOutput.Untrusted(HierarchyOrigin, HierarchyPayload(CurrentScene("scene.hierarchy")));
            }

            string root = ProjectAccess.ResolveRoot(host, arguments, "scene.hierarchy");
            string full = ProjectAccess.ResolveFile(root, relative, "scene.hierarchy");
            return ToolOutput.Untrusted(HierarchyOrigin, HierarchyPayload(GetOrOpen(full, relative, "scene.hierarchy")));
        }

        private static ToolOutput HandleSetProperty(McpHost host, ChangeSetStore store, JsonNode arguments)
        {
            OpenScene scene = CurrentScene("scene.set_property");
            long fileId = RequireLong(arguments, "fileId", "scene.set_property");
            string propertyPath = RequireText(arguments, "path", "scene.set_property");
            string value = RequireValue(arguments, "value", "scene.set_property");

            JsonObject operation = new JsonObject
            {
                ["tool"] = "scene.set_property",
                ["scene"] = scene.RelativePath,
                ["sceneFile"] = scene.FullPath,
                ["fileId"] = fileId,
                ["path"] = propertyPath,
                ["after"] = value,
            };
            ChangeSet set = HoldChange(host, store, operation, "scene.set_property");
            if (store.Policy.RequireApproval)
            {
                return ToolOutput.Untrusted(EditOrigin, PendingEditPayload(scene, set, new JsonObject
                {
                    ["fileId"] = fileId,
                    ["path"] = propertyPath,
                    ["after"] = value,
                })).WithAffectedIdentities(set.Id);
            }

            JsonObject payload = new JsonObject
            {
                ["status"] = "applied",
                ["scene"] = scene.RelativePath,
                ["fileId"] = fileId,
                ["path"] = propertyPath,
                ["before"] = set.Operations[0]["before"]?.DeepClone(),
                ["after"] = value,
                ["changeId"] = set.Id,
                ["changeSet"] = set.ToValue(),
            };
            return ToolOutput.Untrusted(EditOrigin, payload).WithAffectedIdentities(set.Id);
        }

        private static ToolOutput HandleCreateObject(McpHost host, ChangeSetStore store, JsonNode arguments)
        {
            OpenScene scene = CurrentScene("scene.create_object");
            string name = RequireText(arguments, "name", "scene.create_object");
            long parentFileId = OptionalLong(arguments, "parentFileId");

            JsonObject operation = new JsonObject
            {
                ["tool"] = "scene.create_object",
                ["scene"] = scene.RelativePath,
                ["sceneFile"] = scene.FullPath,
                ["name"] = name,
                ["parentFileId"] = parentFileId,
                ["before"] = string.Empty,
                ["after"] = name,
            };
            ChangeSet set = HoldChange(host, store, operation, "scene.create_object");
            if (store.Policy.RequireApproval)
            {
                return ToolOutput.Untrusted(EditOrigin, PendingEditPayload(scene, set, new JsonObject
                {
                    ["name"] = name,
                    ["parentFileId"] = parentFileId,
                })).WithAffectedIdentities(set.Id);
            }

            JsonObject payload = new JsonObject
            {
                ["status"] = "applied",
                ["scene"] = scene.RelativePath,
                ["fileId"] = set.Operations[0]["appliedFileId"]?.DeepClone(),
                ["name"] = name,
                ["parentFileId"] = parentFileId,
                ["changeId"] = set.Id,
                ["changeSet"] = set.ToValue(),
            };
            return ToolOutput.Untrusted(EditOrigin, payload).WithAffectedIdentities(set.Id);
        }

        private static ToolOutput HandleDestroyObject(McpHost host, ChangeSetStore store, JsonNode arguments)
        {
            OpenScene scene = CurrentScene("scene.destroy_object");
            long fileId = RequireLong(arguments, "fileId", "scene.destroy_object");

            string name = NameOf(scene, fileId, "scene.destroy_object");
            JsonObject operation = new JsonObject
            {
                ["tool"] = "scene.destroy_object",
                ["scene"] = scene.RelativePath,
                ["sceneFile"] = scene.FullPath,
                ["fileId"] = fileId,
                ["before"] = name,
                ["after"] = string.Empty,
            };
            ChangeSet set = HoldChange(host, store, operation, "scene.destroy_object");
            if (store.Policy.RequireApproval)
            {
                return ToolOutput.Untrusted(EditOrigin, PendingEditPayload(scene, set, new JsonObject
                {
                    ["fileId"] = fileId,
                    ["name"] = name,
                })).WithAffectedIdentities(set.Id);
            }

            JsonObject payload = new JsonObject
            {
                ["status"] = "applied",
                ["scene"] = scene.RelativePath,
                ["fileId"] = fileId,
                ["name"] = name,
                ["changeId"] = set.Id,
                ["changeSet"] = set.ToValue(),
            };
            return ToolOutput.Untrusted(EditOrigin, payload).WithAffectedIdentities(set.Id);
        }

        private static ToolOutput HandleInspectObject(McpHost host, JsonNode arguments)
        {
            OpenScene scene = CurrentScene("scene.inspect_object");
            long fileId = RequireLong(arguments, "fileId", "scene.inspect_object");

            SceneInspection inspection;
            lock (Gate)
            {
                RefreshModel(scene.Document);
                try
                {
                    inspection = scene.Document.Inspect(fileId);
                }
                catch (SceneParseException error)
                {
                    throw new McpError.InvalidRequest("scene.inspect_object cannot inspect the object: " + error.Message);
                }
            }

            JsonArray childFileIds = new JsonArray();
            foreach (long childFileId in inspection.ChildFileIds)
            {
                childFileIds.Add(childFileId);
            }

            JsonArray components = new JsonArray();
            foreach (SceneComponentInspection component in inspection.Components)
            {
                JsonArray properties = new JsonArray();
                foreach (SceneProperty property in component.Properties)
                {
                    properties.Add(new JsonObject
                    {
                        ["name"] = property.Name,
                        ["path"] = property.Name,
                        ["value"] = property.Value,
                    });
                }

                components.Add(new JsonObject
                {
                    ["fileId"] = component.FileId,
                    ["type"] = component.Type,
                    ["properties"] = properties,
                });
            }

            JsonObject payload = new JsonObject
            {
                ["scene"] = scene.RelativePath,
                ["fileId"] = inspection.FileId,
                ["name"] = inspection.Name,
                ["parentFileId"] = inspection.ParentFileId,
                ["active"] = inspection.Active,
                ["position"] = inspection.Position,
                ["rotation"] = inspection.Rotation,
                ["scale"] = inspection.Scale,
                ["childFileIds"] = childFileIds,
                ["components"] = components,
            };
            return ToolOutput.Untrusted(HierarchyOrigin, payload);
        }

        private static ToolOutput HandleReparentObject(McpHost host, ChangeSetStore store, JsonNode arguments)
        {
            OpenScene scene = CurrentScene("scene.reparent_object");
            long fileId = RequireLong(arguments, "fileId", "scene.reparent_object");
            long parentFileId = OptionalLong(arguments, "parentFileId");

            string previousParent = ParentOf(scene, fileId, "scene.reparent_object");
            JsonObject operation = new JsonObject
            {
                ["tool"] = "scene.reparent_object",
                ["scene"] = scene.RelativePath,
                ["sceneFile"] = scene.FullPath,
                ["fileId"] = fileId,
                ["parentFileId"] = parentFileId,
                ["before"] = previousParent,
                ["after"] = parentFileId.ToString(CultureInfo.InvariantCulture),
            };
            ChangeSet set = HoldChange(host, store, operation, "scene.reparent_object");
            if (store.Policy.RequireApproval)
            {
                return ToolOutput.Untrusted(EditOrigin, PendingEditPayload(scene, set, new JsonObject
                {
                    ["fileId"] = fileId,
                    ["parentFileId"] = parentFileId,
                    ["before"] = previousParent,
                    ["after"] = parentFileId.ToString(CultureInfo.InvariantCulture),
                })).WithAffectedIdentities(set.Id);
            }

            JsonObject payload = new JsonObject
            {
                ["status"] = "applied",
                ["scene"] = scene.RelativePath,
                ["fileId"] = fileId,
                ["parentFileId"] = parentFileId,
                ["before"] = previousParent,
                ["after"] = parentFileId.ToString(CultureInfo.InvariantCulture),
                ["changeId"] = set.Id,
                ["changeSet"] = set.ToValue(),
            };
            return ToolOutput.Untrusted(EditOrigin, payload).WithAffectedIdentities(set.Id);
        }

        private static ToolOutput HandleSetActive(McpHost host, ChangeSetStore store, JsonNode arguments)
        {
            OpenScene scene = CurrentScene("scene.set_active");
            long fileId = RequireLong(arguments, "fileId", "scene.set_active");
            bool active = RequireBool(arguments, "active", "scene.set_active");

            string previous = ActiveOf(scene, fileId, "scene.set_active");
            JsonObject operation = new JsonObject
            {
                ["tool"] = "scene.set_active",
                ["scene"] = scene.RelativePath,
                ["sceneFile"] = scene.FullPath,
                ["fileId"] = fileId,
                ["active"] = active,
                ["before"] = previous,
                ["after"] = active ? "1" : "0",
            };
            ChangeSet set = HoldChange(host, store, operation, "scene.set_active");
            if (store.Policy.RequireApproval)
            {
                return ToolOutput.Untrusted(EditOrigin, PendingEditPayload(scene, set, new JsonObject
                {
                    ["fileId"] = fileId,
                    ["active"] = active,
                    ["before"] = previous,
                    ["after"] = active ? "1" : "0",
                })).WithAffectedIdentities(set.Id);
            }

            JsonObject payload = new JsonObject
            {
                ["status"] = "applied",
                ["scene"] = scene.RelativePath,
                ["fileId"] = fileId,
                ["active"] = active,
                ["before"] = previous,
                ["after"] = active ? "1" : "0",
                ["changeId"] = set.Id,
                ["changeSet"] = set.ToValue(),
            };
            return ToolOutput.Untrusted(EditOrigin, payload).WithAffectedIdentities(set.Id);
        }

        private static ToolOutput HandleDuplicateObject(McpHost host, ChangeSetStore store, JsonNode arguments)
        {
            OpenScene scene = CurrentScene("scene.duplicate_object");
            long fileId = RequireLong(arguments, "fileId", "scene.duplicate_object");
            string? name = ProjectAccess.OptionalString(arguments, "name");
            long requestedParentFileId = OptionalLongOrDefault(arguments, "parentFileId", KeepParentFileId);

            string sourceName = NameOf(scene, fileId, "scene.duplicate_object");
            string copyName = name ?? (sourceName + " Copy");
            long parentFileId = requestedParentFileId;
            if (parentFileId == KeepParentFileId)
            {
                parentFileId = ParentFileIdOf(scene, fileId, "scene.duplicate_object");
            }

            JsonObject operation = new JsonObject
            {
                ["tool"] = "scene.duplicate_object",
                ["scene"] = scene.RelativePath,
                ["sceneFile"] = scene.FullPath,
                ["fileId"] = fileId,
                ["name"] = copyName,
                ["parentFileId"] = parentFileId,
                ["before"] = string.Empty,
                ["after"] = copyName,
            };
            ChangeSet set = HoldChange(host, store, operation, "scene.duplicate_object");
            if (store.Policy.RequireApproval)
            {
                return ToolOutput.Untrusted(EditOrigin, PendingEditPayload(scene, set, new JsonObject
                {
                    ["sourceFileId"] = fileId,
                    ["name"] = copyName,
                    ["parentFileId"] = parentFileId,
                })).WithAffectedIdentities(set.Id);
            }

            JsonObject payload = new JsonObject
            {
                ["status"] = "applied",
                ["scene"] = scene.RelativePath,
                ["fileId"] = set.Operations[0]["appliedFileId"]?.DeepClone(),
                ["sourceFileId"] = fileId,
                ["name"] = copyName,
                ["parentFileId"] = parentFileId,
                ["changeId"] = set.Id,
                ["changeSet"] = set.ToValue(),
            };
            return ToolOutput.Untrusted(EditOrigin, payload).WithAffectedIdentities(set.Id);
        }

        private static ToolOutput HandleSnapshotEditState()
        {
            JsonObject? captured = TryCaptureEditState();
            if (captured == null)
            {
                throw new McpError.InvalidRequest("scene.snapshot_edit_state requires an open scene: call scene.open first");
            }

            return ToolOutput.Untrusted(EditOrigin, captured);
        }

        private static ToolOutput HandleRestoreEditState()
        {
            JsonObject? restored = TryRestoreEditState();
            if (restored == null)
            {
                throw new McpError.InvalidRequest("scene.restore_edit_state requires a captured edit-time state: call scene.snapshot_edit_state first");
            }

            return ToolOutput.Untrusted(EditOrigin, restored);
        }

        public static JsonObject? TryCaptureEditState()
        {
            lock (Gate)
            {
                OpenScene? scene = CurrentSceneOrNull();
                if (scene == null)
                {
                    return null;
                }

                string text = scene.Document.Save();
                EditStates[scene.FullPath] = text;
                return new JsonObject
                {
                    ["status"] = "captured",
                    ["scene"] = scene.RelativePath,
                    ["bytes"] = text.Length,
                };
            }
        }

        public static JsonObject? TryRestoreEditState()
        {
            lock (Gate)
            {
                OpenScene? scene = CurrentSceneOrNull();
                if (scene == null)
                {
                    return null;
                }

                if (!EditStates.TryGetValue(scene.FullPath, out string? text))
                {
                    return null;
                }

                EditStates.Remove(scene.FullPath);
                try
                {
                    scene.Document.Restore(text);
                }
                catch (SceneParseException error)
                {
                    throw new McpError.InvalidRequest("scene.restore_edit_state cannot restore the scene: " + error.Message);
                }

                return new JsonObject
                {
                    ["status"] = "restored",
                    ["scene"] = scene.RelativePath,
                    ["bytes"] = text.Length,
                };
            }
        }

        public static JsonObject ApplyRuntimeProperty(long fileId, string path, string value)
        {
            OpenScene scene = CurrentScene("play.set_runtime_property");
            lock (Gate)
            {
                SceneOp op = SceneOp.SetProperty(fileId, path, value);
                SceneOp inverse = ApplyOp(scene, op, "play.set_runtime_property");
                return new JsonObject
                {
                    ["status"] = "applied",
                    ["scene"] = scene.RelativePath,
                    ["fileId"] = fileId,
                    ["path"] = path,
                    ["before"] = inverse.Value,
                    ["after"] = value,
                };
            }
        }

        private static OpenScene? CurrentSceneOrNull()
        {
            if (currentScene != null && Scenes.TryGetValue(currentScene, out OpenScene? scene))
            {
                return scene;
            }

            return null;
        }

        private static string ParentOf(OpenScene scene, long fileId, string toolName)
        {
            lock (Gate)
            {
                try
                {
                    return scene.Document.Inspect(fileId).ParentFileId.ToString(CultureInfo.InvariantCulture);
                }
                catch (SceneParseException error)
                {
                    throw new McpError.InvalidRequest(toolName + " cannot read the object: " + error.Message);
                }
            }
        }

        private static string ActiveOf(OpenScene scene, long fileId, string toolName)
        {
            lock (Gate)
            {
                try
                {
                    return scene.Document.Inspect(fileId).Active ? "1" : "0";
                }
                catch (SceneParseException error)
                {
                    throw new McpError.InvalidRequest(toolName + " cannot read the object: " + error.Message);
                }
            }
        }

        private static string NameOf(OpenScene scene, long fileId, string toolName)
        {
            lock (Gate)
            {
                try
                {
                    return scene.Document.Inspect(fileId).Name;
                }
                catch (SceneParseException error)
                {
                    throw new McpError.InvalidRequest(toolName + " cannot read the object: " + error.Message);
                }
            }
        }

        private static long ParentFileIdOf(OpenScene scene, long fileId, string toolName)
        {
            lock (Gate)
            {
                try
                {
                    return scene.Document.Inspect(fileId).ParentFileId;
                }
                catch (SceneParseException error)
                {
                    throw new McpError.InvalidRequest(toolName + " cannot read the object: " + error.Message);
                }
            }
        }

        private static ToolOutput HandleSave()
        {
            OpenScene scene = CurrentScene("scene.save");
            lock (Gate)
            {
                try
                {
                    scene.Document.SaveTo(scene.FullPath);
                }
                catch (Exception error) when (error is IOException || error is UnauthorizedAccessException)
                {
                    throw new McpError.Io(error);
                }
            }

            JsonObject payload = new JsonObject
            {
                ["status"] = "saved",
                ["scene"] = scene.RelativePath,
                ["path"] = scene.FullPath,
            };
            return ToolOutput.Untrusted(EditOrigin, payload);
        }

        private static ToolOutput HandleUndo()
        {
            OpenScene scene = CurrentScene("editor.undo");
            UndoResult<SceneOp> result;
            lock (Gate)
            {
                result = scene.History.Undo();
            }

            return ToolOutput.Untrusted(EditOrigin, HistoryPayload(scene, result, "editor.undo"));
        }

        private static ToolOutput HandleRedo()
        {
            OpenScene scene = CurrentScene("editor.redo");
            UndoResult<SceneOp> result;
            lock (Gate)
            {
                result = scene.History.Redo();
            }

            return ToolOutput.Untrusted(EditOrigin, HistoryPayload(scene, result, "editor.redo"));
        }

        private static JsonObject HierarchyPayload(OpenScene scene)
        {
            JsonArray objects = new JsonArray();
            lock (Gate)
            {
                RefreshModel(scene.Document);
                foreach (SceneObject sceneObject in scene.Document.Objects)
                {
                    JsonArray children = new JsonArray();
                    foreach (SceneObject child in sceneObject.Children)
                    {
                        children.Add(child.FileId);
                    }

                    JsonArray components = new JsonArray();
                    foreach (long componentFileId in sceneObject.ComponentFileIds)
                    {
                        components.Add(componentFileId);
                    }

                    objects.Add(new JsonObject
                    {
                        ["fileId"] = sceneObject.FileId,
                        ["name"] = sceneObject.Name,
                        ["parentFileId"] = sceneObject.ParentFileId,
                        ["children"] = children,
                        ["componentFileIds"] = components,
                    });
                }
            }

            return new JsonObject
            {
                ["path"] = scene.RelativePath,
                ["objects"] = objects,
                ["objectCount"] = objects.Count,
                ["canUndo"] = scene.History.CanUndo,
                ["canRedo"] = scene.History.CanRedo,
            };
        }

        private static JsonObject HistoryPayload(OpenScene scene, UndoResult<SceneOp> result, string toolName)
        {
            JsonObject payload = new JsonObject
            {
                ["tool"] = toolName,
                ["scene"] = scene.RelativePath,
                ["ok"] = result.Ok,
                ["canUndo"] = scene.History.CanUndo,
                ["canRedo"] = scene.History.CanRedo,
            };

            if (result.Ok)
            {
                payload["status"] = "applied";
                payload["applied"] = DescribeOp(result.Applied);
            }
            else
            {
                payload["status"] = "error";
                payload["error"] = result.Error;
            }

            return payload;
        }

        private static JsonObject DescribeOp(SceneOp op)
        {
            return new JsonObject
            {
                ["kind"] = op.Kind.ToString(),
                ["fileId"] = op.FileId,
                ["parentFileId"] = op.ParentFileId,
                ["path"] = op.PropertyPath,
                ["value"] = op.Value,
                ["name"] = op.ObjectName,
            };
        }

        private static void RefreshModel(SceneDocument document)
        {
            string text = document.Save();
            try
            {
                document.Apply(SceneOp.SetProperty(0, string.Empty, string.Empty).WithSnapshot(text));
            }
            catch (SceneParseException error)
            {
                throw new McpError.InvalidRequest("scene.hierarchy cannot read the scene: " + error.Message);
            }
        }

        private static SceneOp ApplyOp(OpenScene scene, SceneOp op, string toolName)
        {
            try
            {
                return scene.Document.Apply(op);
            }
            catch (SceneParseException error)
            {
                throw new McpError.InvalidRequest(toolName + " cannot apply the edit: " + error.Message);
            }
        }

        private static SceneDocument OpenDocument(string fullPath, string toolName)
        {
            try
            {
                return SceneDocument.Open(fullPath);
            }
            catch (SceneParseException error)
            {
                throw new McpError.InvalidRequest(toolName + " cannot read the scene: " + error.Message);
            }
            catch (Exception error) when (error is IOException || error is UnauthorizedAccessException)
            {
                throw new McpError.Io(error);
            }
        }

        private static OpenScene GetOrOpen(string fullPath, string relative, string toolName)
        {
            lock (Gate)
            {
                if (Scenes.TryGetValue(fullPath, out OpenScene? opened))
                {
                    return opened;
                }
            }

            if (!File.Exists(fullPath))
            {
                throw new McpError.InvalidRequest(toolName + ": the scene file does not exist: " + relative);
            }

            OpenScene scene = new OpenScene(relative, fullPath, OpenDocument(fullPath, toolName));
            lock (Gate)
            {
                Scenes[fullPath] = scene;
            }

            return scene;
        }

        private static OpenScene CurrentScene(string toolName)
        {
            lock (Gate)
            {
                if (currentScene != null && Scenes.TryGetValue(currentScene, out OpenScene? scene))
                {
                    return scene;
                }
            }

            throw new McpError.InvalidRequest(toolName + " requires an open scene: call scene.open first");
        }

        private static ChangeSet HoldChange(McpHost host, ChangeSetStore store, JsonObject operation, string toolName)
        {
            store.EnsureAgentWritesAllowed(toolName);
            string sessionId = host.CurrentSessionId
                ?? throw new McpError.InvalidRequest(toolName + " requires an active MCP session");
            return store.Hold(host.GetSessionIdentity(sessionId).Agent, operation);
        }

        private static void RegisterSceneAppliers(ChangeSetStore store)
        {
            store.RegisterApplier("scene.set_property", ApplyRecordedSceneOperation);
            store.RegisterApplier("scene.create_object", ApplyRecordedSceneOperation);
            store.RegisterApplier("scene.destroy_object", ApplyRecordedSceneOperation);
            store.RegisterApplier("scene.reparent_object", ApplyRecordedSceneOperation);
            store.RegisterApplier("scene.set_active", ApplyRecordedSceneOperation);
            store.RegisterApplier("scene.duplicate_object", ApplyRecordedSceneOperation);
        }

        private static JsonObject ApplyRecordedSceneOperation(JsonObject operation)
        {
            string toolName = RecordedText(operation, "tool", "scene.review");
            string relative = RecordedText(operation, "scene", toolName);
            string fullPath = RecordedText(operation, "sceneFile", toolName);
            OpenScene scene = GetOrOpen(fullPath, relative, toolName);
            SceneOp op = RecordedSceneOp(operation, toolName);
            lock (Gate)
            {
                SceneOp inverse = ApplyOp(scene, op, toolName);
                scene.History.Push(op, inverse);
                if (toolName == "scene.set_property")
                {
                    operation["before"] = inverse.Value;
                }
                else if (toolName == "scene.create_object" || toolName == "scene.duplicate_object")
                {
                    operation["appliedFileId"] = inverse.FileId;
                }
            }

            return new JsonObject();
        }

        private static SceneOp RecordedSceneOp(JsonObject operation, string toolName)
        {
            switch (toolName)
            {
                case "scene.set_property":
                    return SceneOp.SetProperty(
                        RecordedLong(operation, "fileId", toolName),
                        RecordedText(operation, "path", toolName),
                        RecordedText(operation, "after", toolName));
                case "scene.create_object":
                    return SceneOp.CreateGameObject(
                        RecordedText(operation, "name", toolName),
                        RecordedLong(operation, "parentFileId", toolName));
                case "scene.destroy_object":
                    return SceneOp.DestroyGameObject(RecordedLong(operation, "fileId", toolName));
                case "scene.reparent_object":
                    return SceneOp.Reparent(
                        RecordedLong(operation, "fileId", toolName),
                        RecordedLong(operation, "parentFileId", toolName));
                case "scene.set_active":
                    return SceneOp.SetActive(
                        RecordedLong(operation, "fileId", toolName),
                        RecordedFlag(operation, toolName));
                case "scene.duplicate_object":
                    return SceneOp.DuplicateGameObject(
                        RecordedLong(operation, "fileId", toolName),
                        RecordedText(operation, "name", toolName),
                        RecordedLong(operation, "parentFileId", toolName));
                default:
                    throw new McpError.InvalidRequest(toolName + " has no recorded scene operation to apply");
            }
        }

        private static string RecordedText(JsonObject operation, string propertyName, string toolName)
        {
            if (operation[propertyName] is JsonValue value
                && value.TryGetValue(out string? text)
                && text != null)
            {
                return text;
            }

            throw new McpError.InvalidRequest(
                toolName + " holds a recorded operation without a textual '" + propertyName + "'");
        }

        private static long RecordedLong(JsonObject operation, string propertyName, string toolName)
        {
            if (TryReadLong(operation[propertyName], out long number))
            {
                return number;
            }

            throw new McpError.InvalidRequest(
                toolName + " holds a recorded operation without a numeric '" + propertyName + "'");
        }

        private static bool RecordedFlag(JsonObject operation, string toolName)
        {
            if (operation["active"] is JsonValue value && value.TryGetValue(out bool active))
            {
                return active;
            }

            throw new McpError.InvalidRequest(
                toolName + " holds a recorded operation without a boolean 'active'");
        }

        private static JsonObject PendingEditPayload(OpenScene scene, ChangeSet set, JsonObject payload)
        {
            payload["status"] = "pending-review";
            payload["scene"] = scene.RelativePath;
            payload["changeId"] = set.Id;
            payload["changeSet"] = set.ToValue();
            return payload;
        }

        private static string RequireText(JsonNode arguments, string propertyName, string toolName)
        {
            string text = RequireValue(arguments, propertyName, toolName);
            if (text.Trim().Length == 0)
            {
                throw new McpError.InvalidRequest(toolName + " requires a non-empty '" + propertyName + "' argument");
            }

            return text;
        }

        private static string RequireValue(JsonNode arguments, string propertyName, string toolName)
        {
            if (arguments is JsonObject obj && obj[propertyName] is JsonValue value)
            {
                if (value.TryGetValue(out string? text))
                {
                    return text;
                }

                if (value.TryGetValue(out double number))
                {
                    return number.ToString(CultureInfo.InvariantCulture);
                }
            }

            throw new McpError.InvalidRequest(toolName + " requires a '" + propertyName + "' argument");
        }

        private static long RequireLong(JsonNode arguments, string propertyName, string toolName)
        {
            if (arguments is JsonObject obj && TryReadLong(obj[propertyName], out long number))
            {
                return number;
            }

            throw new McpError.InvalidRequest(toolName + " requires a '" + propertyName + "' argument");
        }

        private static long OptionalLong(JsonNode arguments, string propertyName)
        {
            if (arguments is JsonObject obj && TryReadLong(obj[propertyName], out long number))
            {
                return number;
            }

            return 0;
        }

        private static long OptionalLongOrDefault(JsonNode arguments, string propertyName, long defaultValue)
        {
            if (arguments is JsonObject obj && TryReadLong(obj[propertyName], out long number))
            {
                return number;
            }

            return defaultValue;
        }

        private static bool RequireBool(JsonNode arguments, string propertyName, string toolName)
        {
            if (arguments is JsonObject obj && obj[propertyName] is JsonValue value)
            {
                if (value.TryGetValue(out bool flag))
                {
                    return flag;
                }

                if (value.TryGetValue(out string? text))
                {
                    if (string.Equals(text, "1", StringComparison.Ordinal)
                        || string.Equals(text, "true", StringComparison.OrdinalIgnoreCase))
                    {
                        return true;
                    }

                    if (string.Equals(text, "0", StringComparison.Ordinal)
                        || string.Equals(text, "false", StringComparison.OrdinalIgnoreCase))
                    {
                        return false;
                    }
                }
            }

            throw new McpError.InvalidRequest(toolName + " requires a '" + propertyName + "' argument");
        }

        private static bool TryReadLong(JsonNode? node, out long number)
        {
            number = 0;
            if (!(node is JsonValue value))
            {
                return false;
            }

            if (value.TryGetValue(out number))
            {
                return true;
            }

            if (value.TryGetValue(out int small))
            {
                number = small;
                return true;
            }

            if (value.TryGetValue(out double real)
                && real >= long.MinValue
                && real <= long.MaxValue
                && real == Math.Floor(real))
            {
                number = (long)real;
                return true;
            }

            return false;
        }

        private sealed class OpenScene
        {
            public OpenScene(string relativePath, string fullPath, SceneDocument document)
            {
                RelativePath = relativePath;
                FullPath = fullPath;
                Document = document;
                History = new UndoHistory<SceneOp>(document.Apply);
            }

            public string RelativePath { get; }

            public string FullPath { get; }

            public SceneDocument Document { get; }

            public UndoHistory<SceneOp> History { get; }
        }
    }
}
