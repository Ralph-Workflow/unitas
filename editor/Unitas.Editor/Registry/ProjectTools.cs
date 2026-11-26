using System;
using System.Collections.Generic;
using System.IO;
using System.Text.Json.Nodes;

namespace Unitas.Editor.Mcp
{
    public static class ProjectTools
    {
        public const string ListOrigin = "unitas://project/list";
        public const string FileOrigin = "unitas://project/file";

        private const int MaxListEntries = 1000;

        private static readonly string[] GeneratedDirectoryNames =
        {
            "Library",
            "Temp",
            "Logs",
            "obj",
            "bin",
            "Build",
            "Builds",
            "UserSettings",
            ".git",
            ".vs",
            ".vscode",
        };

        public static void Register(McpHost host, ChangeSetStore store)
        {
            ArgumentNullException.ThrowIfNull(host);
            ArgumentNullException.ThrowIfNull(store);

            host.RegisterTool(
                new ToolDefinition(
                    "project.list",
                    "Enumerate the files and directories of the open unitas project",
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
                arguments => HandleList(host, arguments));

            host.RegisterTool(
                new ToolDefinition(
                    "project.read_file",
                    "Read one text file of the open unitas project",
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
                arguments => HandleRead(host, arguments));

            host.RegisterTool(
                new ToolDefinition(
                    "project.write_file",
                    "Write one text file of the open unitas project and record the edit in the agent change set",
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
                            ["content"] = new JsonObject
                            {
                                ["type"] = "string",
                            },
                        },
                        ["required"] = new JsonArray("path", "content"),
                    },
                    Scope.EditContent),
                arguments => HandleWrite(host, store, arguments));
        }

        private static ToolOutput HandleList(McpHost host, JsonNode arguments)
        {
            string root = ProjectAccess.ResolveRoot(host, arguments, "project.list");
            string? subdirectory = ProjectAccess.OptionalString(arguments, "path");
            string start = subdirectory == null
                ? root
                : ProjectAccess.ResolveFile(root, subdirectory, "project.list");
            if (!Directory.Exists(start))
            {
                throw new McpError.InvalidRequest("project.list requires a directory path: " + subdirectory);
            }

            List<string> paths = new List<string>();
            bool truncated = false;
            Collect(start, root, paths, ref truncated);

            JsonArray entries = new JsonArray();
            foreach (string relative in paths)
            {
                string full = Path.Combine(root, relative);
                bool isDirectory = Directory.Exists(full);
                JsonObject entry = new JsonObject
                {
                    ["path"] = relative,
                    ["type"] = isDirectory ? "directory" : "file",
                };
                if (!isDirectory)
                {
                    entry["bytes"] = FileSize(full);
                }

                entries.Add(entry);
            }

            JsonObject payload = new JsonObject
            {
                ["projectPath"] = root,
                ["entries"] = entries,
                ["truncated"] = truncated,
            };
            return ToolOutput.Untrusted(ListOrigin, payload);
        }

        private static ToolOutput HandleRead(McpHost host, JsonNode arguments)
        {
            string root = ProjectAccess.ResolveRoot(host, arguments, "project.read_file");
            string relative = RequirePath(arguments, "project.read_file");
            string target = ProjectAccess.ResolveFile(root, relative, "project.read_file");
            if (!File.Exists(target))
            {
                throw new McpError.InvalidRequest("project.read_file: the file does not exist: " + relative);
            }

            string content = ReadText(target, "project.read_file");
            JsonObject payload = new JsonObject
            {
                ["path"] = relative,
                ["bytes"] = FileSize(target),
                ["content"] = content,
            };
            return ToolOutput.Untrusted(FileOrigin, payload);
        }

