using System;
using System.IO;
using System.Text.Json;
using System.Text.Json.Nodes;

namespace Unitas.Editor.Mcp
{
    public static class BuildTools
    {
        // Must match the LICENSING_RECORD_FILENAME the native build.player tool
        // writes next to the staged bundle (crates/unitas-editor-abi).
        private const string LicensingRecordFilename = "licensing-record.json";

        public static void Register(McpHost host)
        {
            ArgumentNullException.ThrowIfNull(host);

            host.RegisterTool(
                new ToolDefinition(
                    "build.licensing",
                    "Read the licensing record (governing GPLv3/commercial terms and third-party notices, LIC-001) that a unitas player bundle carries",
                    new JsonObject
                    {
                        ["type"] = "object",
                        ["properties"] = new JsonObject
                        {
                            ["bundlePath"] = new JsonObject
                            {
                                ["type"] = "string",
                            },
                        },
                        ["required"] = new JsonArray("bundlePath"),
                    },
                    Scope.ReadProject),
                arguments => ReadLicensingRecord(arguments));
        }

        private static ToolOutput ReadLicensingRecord(JsonNode arguments)
        {
            string bundlePath = RequiredBundlePath(arguments);
            if (!Directory.Exists(bundlePath))
            {
                throw new McpError.InvalidRequest("bundle directory does not exist: " + bundlePath);
            }

            string recordPath = Path.Combine(bundlePath, LicensingRecordFilename);
            if (!File.Exists(recordPath))
            {
                throw new McpError.InvalidRequest(
                    "bundle at '" + bundlePath + "' carries no licensing record at '" + recordPath + "'");
            }

            JsonNode? record;
            try
            {
                record = JsonNode.Parse(File.ReadAllText(recordPath));
            }
            catch (JsonException error)
            {
                throw new McpError.ParseError(
                    "licensing record at '" + recordPath + "' is not valid JSON: " + error.Message);
            }
            catch (Exception error) when (error is IOException || error is UnauthorizedAccessException)
            {
                throw new McpError.Io(error);
            }

            if (record is not JsonObject)
            {
                throw new McpError.ParseError(
                    "licensing record at '" + recordPath + "' must be a JSON object");
            }

            return ToolOutput.Untrusted("unitas://build/licensing", record);
        }

        private static string RequiredBundlePath(JsonNode arguments)
        {
            if (arguments is JsonObject argumentObject
                && argumentObject["bundlePath"] is JsonValue value
                && value.TryGetValue(out string? bundlePath)
                && !string.IsNullOrWhiteSpace(bundlePath))
            {
                return bundlePath;
            }

            throw new McpError.InvalidParams(
                "build.licensing requires a non-empty 'bundlePath' argument");
        }
    }
}
