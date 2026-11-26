using System;
using System.IO;
using System.Text.Json.Nodes;

namespace Unitas.Editor.Mcp
{
    // Shared project-path resolution for the authoring tools: every tool that
    // touches project content resolves the same project root and refuses any
    // path that leaves it.
    internal static class ProjectAccess
    {
        internal static string ResolveRoot(McpHost host, JsonNode arguments, string toolName)
        {
            string? requested = OptionalString(arguments, "projectPath");
            string root;
            if (!string.IsNullOrWhiteSpace(requested))
            {
                root = FullPath(requested!, toolName);
            }
            else
            {
                root = host.ProjectPath ?? FullPath(Directory.GetCurrentDirectory(), toolName);
            }

            if (!Directory.Exists(root))
            {
                throw new McpError.InvalidRequest(
                    toolName + " requires an existing project directory: " + root);
            }

            return root;
        }

        internal static string ResolveFile(string root, string relativePath, string toolName)
        {
            if (string.IsNullOrWhiteSpace(relativePath))
            {
                throw new McpError.InvalidParams(toolName + " requires a non-empty 'path' argument");
            }

            string candidate = FullPath(Path.Combine(root, relativePath), toolName);
            if (!AuditStore.IsPathInsideDirectory(candidate, root))
            {
                throw new McpError.InvalidRequest(
                    toolName + " refuses the path that leaves the project: " + relativePath);
            }

            return candidate;
        }

        internal static string? OptionalString(JsonNode arguments, string propertyName)
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

        private static string FullPath(string path, string toolName)
        {
            try
            {
                return Path.GetFullPath(path);
            }
            catch (Exception error) when (error is ArgumentException || error is NotSupportedException)
            {
                throw new McpError.InvalidRequest(toolName + " received an invalid path: " + error.Message);
            }
        }
    }
}
