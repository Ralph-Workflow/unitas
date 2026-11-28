using System;
using System.IO;
using System.Security.Cryptography;
using System.Text;
using System.Text.Json.Nodes;

namespace Unitas.Editor.Mcp
{
    public static class AssetTools
    {
        public const string ImportOrigin = "unitas://assets/import";

        private const string ToolName = "assets.import";
        private const string AssetsPrefix = "Assets/";
        private const string MetaExtension = ".meta";
        private const int MaxPayloadBytes = 16 * 1024 * 1024;
        private const int GuidHexLength = 32;
        private const string Base64EncodingTag = "base64:";

        public static void Register(McpHost host, ChangeSetStore store)
        {
            ArgumentNullException.ThrowIfNull(host);
            ArgumentNullException.ThrowIfNull(store);

            host.RegisterTool(
                new ToolDefinition(
                    ToolName,
                    "Create or import a text or binary project asset under Assets/, write its Unity .meta file, and record the edit in the agent change set",
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
                            ["text"] = new JsonObject
                            {
                                ["type"] = "string",
                            },
                            ["base64"] = new JsonObject
                            {
                                ["type"] = "string",
                            },
                            ["guid"] = new JsonObject
                            {
                                ["type"] = "string",
                            },
                        },
                        ["required"] = new JsonArray("path"),
                    },
                    Scope.EditContent),
                arguments => HandleImport(host, store, arguments));

