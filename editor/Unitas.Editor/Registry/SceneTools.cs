using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Text.Json.Nodes;
using Unitas.Editor.Scene;

namespace Unitas.Editor.Mcp
{
    /// <summary>
    /// Scene authoring commands. One registry entry per command keeps the MCP surface and
    /// the headless CLI on the same implementation, with the same scopes, the same
    /// untrusted-content marking and the same change-set provenance as the other tools.
    /// </summary>
    public static class SceneTools
    {
        public const string HierarchyOrigin = "unitas://scene/hierarchy";
        public const string EditOrigin = "unitas://scene/edit";

        // The registry lives for the whole process, so the open documents and their undo
        // histories are shared state behind one lock.
        private static readonly object Gate = new object();
        private static readonly Dictionary<string, OpenScene> Scenes = new Dictionary<string, OpenScene>(StringComparer.Ordinal);
        private static string? currentScene;

        public static void Register(McpHost host, ChangeSetStore store)
        {
            ArgumentNullException.ThrowIfNull(host);
            ArgumentNullException.ThrowIfNull(store);

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
                _ => HandleSave());

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
                _ => HandleUndo());

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
                _ => HandleRedo());

            host.RegisterResource(
                new ResourceDefinition(
                    HierarchyOrigin,
                    "Scene Hierarchy",
                    "GameObject hierarchy of the open scene with the file ids of its children and components",
                    "application/json",
                    Scope.ReadProject),
                () => ResourceContent.Untrusted(HierarchyOrigin, HierarchyPayload(CurrentScene("scene.hierarchy"))));
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

            SceneOp op = SceneOp.SetProperty(fileId, propertyPath, value);
            SceneOp inverse;
            lock (Gate)
            {
                inverse = ApplyOp(scene, op, "scene.set_property");
                scene.History.Push(op, inverse);
            }

            JsonObject operation = new JsonObject
            {
                ["tool"] = "scene.set_property",
                ["scene"] = scene.RelativePath,
                ["fileId"] = fileId,
                ["path"] = propertyPath,
                ["before"] = inverse.Value,
                ["after"] = value,
            };
            ChangeSet set = HoldChange(host, store, operation, "scene.set_property");

            JsonObject payload = new JsonObject
            {
                ["status"] = "applied",
                ["scene"] = scene.RelativePath,
                ["fileId"] = fileId,
                ["path"] = propertyPath,
                ["before"] = inverse.Value,
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

            SceneOp op = SceneOp.CreateGameObject(name, parentFileId);
            SceneOp inverse;
            lock (Gate)
            {
                inverse = ApplyOp(scene, op, "scene.create_object");
                scene.History.Push(op, inverse);
            }

            JsonObject operation = new JsonObject
            {
                ["tool"] = "scene.create_object",
                ["scene"] = scene.RelativePath,
                ["fileId"] = inverse.FileId,
                ["name"] = name,
                ["parentFileId"] = parentFileId,
                ["before"] = string.Empty,
                ["after"] = name,
            };
            ChangeSet set = HoldChange(host, store, operation, "scene.create_object");

            JsonObject payload = new JsonObject
            {
                ["status"] = "applied",
                ["scene"] = scene.RelativePath,
                ["fileId"] = inverse.FileId,
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

            SceneOp op = SceneOp.DestroyGameObject(fileId);
            SceneOp inverse;
            lock (Gate)
            {
                inverse = ApplyOp(scene, op, "scene.destroy_object");
                scene.History.Push(op, inverse);
            }

            JsonObject operation = new JsonObject
            {
                ["tool"] = "scene.destroy_object",
                ["scene"] = scene.RelativePath,
                ["fileId"] = fileId,
                ["before"] = inverse.ObjectName,
                ["after"] = string.Empty,
            };
            ChangeSet set = HoldChange(host, store, operation, "scene.destroy_object");

            JsonObject payload = new JsonObject
            {
                ["status"] = "applied",
                ["scene"] = scene.RelativePath,
                ["fileId"] = fileId,
                ["name"] = inverse.ObjectName,
                ["changeId"] = set.Id,
                ["changeSet"] = set.ToValue(),
            };
            return ToolOutput.Untrusted(EditOrigin, payload).WithAffectedIdentities(set.Id);
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

        // The hierarchy is the view of the document, so it is rebuilt from the current
        // document text: a property edit changes that text without touching the objects
        // the document cached when it was loaded.
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

        // Reloading the document text rebuilds the object model from it; the text is the
        // document's own text, so the reload changes the model and not the document.
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
            string sessionId = host.CurrentSessionId
                ?? throw new McpError.InvalidRequest(toolName + " requires an active MCP session");
            return store.Hold(host.GetSessionIdentity(sessionId).Agent, operation);
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

                // A number is a valid scene value too, and the scene file spells it
                // without a locale.
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

        // One open scene: the document the commands edit and the undo stack of that
        // document, so undo and redo restore the bytes the edit replaced.
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