        private static ToolOutput HandleWrite(McpHost host, ChangeSetStore store, JsonNode arguments)
        {
            string root = ProjectAccess.ResolveRoot(host, arguments, "project.write_file");
            string relative = RequirePath(arguments, "project.write_file");
            string content = RequireContent(arguments);
            string target = ProjectAccess.ResolveFile(root, relative, "project.write_file");
            string sessionId = host.CurrentSessionId
                ?? throw new McpError.InvalidRequest("project.write_file requires an active MCP session");

            // The writer is content-generic, so script and assembly-definition
            // targets additionally require the edit-scripts scope: a session
            // that holds edit-content alone cannot rewrite the script pipeline.
            if (IsScriptPath(relative))
            {
                host.RequireScope(sessionId, Scope.EditScripts);
            }

            string before = File.Exists(target) ? ReadText(target, "project.write_file") : string.Empty;
            try
            {
                string? directory = Path.GetDirectoryName(target);
                if (!string.IsNullOrEmpty(directory))
                {
                    Directory.CreateDirectory(directory);
                }

                File.WriteAllText(target, content);
            }
            catch (Exception error) when (error is IOException || error is UnauthorizedAccessException)
            {
                throw new McpError.Io(error);
            }

            // The write is applied so an agent can author unattended; the change
            // set is the provenance record of that applied edit and stays
            // pending until a person reviews it.
            JsonObject operation = new JsonObject
            {
                ["tool"] = "project.write_file",
                ["path"] = relative,
                ["before"] = before,
                ["after"] = content,
            };
            ChangeSet set = store.Hold(host.GetSessionIdentity(sessionId).Agent, operation);

            JsonObject payload = new JsonObject
            {
                ["status"] = "applied",
                ["path"] = relative,
                ["bytes"] = content.Length,
                ["changeId"] = set.Id,
                ["changeSet"] = set.ToValue(),
            };
            return ToolOutput.Untrusted(FileOrigin, payload).WithAffectedIdentities(set.Id);
        }

        private static void Collect(string directory, string root, List<string> paths, ref bool truncated)
        {
            string[] entries;
            try
            {
                entries = Directory.GetFileSystemEntries(directory);
            }
            catch (Exception error) when (error is IOException || error is UnauthorizedAccessException)
            {
                return;
            }

            Array.Sort(entries, StringComparer.Ordinal);
            foreach (string entry in entries)
            {
                if (paths.Count >= MaxListEntries)
                {
                    truncated = true;
                    return;
                }

                if (Directory.Exists(entry))
                {
                    if (IsGeneratedDirectory(Path.GetFileName(entry)))
                    {
                        continue;
                    }

                    paths.Add(RelativePath(root, entry));
                    Collect(entry, root, paths, ref truncated);
                }
                else
                {
                    paths.Add(RelativePath(root, entry));
                }
            }
        }

        private static bool IsGeneratedDirectory(string name)
        {
            foreach (string generated in GeneratedDirectoryNames)
            {
                if (string.Equals(name, generated, StringComparison.OrdinalIgnoreCase))
                {
                    return true;
                }
            }

            return false;
        }

        private static bool IsScriptPath(string relativePath)
        {
            string extension = Path.GetExtension(relativePath);
            return string.Equals(extension, ".cs", StringComparison.OrdinalIgnoreCase)
                || string.Equals(extension, ".asmdef", StringComparison.OrdinalIgnoreCase)
                || string.Equals(extension, ".asmref", StringComparison.OrdinalIgnoreCase);
        }

        private static string RelativePath(string root, string full)
        {
            string relative = Path.GetRelativePath(root, full);
            return relative.Replace(Path.DirectorySeparatorChar, '/');
        }

        private static long FileSize(string full)
        {
            try
            {
                return new FileInfo(full).Length;
            }
            catch (Exception error) when (error is IOException || error is UnauthorizedAccessException)
            {
                return 0;
            }
        }

        private static string ReadText(string target, string toolName)
        {
            try
            {
                return File.ReadAllText(target);
            }
            catch (Exception error) when (error is IOException || error is UnauthorizedAccessException)
            {
                throw new McpError.Io(error);
            }
        }

        private static string RequirePath(JsonNode arguments, string toolName)
        {
            string? path = ProjectAccess.OptionalString(arguments, "path");
            if (path == null)
            {
                throw new McpError.InvalidParams(toolName + " requires a non-empty 'path' argument");
            }

            return path;
        }

        private static string RequireContent(JsonNode arguments)
        {
            if (arguments is JsonObject obj
                && obj["content"] is JsonValue value
                && value.TryGetValue(out string? content))
            {
                return content;
            }

            throw new McpError.InvalidParams("project.write_file requires a 'content' argument");
        }
    }
}