            store.RegisterApplier(ToolName, operation => ApplyHeldImport(host, operation));
        }

        private static ToolOutput HandleImport(McpHost host, ChangeSetStore store, JsonNode arguments)
        {
            string root = ProjectAccess.ResolveRoot(host, arguments, ToolName);
            string relativePath = ValidateAndNormalizeAssetPath(root, arguments);
            string sessionId = host.CurrentSessionId
                ?? throw new McpError.InvalidRequest(ToolName + " requires an active MCP session");

            if (IsScriptPath(relativePath))
            {
                host.RequireScope(sessionId, Scope.EditScripts);
            }

            store.EnsureAgentWritesAllowed(ToolName, arguments);

            (byte[] payloadBytes, bool isText, string? rawText) = ResolvePayload(arguments, relativePath);
            string targetFile = Path.Combine(root, relativePath.Replace('/', Path.DirectorySeparatorChar));
            string metaRelativePath = relativePath + MetaExtension;
            string metaFile = targetFile + MetaExtension;

            string guid = ResolveGuid(arguments, relativePath, metaFile);
            (string importerName, string metaContent) = BuildMetaContent(relativePath, guid);

            string beforeAsset = CaptureExistingFileState(targetFile, isText);
            string beforeMeta = File.Exists(metaFile) ? ReadTextFile(metaFile) : string.Empty;

            string afterAsset = isText && rawText != null
                ? rawText
                : Base64EncodingTag + Convert.ToBase64String(payloadBytes);

            string? requestedProject = ProjectAccess.OptionalString(arguments, "projectPath");
            JsonObject assetOperation = new JsonObject
            {
                ["tool"] = ToolName,
                ["path"] = relativePath,
                ["encoding"] = isText ? "utf-8" : "base64",
                ["before"] = beforeAsset,
                ["after"] = afterAsset,
            };
            JsonObject metaOperation = new JsonObject
            {
                ["tool"] = ToolName,
                ["path"] = metaRelativePath,
                ["encoding"] = "utf-8",
                ["before"] = beforeMeta,
                ["after"] = metaContent,
            };
            if (requestedProject != null)
            {
                assetOperation["projectPath"] = requestedProject;
                metaOperation["projectPath"] = requestedProject;
            }

            string author = host.GetSessionIdentity(sessionId).Agent;
            ChangeSet set = store.Hold(author, assetOperation);
            store.AddOperation(set, metaOperation);

            bool staged = store.PolicyForArguments(arguments).RequireApproval;

            JsonArray diagnostics = staged
                ? new JsonArray()
                : RefreshNativeAssetInventory(host, root, relativePath);

            JsonObject payload = new JsonObject
            {
                ["status"] = staged ? "pending-review" : "imported",
                ["path"] = relativePath,
                ["metaPath"] = metaRelativePath,
                ["guid"] = guid,
                ["importer"] = importerName,
                ["bytes"] = payloadBytes.Length,
                ["encoding"] = isText ? "utf-8" : "base64",
                ["diagnostics"] = diagnostics,
                ["changeId"] = set.Id,
                ["changeSet"] = set.ToValue(),
            };
            return ToolOutput.Untrusted(ImportOrigin, payload).WithAffectedIdentities(set.Id);
        }

        private static string ValidateAndNormalizeAssetPath(string root, JsonNode arguments)
        {
            string? rawPath = ProjectAccess.OptionalString(arguments, "path");
            if (string.IsNullOrWhiteSpace(rawPath))
            {
                throw new McpError.InvalidParams(ToolName + " requires a non-empty 'path' argument");
            }

            string trimmed = rawPath!.Trim();
            if (Path.IsPathRooted(trimmed) || trimmed.StartsWith('/') || trimmed.StartsWith('\\'))
            {
                throw new McpError.InvalidRequest(
                    ToolName + " failed for '" + rawPath + "': path must be project-relative under Assets/, not an absolute path");
            }

            string normalized = trimmed.Replace('\\', '/');
            string[] segments = normalized.Split('/');
            foreach (string segment in segments)
            {
                if (segment.Length == 0 || string.Equals(segment, ".", StringComparison.Ordinal) || string.Equals(segment, "..", StringComparison.Ordinal))
                {
                    throw new McpError.InvalidRequest(
                        ToolName + " failed for '" + rawPath + "': path must not contain empty, '.', or '..' traversal segments");
                }
            }

            if (!normalized.StartsWith(AssetsPrefix, StringComparison.Ordinal) || normalized.Length <= AssetsPrefix.Length)
            {
                throw new McpError.InvalidRequest(
                    ToolName + " failed for '" + rawPath + "': path must reside inside Assets/");
            }

            if (normalized.EndsWith(MetaExtension, StringComparison.OrdinalIgnoreCase))
            {
                throw new McpError.InvalidRequest(
                    ToolName + " failed for '" + rawPath + "': target path must name an asset file rather than a .meta sidecar");
            }

            string assetsRoot = Path.GetFullPath(Path.Combine(root, "Assets"));
            string candidate = ProjectAccess.ResolveFile(root, normalized, ToolName);
            if (!AuditStore.IsPathInsideDirectory(candidate, assetsRoot)
                || string.Equals(
                    Path.TrimEndingDirectorySeparator(candidate),
                    Path.TrimEndingDirectorySeparator(assetsRoot),
                    StringComparison.Ordinal))
            {
                throw new McpError.InvalidRequest(
                    ToolName + " failed for '" + rawPath + "': path escapes Assets/");
            }

            return normalized;
        }

        private static (byte[] Bytes, bool IsText, string? RawText) ResolvePayload(JsonNode arguments, string relativePath)
        {
            if (arguments is not JsonObject obj)
            {
                throw new McpError.InvalidParams(
                    ToolName + " failed for '" + relativePath + "': arguments must be a JSON object");
            }

            bool hasTextKey = obj.ContainsKey("text") && obj["text"] != null;
            bool hasBase64Key = obj.ContainsKey("base64") && obj["base64"] != null;

            if (hasTextKey && hasBase64Key)
            {
                throw new McpError.InvalidParams(
                    ToolName + " failed for '" + relativePath + "': specify either 'text' or 'base64', not both");
            }

            if (!hasTextKey && !hasBase64Key)
            {
                throw new McpError.InvalidParams(
                    ToolName + " failed for '" + relativePath + "': one of 'text' or 'base64' is required");
            }

            if (hasTextKey)
            {
                if (obj["text"] is not JsonValue textVal || !textVal.TryGetValue(out string? text) || text == null)
                {
                    throw new McpError.InvalidParams(
                        ToolName + " failed for '" + relativePath + "': 'text' must be a UTF-8 string");
                }

                byte[] utf8;
                try
                {
                    UTF8Encoding strictUtf8 = new UTF8Encoding(encoderShouldEmitUTF8Identifier: false, throwOnInvalidBytes: true);
                    utf8 = strictUtf8.GetBytes(text);
                }
                catch (EncoderFallbackException error)
                {
                    throw new McpError.InvalidParams(
                        ToolName + " failed for '" + relativePath + "': invalid UTF-8 text payload: " + error.Message);
                }

                EnsurePayloadWithinLimit(utf8.Length, relativePath);
                return (utf8, true, text);
            }

            if (obj["base64"] is not JsonValue base64Val || !base64Val.TryGetValue(out string? base64Text) || base64Text == null)
            {
                throw new McpError.InvalidParams(
                    ToolName + " failed for '" + relativePath + "': 'base64' must be a base64-encoded string");
            }

            byte[] decoded;
            try
            {
                decoded = Convert.FromBase64String(base64Text);
            }
            catch (FormatException error)
            {
                throw new McpError.InvalidParams(
                    ToolName + " failed for '" + relativePath + "': malformed base64 payload: " + error.Message);
            }

            EnsurePayloadWithinLimit(decoded.Length, relativePath);
            return (decoded, false, null);
        }

        private static void EnsurePayloadWithinLimit(int byteCount, string relativePath)
        {
            if (byteCount > MaxPayloadBytes)
            {
                throw new McpError.InvalidParams(
                    ToolName + " failed for '" + relativePath + "': payload size " + byteCount + " bytes exceeds maximum of " + MaxPayloadBytes + " bytes");
            }
        }

        private static string ResolveGuid(JsonNode arguments, string relativePath, string metaFile)
        {
            if (arguments is JsonObject obj && obj.ContainsKey("guid") && obj["guid"] != null)
            {
                if (obj["guid"] is not JsonValue guidValue || !guidValue.TryGetValue(out string? explicitGuid) || string.IsNullOrWhiteSpace(explicitGuid))
                {
                    throw new McpError.InvalidParams(
                        ToolName + " failed for '" + relativePath + "': 'guid' must be a 32-character lowercase hexadecimal string");
                }

                string candidate = explicitGuid!.Trim();
                if (!IsValidLowerHexGuid(candidate))
                {
                    throw new McpError.InvalidParams(
                        ToolName + " failed for '" + relativePath + "': invalid guid '" + candidate + "' (expected 32 lowercase hexadecimal characters)");
                }

                return candidate;
            }

            if (File.Exists(metaFile))
            {
                string? existingGuid = TryParseGuidFromMeta(ReadTextFile(metaFile));
                if (existingGuid != null && IsValidLowerHexGuid(existingGuid))
                {
                    return existingGuid;
                }
            }

            return GenerateDeterministicGuid(relativePath);
        }

        private static string? TryParseGuidFromMeta(string metaText)
        {
            foreach (string rawLine in metaText.Split('\n'))
            {
                string line = rawLine.Trim();
                if (line.StartsWith("guid:", StringComparison.Ordinal))
                {
                    string value = line.Substring("guid:".Length).Trim();
                    if (value.Length > 0)
                    {
                        return value;
                    }
                }
            }

            return null;
        }

        private static string GenerateDeterministicGuid(string relativePath)
        {
            byte[] hash = SHA256.HashData(Encoding.UTF8.GetBytes("unitas-asset-guid:" + relativePath));
            StringBuilder builder = new StringBuilder(GuidHexLength);
            for (int index = 0; index < 16; index++)
            {
                builder.Append(hash[index].ToString("x2", System.Globalization.CultureInfo.InvariantCulture));
            }

            return builder.ToString();
        }

        private static bool IsValidLowerHexGuid(string guid)
        {
            if (guid.Length != GuidHexLength)
            {
                return false;
            }

            foreach (char ch in guid)
            {
                bool isDigit = ch >= '0' && ch <= '9';
                bool isLowerHex = ch >= 'a' && ch <= 'f';
                if (!isDigit && !isLowerHex)
                {
                    return false;
                }
            }

            return true;
        }

        private static (string ImporterName, string MetaContent) BuildMetaContent(string relativePath, string guid)
        {
            string extension = Path.GetExtension(relativePath).ToLowerInvariant();
            string header = "fileFormatVersion: 2\nguid: " + guid + "\n";
            switch (extension)
            {
                case ".mat":
                    return (
                        "NativeFormatImporter",
                        header
                        + "NativeFormatImporter:\n"
                        + "  externalObjects: {}\n"
                        + "  mainObjectFileID: 2100000\n"
                        + "  userData:\n"
                        + "  assetBundleName:\n"
                        + "  assetBundleVariant:\n");
                case ".physicmaterial":
                    return (
                        "NativeFormatImporter",
                        header
                        + "NativeFormatImporter:\n"
                        + "  externalObjects: {}\n"
                        + "  mainObjectFileID: 13400000\n"
                        + "  userData:\n"
                        + "  assetBundleName:\n"
                        + "  assetBundleVariant:\n");
                case ".prefab":
                    return (
                        "NativeFormatImporter",
                        header
                        + "NativeFormatImporter:\n"
                        + "  externalObjects: {}\n"
                        + "  mainObjectFileID: 100100000\n"
                        + "  userData:\n"
                        + "  assetBundleName:\n"
                        + "  assetBundleVariant:\n");
                case ".asset":
                    return (
                        "NativeFormatImporter",
                        header
                        + "NativeFormatImporter:\n"
                        + "  externalObjects: {}\n"
                        + "  mainObjectFileID: 11400000\n"
                        + "  userData:\n"
                        + "  assetBundleName:\n"
                        + "  assetBundleVariant:\n");
                case ".wav":
                case ".wave":
                case ".ogg":
                case ".mp3":
                case ".aif":
                case ".aiff":
                case ".aifc":
                    return (
                        "AudioImporter",
                        header
                        + "AudioImporter:\n"
                        + "  externalObjects: {}\n"
                        + "  serializedVersion: 2\n"
                        + "  defaultSettings:\n"
                        + "    serializedVersion: 2\n"
                        + "    loadType: 0\n"
                        + "    sampleRateSetting: 0\n"
                        + "    compressionFormat: 1\n"
                        + "    quality: 1\n"
                        + "  userData:\n"
                        + "  assetBundleName:\n"
                        + "  assetBundleVariant:\n");
                case ".png":
                case ".jpg":
                case ".jpeg":
                case ".tga":
                case ".targa":
                case ".bmp":
                case ".gif":
                case ".psd":
                case ".exr":
                case ".hdr":
                    return (
                        "TextureImporter",
                        header
                        + "TextureImporter:\n"
                        + "  internalIDToNameTable: []\n"
                        + "  externalObjects: {}\n"
                        + "  serializedVersion: 2\n"
                        + "  mipmaps:\n"
                        + "    mipMapMode: 0\n"
                        + "    enableMipMap: 1\n"
                        + "  isReadable: 0\n"
                        + "  sRGBTexture: 1\n"
                        + "  alphaIsTransparency: 0\n"
                        + "  textureType: 0\n"
                        + "  maxTextureSize: 2048\n"
                        + "  mipmapEnabled: 1\n"
                        + "  spriteMode: 0\n"
                        + "  wrapMode: 0\n"
                        + "  filterMode: 1\n"
                        + "  userData:\n"
                        + "  assetBundleName:\n"
                        + "  assetBundleVariant:\n");
                case ".fbx":
                case ".obj":
                case ".gltf":
                case ".glb":
                    return (
                        "ModelImporter",
                        header
                        + "ModelImporter:\n"
                        + "  externalObjects: {}\n"
                        + "  serializedVersion: 2\n"
                        + "  globalScale: 1\n"
                        + "  useFileScale: 1\n"
                        + "  meshCompression: 0\n"
                        + "  materialImportMode: 1\n"
                        + "  bakeAxisConversion: 0\n"
                        + "  userData:\n"
                        + "  assetBundleName:\n"
                        + "  assetBundleVariant:\n");
                case ".ttf":
                case ".otf":
                    return (
                        "TrueTypeFontImporter",
                        header
                        + "TrueTypeFontImporter:\n"
                        + "  externalObjects: {}\n"
                        + "  serializedVersion: 2\n"
                        + "  fontSize: 16\n"
                        + "  forceTextureCase: -2\n"
                        + "  characterSpacing: 0\n"
                        + "  characterPadding: 2\n"
                        + "  includeFontData: 1\n"
                        + "  fontNames: []\n"
                        + "  fallbackFontReferences: []\n"
                        + "  customCharacters:\n"
                        + "  fontRenderingMode: 0\n"
                        + "  ascentCalculationMode: 1\n"
                        + "  descentCalculationMode: 1\n"
                        + "  fontDisplayName:\n"
                        + "  userData:\n"
                        + "  assetBundleName:\n"
                        + "  assetBundleVariant:\n");
                case ".cs":
                    return (
                        "MonoImporter",
                        header
                        + "MonoImporter:\n"
                        + "  externalObjects: {}\n"
                        + "  serializedVersion: 2\n"
                        + "  defaultReferences: []\n"
                        + "  executionOrder: 0\n"
                        + "  icon: {instanceID: 0}\n"
                        + "  userData:\n"
                        + "  assetBundleName:\n"
                        + "  assetBundleVariant:\n");
                case ".asmdef":
                    return (
                        "AssemblyDefinitionImporter",
                        header
                        + "AssemblyDefinitionImporter:\n"
                        + "  externalObjects: {}\n"
                        + "  userData:\n"
                        + "  assetBundleName:\n"
                        + "  assetBundleVariant:\n");
                case ".asmref":
                    return (
                        "AssemblyDefinitionReferenceImporter",
                        header
                        + "AssemblyDefinitionReferenceImporter:\n"
                        + "  externalObjects: {}\n"
                        + "  userData:\n"
                        + "  assetBundleName:\n"
                        + "  assetBundleVariant:\n");
                case ".txt":
                case ".json":
                case ".bytes":
                case ".csv":
                case ".xml":
                case ".html":
                case ".htm":
                case ".yaml":
                case ".fnt":
                case ".md":
                    return (
                        "TextScriptImporter",
                        header
                        + "TextScriptImporter:\n"
                        + "  externalObjects: {}\n"
                        + "  userData:\n"
                        + "  assetBundleName:\n"
                        + "  assetBundleVariant:\n");
                default:
                    return (
                        "DefaultImporter",
                        header
                        + "DefaultImporter:\n"
                        + "  externalObjects: {}\n"
                        + "  userData:\n"
                        + "  assetBundleName:\n"
                        + "  assetBundleVariant:\n");
            }
        }

        private static string CaptureExistingFileState(string targetFile, bool isText)
        {
            if (!File.Exists(targetFile))
            {
                return string.Empty;
            }

            if (isText)
            {
                return ReadTextFile(targetFile);
            }

            try
            {
                return Base64EncodingTag + Convert.ToBase64String(File.ReadAllBytes(targetFile));
            }
            catch (Exception error) when (error is IOException || error is UnauthorizedAccessException)
            {
                throw new McpError.Io(error);
            }
        }

        private static string ReadTextFile(string path)
        {
            try
            {
                return File.ReadAllText(path);
            }
            catch (Exception error) when (error is IOException || error is UnauthorizedAccessException)
            {
                throw new McpError.Io(error);
            }
        }

        private static JsonObject ApplyHeldImport(McpHost host, JsonObject operation)
        {
            string root = ProjectAccess.ResolveRoot(host, operation, ToolName);
            if (operation["path"] is not JsonValue pathValue
                || !pathValue.TryGetValue(out string? relative)
                || string.IsNullOrWhiteSpace(relative))
            {
                throw new McpError.InvalidParams(ToolName + " recorded an operation without a path");
            }

            if (operation["after"] is not JsonValue afterValue || !afterValue.TryGetValue(out string? after))
            {
                throw new McpError.InvalidParams(ToolName + " recorded an operation without its after text");
            }

            bool isText = string.Equals(
                operation["encoding"]?.GetValue<string>(),
                "utf-8",
                StringComparison.Ordinal);
            if (!isText && !after.StartsWith(Base64EncodingTag, StringComparison.Ordinal))
            {
                throw new McpError.InvalidParams(ToolName + " recorded a binary operation without a base64 payload");
            }

            string targetFile = ProjectAccess.ResolveFile(root, relative, ToolName);
            WriteImportedFile(targetFile, after, isText);
            return operation;
        }

        private static void WriteImportedFile(string targetFile, string after, bool isText)
        {
            try
            {
                string? directory = Path.GetDirectoryName(targetFile);
                if (!string.IsNullOrEmpty(directory))
                {
                    Directory.CreateDirectory(directory);
                }

                if (isText)
                {
                    File.WriteAllText(targetFile, after);
                }
                else
                {
                    File.WriteAllBytes(targetFile, Convert.FromBase64String(after.Substring(Base64EncodingTag.Length)));
                }
            }
            catch (Exception error) when (
                error is IOException || error is UnauthorizedAccessException || error is FormatException)
            {
                throw new McpError.Io(error);
            }
        }

        private static JsonArray RefreshNativeAssetInventory(McpHost host, string root, string relativePath)
        {
            JsonArray diagnostics = new JsonArray();
            string? sessionId = host.CurrentSessionId;
            if (sessionId == null)
            {
                return diagnostics;
            }

            bool hasDiagnosticsTool = false;
            foreach (ToolDefinition tool in host.ListTools())
            {
                if (string.Equals(tool.Name, "diagnostics.list", StringComparison.Ordinal))
                {
                    hasDiagnosticsTool = true;
                    break;
                }
            }

            if (!hasDiagnosticsTool)
            {
                return diagnostics;
            }

            try
            {
                host.RequireScope(sessionId, Scope.ReadProject);
                JsonNode response = host.CallTool(
                    sessionId,
                    "diagnostics.list",
                    new JsonObject
                    {
                        ["projectPath"] = root,
                    });
                if (response["content"]?["diagnostics"] is JsonArray entries)
                {
                    foreach (JsonNode? entry in entries)
                    {
                        string? path = entry?["path"]?.GetValue<string>();
                        if (string.Equals(path, relativePath, StringComparison.Ordinal))
                        {
                            diagnostics.Add(entry!.DeepClone());
                        }
                    }
                }
            }
            catch (McpError)
            {
            }

            return diagnostics;
        }

        private static bool IsScriptPath(string relativePath)
        {
            string extension = Path.GetExtension(relativePath);
            return string.Equals(extension, ".cs", StringComparison.OrdinalIgnoreCase)
                || string.Equals(extension, ".asmdef", StringComparison.OrdinalIgnoreCase)
                || string.Equals(extension, ".asmref", StringComparison.OrdinalIgnoreCase);
        }
    }
}
