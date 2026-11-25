using System;
using System.IO;
using System.Text;
using System.Text.Json.Nodes;

namespace Unitas.Editor.Mcp
{
    public static class AuditTools
    {
        private const string ImmutableAuditMessage =
            "audit records are append-only and cannot be altered or deleted by any session (AUT-008)";

        public static void Register(McpHost host)
        {
            ArgumentNullException.ThrowIfNull(host);

            host.RegisterTool(
                new ToolDefinition(
                    "audit.export",
                    "Export the per-project audit log (person-only, AUT-008)",
                    new JsonObject
                    {
                        ["type"] = "object",
                        ["properties"] = new JsonObject
                        {
                            ["personCredential"] = new JsonObject
                            {
                                ["type"] = "string",
                            },
                            ["outputPath"] = new JsonObject
                            {
                                ["type"] = "string",
                            },
                        },
                    },
                    Scope.ReadProject),
                arguments => ExportAuditLog(host, arguments));

            host.RegisterTool(
                new ToolDefinition(
                    "audit.delete",
                    "Refused operation: audit records are append-only (AUT-008)",
                    EmptyObjectSchema(),
                    Scope.ReadProject),
                _ => throw new McpError.InvalidRequest(ImmutableAuditMessage));

            host.RegisterTool(
                new ToolDefinition(
                    "audit.clear",
                    "Refused operation: audit records are append-only (AUT-008)",
                    EmptyObjectSchema(),
                    Scope.ReadProject),
                _ => throw new McpError.InvalidRequest(ImmutableAuditMessage));

            host.RegisterTool(
                new ToolDefinition(
                    "audit.alter",
                    "Refused operation: audit records are append-only (AUT-008)",
                    EmptyObjectSchema(),
                    Scope.ReadProject),
                _ => throw new McpError.InvalidRequest(ImmutableAuditMessage));
        }

        private static ToolOutput ExportAuditLog(McpHost host, JsonNode arguments)
        {
            string? sessionId = host.CurrentSessionId;
            string? transportName = sessionId != null
                ? host.GetSessionIdentity(sessionId).Client
                : null;

            JsonObject? argObject = arguments as JsonObject;
            string? token = argObject?["personCredential"]?.GetValue<string>();
            host.PersonCredential.RequirePerson(token, transportName, "audit.export");

            JsonArray exported = host.Audit.ExportJson();
            string? outputPath = argObject?["outputPath"]?.GetValue<string>();
            if (!string.IsNullOrWhiteSpace(outputPath))
            {
                string fullOutputPath = Path.GetFullPath(outputPath);
                AuditStore.EnsurePathOutsideProject(
                    fullOutputPath,
                    host.Audit.Store?.ProtectedProjectPath,
                    "audit export path must not write provenance into project files (AUT-008)");

                try
                {
                    string? parentDirectory = Path.GetDirectoryName(fullOutputPath);
                    if (!string.IsNullOrEmpty(parentDirectory))
                    {
                        Directory.CreateDirectory(parentDirectory);
                    }

                    File.WriteAllText(fullOutputPath, exported.ToJsonString(), Encoding.UTF8);
                }
                catch (Exception error) when (error is IOException || error is UnauthorizedAccessException)
                {
                    throw new McpError.Io(error);
                }
            }

            return ToolOutput.Untrusted(
                "unitas://audit/log",
                new JsonObject
                {
                    ["count"] = exported.Count,
                    ["records"] = exported,
                });
        }

        private static JsonObject EmptyObjectSchema()
        {
            return new JsonObject
            {
                ["type"] = "object",
                ["properties"] = new JsonObject(),
            };
        }
    }
}
