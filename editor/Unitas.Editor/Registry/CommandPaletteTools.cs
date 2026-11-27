using System;
using System.Collections.Generic;
using System.Text.Json.Nodes;
using UnityEditor;

namespace Unitas.Editor.Mcp
{
    /// <summary>
    /// The command palette. Every editor command an agent can run is one entry: the commands
    /// of the tool registry and the menu commands the project registered. Each entry names its
    /// availability and, when it is unavailable, the reason it is unavailable.
    /// </summary>
    public static class CommandPaletteTools
    {
        public const string PaletteOrigin = "unitas://editor/commands";

        public static void Register(McpHost host)
        {
            ArgumentNullException.ThrowIfNull(host);

            host.RegisterTool(
                new ToolDefinition(
                    "editor.commands",
                    "Enumerate every editor command with its availability and, when it is unavailable, the reason",
                    new JsonObject
                    {
                        ["type"] = "object",
                        ["properties"] = new JsonObject
                        {
                            ["query"] = new JsonObject
                            {
                                ["type"] = "string",
                            },
                        },
                    },
                    Scope.ReadProject),
                arguments => HandleList(host, arguments));

            host.RegisterTool(
                new ToolDefinition(
                    "editor.invoke_command",
                    "Run one editor command by its command id; a registered tool runs with its own scope and a menu command runs the project code behind it",
                    new JsonObject
                    {
                        ["type"] = "object",
                        ["properties"] = new JsonObject
                        {
                            ["command"] = new JsonObject
                            {
                                ["type"] = "string",
                            },
                            ["arguments"] = new JsonObject
                            {
                                ["type"] = "object",
                            },
                        },
                        ["required"] = new JsonArray("command"),
                    },
                    Scope.ExecuteProjectCode),
                arguments => HandleInvoke(host, arguments));
        }

        private static ToolOutput HandleList(McpHost host, JsonNode arguments)
        {
            string? query = ProjectAccess.OptionalString(arguments, "query");
            string? sessionId = host.CurrentSessionId;

            JsonArray commands = new JsonArray();
            foreach (ToolDefinition definition in host.ListTools())
            {
                bool available = true;
                string disabledReason = string.Empty;
                if (sessionId != null)
                {
                    try
                    {
                        host.RequireScope(sessionId, definition.RequiredScope);
                    }
                    catch (McpError)
                    {
                        available = false;
                        disabledReason = "the session does not hold the "
                            + Scopes.AsString(definition.RequiredScope) + " scope";
                    }
                }

                if (!Matches(query, definition.Name, definition.Description))
                {
                    continue;
                }

                commands.Add(new JsonObject
                {
                    ["id"] = definition.Name,
                    ["kind"] = "tool",
                    ["displayName"] = definition.Name,
                    ["description"] = definition.Description,
                    ["requiredScope"] = Scopes.AsString(definition.RequiredScope),
                    ["reversibility"] = definition.EffectiveReversibility,
                    ["reversible"] = definition.IsReversible,
                    ["available"] = available,
                    ["disabledReason"] = disabledReason,
                });
            }

            foreach (MenuCommandDescriptor menu in MenuItem.GetCommandPaletteEntries())
            {
                if (!Matches(query, menu.ToolId, menu.MenuPath))
                {
                    continue;
                }

                commands.Add(new JsonObject
                {
                    ["id"] = menu.ToolId,
                    ["kind"] = "menu",
                    ["displayName"] = menu.DisplayName,
                    ["description"] = menu.MenuPath,
                    ["menuPath"] = menu.MenuPath,
                    ["shortcut"] = menu.Shortcut,
                    ["priority"] = menu.Priority,
                    ["available"] = menu.IsEnabled,
                    ["disabledReason"] = menu.DisabledReason,
                });
            }

            JsonObject payload = new JsonObject
            {
                ["commands"] = commands,
                ["commandCount"] = commands.Count,
            };
            return ToolOutput.Untrusted(PaletteOrigin, payload);
        }

        private static ToolOutput HandleInvoke(McpHost host, JsonNode arguments)
        {
            string command = ProjectAccess.OptionalString(arguments, "command")
                ?? throw new McpError.InvalidParams("editor.invoke_command requires a non-empty 'command' argument");
            string sessionId = host.CurrentSessionId
                ?? throw new McpError.InvalidRequest("editor.invoke_command requires an active MCP session");
            JsonObject commandArguments = arguments["arguments"] as JsonObject ?? new JsonObject();

            foreach (ToolDefinition definition in host.ListTools())
            {
                if (string.Equals(definition.Name, command, StringComparison.Ordinal))
                {
                    JsonNode result = host.CallTool(sessionId, command, commandArguments);
                    JsonObject payload = new JsonObject
                    {
                        ["command"] = command,
                        ["kind"] = "tool",
                        ["ok"] = true,
                        ["status"] = "applied",
                        ["result"] = result.DeepClone(),
                    };
                    return ToolOutput.Untrusted(PaletteOrigin, payload);
                }
            }

            MenuCommandDescriptor? menu = MenuItem.FindCommand(command);
            if (menu == null)
            {
                throw new McpError.InvalidRequest("editor.invoke_command does not know the command '" + command + "'");
            }

            MenuCommandInvocationResult invocation = MenuItem.Invoke(command, sessionId);
            if (!invocation.Succeeded)
            {
                throw new McpError.InvalidRequest(
                    "editor.invoke_command refuses '" + command + "': " + invocation.RefusalReason);
            }

            JsonObject menuPayload = new JsonObject
            {
                ["command"] = invocation.ToolId,
                ["kind"] = "menu",
                ["menuPath"] = invocation.MenuPath,
                ["ok"] = true,
                ["status"] = "applied",
                ["mutationCount"] = invocation.Mutations.Count,
            };
            return ToolOutput.Untrusted(PaletteOrigin, menuPayload);
        }

        // A palette search matches the command id, its display name and its description.
        private static bool Matches(string? query, string id, string description)
        {
            if (string.IsNullOrWhiteSpace(query))
            {
                return true;
            }

            string wanted = query.Trim();
            return id.Contains(wanted, StringComparison.OrdinalIgnoreCase)
                || description.Contains(wanted, StringComparison.OrdinalIgnoreCase);
        }
    }
}
