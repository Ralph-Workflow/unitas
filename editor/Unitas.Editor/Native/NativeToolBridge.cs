using System;
using System.Collections.Generic;
using System.Text.Json;
using System.Text.Json.Nodes;

namespace Unitas.Editor.Mcp
{
    public static class NativeToolBridge
    {
        public static NativeLoadOutcome TryRegisterFromNativeLibrary(McpHost host, string? libraryPath = null)
        {
            ArgumentNullException.ThrowIfNull(host);
            if (!NativeLibraryLoader.TryOpen(
                libraryPath,
                out NativeLibraryLoader? loader,
                out NativeLoadOutcome failureOutcome)
                || loader == null)
            {
                return failureOutcome;
            }

            return RegisterFromSource(host, loader);
        }

        public static NativeLoadOutcome RegisterFromSource(McpHost host, INativeLibrarySource source)
        {
            ArgumentNullException.ThrowIfNull(host);
            ArgumentNullException.ThrowIfNull(source);

            IReadOnlyList<NativeToolDescriptor> descriptors;
            try
            {
                string manifestJson = source.DescribeJson();
                descriptors = ParseManifest(manifestJson);
            }
            catch (McpError error)
            {
                return NativeLoadOutcome.LoadFailed(error.Message);
            }

            try
            {
                host.RegisterNativeTools(
                    descriptors,
                    (toolName, arguments) => InvokeNativeTool(source, toolName, arguments));
            }
            catch (McpError error)
            {
                return NativeLoadOutcome.LoadFailed(error.Message);
            }

            return NativeLoadOutcome.Loaded(source.LibraryPath, descriptors.Count);
        }

        public static IReadOnlyList<NativeToolDescriptor> ParseManifest(string manifestJson)
        {
            ArgumentNullException.ThrowIfNull(manifestJson);
            JsonNode? parsed;
            try
            {
                parsed = JsonNode.Parse(manifestJson);
            }
            catch (JsonException error)
            {
                throw new McpError.ParseError("invalid native tool manifest JSON: " + error.Message);
            }

            if (!(parsed is JsonObject root) || !(root["tools"] is JsonArray toolsArray))
            {
                throw new McpError.ParseError("native tool manifest must contain a 'tools' array");
            }

            List<NativeToolDescriptor> descriptors = new List<NativeToolDescriptor>();
            foreach (JsonNode? item in toolsArray)
            {
                if (!(item is JsonObject toolObject))
                {
                    throw new McpError.ParseError("native tool descriptor must be a JSON object");
                }

                string name = RequireString(toolObject, "name");
                string description = RequireString(toolObject, "description");
                string requiredScopeRaw = RequireString(toolObject, "requiredScope");
                string origin = RequireString(toolObject, "origin");

                if (!Scopes.TryParse(requiredScopeRaw, out Scope requiredScope))
                {
                    throw new McpError.ParseError(
                        "native tool '" + name + "' declares unknown scope '" + requiredScopeRaw + "'");
                }

                JsonNode? schemaNode = toolObject["inputSchema"];
                if (schemaNode == null)
                {
                    throw new McpError.ParseError(
                        "native tool '" + name + "' is missing required 'inputSchema'");
                }

                descriptors.Add(new NativeToolDescriptor(
                    name,
                    description,
                    schemaNode.DeepClone(),
                    requiredScope,
                    origin));
            }

            return descriptors;
        }

        public static JsonNode ParseInvokeResponse(string toolName, string responseJson)
        {
            ArgumentNullException.ThrowIfNull(toolName);
            ArgumentNullException.ThrowIfNull(responseJson);

            JsonNode? parsed;
            try
            {
                parsed = JsonNode.Parse(responseJson);
            }
            catch (JsonException error)
            {
                throw new McpError.ParseError(
                    "native tool '" + toolName + "' returned invalid JSON: " + error.Message);
            }

            if (parsed == null)
            {
                return new JsonObject();
            }

            if (parsed is JsonObject map && map["error"] is JsonObject errorObject)
            {
                ThrowMappedError(toolName, errorObject);
            }

            return parsed;
        }

        private static JsonNode InvokeNativeTool(
            INativeLibrarySource source,
            string toolName,
            JsonNode arguments)
        {
            string argsJson = arguments.ToJsonString();
            string responseJson = source.InvokeJson(toolName, argsJson);
            return ParseInvokeResponse(toolName, responseJson);
        }

        private static string RequireString(JsonObject obj, string field)
        {
            if (obj[field] is JsonValue value
                && value.TryGetValue(out string? text)
                && !string.IsNullOrWhiteSpace(text))
            {
                return text;
            }

            throw new McpError.ParseError(
                "native tool descriptor is missing non-empty string field '" + field + "'");
        }

        private static void ThrowMappedError(string toolName, JsonObject errorObject)
        {
            string kind = ReadOptionalString(errorObject, "kind") ?? "internal";
            string message = ReadOptionalString(errorObject, "message")
                ?? ("native tool '" + toolName + "' failed");

            if (string.Equals(kind, "missing-scope", StringComparison.Ordinal))
            {
                string? scopeText = ReadOptionalString(errorObject, "scope");
                if (scopeText != null && Scopes.TryParse(scopeText, out Scope missingScope))
                {
                    throw new McpError.MissingScope(missingScope, "native-abi");
                }
            }

            if (string.Equals(kind, "unknown-tool", StringComparison.Ordinal))
            {
                string target = ReadOptionalString(errorObject, "tool") ?? toolName;
                throw new McpError.UnknownTool(target);
            }

            if (string.Equals(kind, "invalid-args", StringComparison.Ordinal))
            {
                throw new McpError.InvalidParams(message);
            }

            throw new McpError.InvalidRequest(kind + ": " + message);
        }

        private static string? ReadOptionalString(JsonObject obj, string field)
        {
            if (obj[field] is JsonValue value && value.TryGetValue(out string? text))
            {
                return text;
            }

            return null;
        }
    }
}
