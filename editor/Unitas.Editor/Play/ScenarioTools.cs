using System;
using System.Collections.Generic;
using System.IO;
using System.Runtime.Loader;
using System.Text.Json;
using System.Text.Json.Nodes;
using Unitas.Bridge.Playtest;
using Unitas.Bridge.Scripting;
using Unitas.Scripting;
using UnityEngine;

namespace Unitas.Editor.Mcp
{
    public static class ScenarioTools
    {
        public const string ScenarioRunOrigin = "unitas://scenario/run";

        private const int PathSearchDepth = 8;
        private const string ScriptAssemblyFileName = "Assembly-CSharp.dll";
        private const string ScriptAssemblyFolderName = "UnitasScriptAssemblies";
        private const string EditorToolStep = "editor_tool";
        private const string EditorFileDigestStep = "editor_file_digest";
        private const string EditorAssertFileUnchangedStep = "editor_assert_file_unchanged";
        private const string EditorAssertFileChangedStep = "editor_assert_file_changed";

        private static readonly string[] EditorStepKinds =
        {
            EditorToolStep,
            EditorFileDigestStep,
            EditorAssertFileUnchangedStep,
            EditorAssertFileChangedStep,
        };

        private sealed class EditorStep
        {
            public EditorStep(string kind, JsonObject body)
            {
                Kind = kind;
                Body = body;
            }

            public string Kind { get; }

            public JsonObject Body { get; }
        }

        public static void Register(McpHost host)
        {
            ArgumentNullException.ThrowIfNull(host);
            host.RegisterTool(
                new ToolDefinition(
                    "scenario.run",
                    "Run a declared scenario document (untrusted project text) and return structured per-step and per-checkpoint results",
                    new JsonObject
                    {
                        ["type"] = "object",
                        ["properties"] = new JsonObject
                        {
                            ["scenarioPath"] = new JsonObject
                            {
                                ["type"] = "string",
                            },
                            ["projectPath"] = new JsonObject
                            {
                                ["type"] = "string",
                            },
                        },
                        ["required"] = new JsonArray("scenarioPath"),
                    },
                    Scope.ExecuteProjectCode),
                arguments => HandleRun(host, arguments));
        }

        private static ToolOutput HandleRun(McpHost host, JsonNode arguments)
        {
            string callerSessionId = RequireCallerSession(host);
            string scenarioPath = RequireScenarioPath(arguments);
            string? projectPath = ExtractOptionalString(arguments, "projectPath");
            string operationId = host.StartOperation(callerSessionId, "scenario.run", Scope.ExecuteProjectCode);
            host.Operations.UpdateStage(operationId, "running");
            System.Threading.SynchronizationContext? previousContext = System.Threading.SynchronizationContext.Current;
            try
            {
                string scenarioText = LoadScenarioText(scenarioPath, projectPath);
                string playText = SplitEditorSteps(scenarioText, out List<EditorStep> editorSteps);
                ScenarioDocument document = ParsePlayDocument(playText);
                string? projectRoot = ResolveScenarioProjectRoot(document, projectPath);
                JsonArray editorResults = RunEditorSteps(host, callerSessionId, editorSteps, projectRoot);
                RegisterProjectGlue(document, projectPath);
                PurgeStaleSceneObjects();
                ScenarioResult result = new ScenarioRunner().Run(document);
                JsonObject payload = BuildPayload(result);
                payload["editorSteps"] = editorResults;
                host.CompleteOperation(operationId, ScenarioRunOrigin, payload);
                return ToolOutput.Untrusted(ScenarioRunOrigin, payload)
                    .WithOperation(operationId);
            }
            catch (McpError error)
            {
                host.FailOperation(operationId, error.Message);
                throw;
            }
            catch (Exception error)
            {
                host.FailOperation(operationId, error.Message);
                throw new McpError.InvalidRequest(error.Message);
            }
            finally
            {
                System.Threading.SynchronizationContext.SetSynchronizationContext(previousContext);
            }
        }

        private static string LoadScenarioText(string scenarioPath, string? projectPath)
        {
            string root = string.IsNullOrWhiteSpace(projectPath)
                ? Directory.GetCurrentDirectory()
                : Path.GetFullPath(projectPath);
            string? resolved = ResolveExistingPath(scenarioPath, root);
            if (resolved == null)
            {
                throw new McpError.InvalidRequest("scenario file does not exist: " + scenarioPath);
            }

            string text = File.ReadAllText(resolved);
            string scenarioDirectory = Path.GetDirectoryName(resolved) ?? root;
            return ResolveEmbeddedPaths(text, scenarioDirectory);
        }

        private static ScenarioDocument ParsePlayDocument(string json)
        {
            try
            {
                return ScenarioDocument.Parse(json);
            }
            catch (ScenarioFormatException error)
            {
                throw new McpError.InvalidRequest("scenario document is invalid: " + error.Message);
            }
        }

        private static string? ResolveScenarioProjectRoot(ScenarioDocument document, string? projectPath)
        {
            string root = string.IsNullOrWhiteSpace(projectPath)
                ? Directory.GetCurrentDirectory()
                : Path.GetFullPath(projectPath);
            return ResolveProjectRoot(document, root);
        }

        // Editor steps are the authoring half of a scenario: they run the
        // editor's own commands before the play phase starts, so a scenario can
        // edit a scene, prove the edit landed, and undo it again.
        private static string SplitEditorSteps(string scenarioText, out List<EditorStep> editorSteps)
        {
            editorSteps = new List<EditorStep>();
            JsonNode? parsed = JsonNode.Parse(scenarioText);
            if (parsed is not JsonObject map || map["steps"] is not JsonArray steps)
            {
                return scenarioText;
            }

            JsonArray playSteps = new JsonArray();
            foreach (JsonNode? step in steps)
            {
                if (step is JsonObject stepObject && stepObject.Count == 1)
                {
                    KeyValuePair<string, JsonNode?> only = default;
                    foreach (KeyValuePair<string, JsonNode?> pair in stepObject)
                    {
                        only = pair;
                        break;
                    }

                    if (Array.IndexOf(EditorStepKinds, only.Key) >= 0)
                    {
                        editorSteps.Add(new EditorStep(
                            only.Key,
                            only.Value as JsonObject ?? new JsonObject()));
                        continue;
                    }
                }

                playSteps.Add(step?.DeepClone());
            }

            map["steps"] = playSteps;
            return map.ToJsonString();
        }

        private static JsonArray RunEditorSteps(
            McpHost host,
            string sessionId,
            List<EditorStep> editorSteps,
            string? projectRoot)
        {
            JsonArray results = new JsonArray();
            Dictionary<string, string> digests = new Dictionary<string, string>(StringComparer.Ordinal);
            Dictionary<string, string> namedDigests = new Dictionary<string, string>(StringComparer.Ordinal);
            Dictionary<string, byte[]> baselines = new Dictionary<string, byte[]>(StringComparer.Ordinal);
            foreach (EditorStep step in editorSteps)
            {
                JsonObject result = RunEditorStep(host, sessionId, step, projectRoot, digests, namedDigests, baselines);
                results.Add(result);
                if (result["ok"]?.GetValue<bool>() != true)
                {
                    string reason = result["error"]?.GetValue<string>() ?? "editor step failed";
                    RestoreBaselines(baselines);
                    throw new McpError.InvalidRequest(
                        "scenario editor step " + step.Kind + " failed: " + reason);
                }
            }

            return results;
        }

        // A failed authoring step must not leave the project scene edited, so the
        // bytes recorded before the first edit are written back.
        private static void RestoreBaselines(Dictionary<string, byte[]> baselines)
        {
            foreach (KeyValuePair<string, byte[]> baseline in baselines)
            {
                try
                {
                    File.WriteAllBytes(baseline.Key, baseline.Value);
                }
                catch (IOException)
                {
                }
                catch (UnauthorizedAccessException)
                {
                }
            }
        }

        private static JsonObject RunEditorStep(
            McpHost host,
            string sessionId,
            EditorStep step,
            string? projectRoot,
            Dictionary<string, string> digests,
            Dictionary<string, string> namedDigests,
            Dictionary<string, byte[]> baselines)
        {
            JsonObject result = new JsonObject
            {
                ["kind"] = step.Kind,
                ["ok"] = false,
            };
            try
            {
                switch (step.Kind)
                {
                    case EditorToolStep:
                        {
                            string tool = RequireStepString(step, "tool");
                            JsonObject arguments = step.Body["arguments"] as JsonObject ?? new JsonObject();
                            if (projectRoot != null && arguments["projectPath"] == null)
                            {
                                arguments["projectPath"] = projectRoot;
                            }

                            JsonNode payload = host.CallTool(sessionId, tool, arguments);
                            result["tool"] = tool;
                            result["payload"] = payload.DeepClone();
                            result["ok"] = true;
                            break;
                        }

                    case EditorFileDigestStep:
                        {
                            string path = ResolveStepFile(step, projectRoot);
                            string digest = FileDigest(path);
                            if (!digests.ContainsKey(path))
                            {
                                digests[path] = digest;
                                baselines[path] = File.ReadAllBytes(path);
                            }

                            // A named digest is a second baseline for the same file, so a
                            // scenario can prove that a file came back to a state it held
                            // between two of its steps.
                            string? digestId = OptionalStepString(step, "id");
                            if (digestId != null)
                            {
                                namedDigests[digestId] = digest;
                                result["id"] = digestId;
                            }

                            result["path"] = path;
                            result["sha256"] = digest;
                            result["ok"] = true;
                            break;
                        }

                    case EditorAssertFileUnchangedStep:
                    case EditorAssertFileChangedStep:
                        {
                            string path = ResolveStepFile(step, projectRoot);
                            string? digestId = OptionalStepString(step, "id");
                            string? recorded = null;
                            bool known;
                            if (digestId != null)
                            {
                                known = namedDigests.TryGetValue(digestId, out recorded);
                            }
                            else
                            {
                                known = digests.TryGetValue(path, out recorded);
                            }

                            string baseline = recorded ?? string.Empty;
                            if (!known)
                            {
                                throw new McpError.InvalidRequest(
                                    step.Kind + " has no recorded digest for "
                                    + (digestId ?? path));
                            }

                            string current = FileDigest(path);
                            bool changed = !string.Equals(current, baseline, StringComparison.Ordinal);
                            bool expectChanged = step.Kind == EditorAssertFileChangedStep;
                            result["path"] = path;
                            result["baseline"] = baseline;
                            result["current"] = current;
                            result["ok"] = changed == expectChanged;
                            if (result["ok"]?.GetValue<bool>() != true)
                            {
                                result["error"] = expectChanged
                                    ? "the file did not change: " + path
                                    : "the file changed but was expected to be restored: " + path
                                        + " (baseline " + baseline + ", current " + current + ")";
                            }

                            break;
                        }

                    default:
                        throw new McpError.InvalidRequest("unknown editor step kind: " + step.Kind);
                }
            }
            catch (McpError error)
            {
                result["error"] = error.Message;
            }
            catch (Exception error)
            {
                result["error"] = error.Message;
            }

            return result;
        }

        private static string RequireStepString(EditorStep step, string property)
        {
            if (step.Body[property] is JsonValue value && value.TryGetValue(out string? text))
            {
                return text;
            }

            throw new McpError.InvalidRequest(
                "editor step " + step.Kind + " requires a '" + property + "' string");
        }

        private static string? OptionalStepString(EditorStep step, string property)
        {
            if (step.Body[property] is JsonValue value
                && value.TryGetValue(out string? text)
                && !string.IsNullOrWhiteSpace(text))
            {
                return text;
            }

            return null;
        }

        private static string ResolveStepFile(EditorStep step, string? projectRoot)
        {
            string path = RequireStepString(step, "path");
            string candidate = Path.IsPathRooted(path)
                ? path
                : Path.Combine(projectRoot ?? Directory.GetCurrentDirectory(), path);
            if (!File.Exists(candidate))
            {
                throw new McpError.InvalidRequest("editor step file does not exist: " + candidate);
            }

            return Path.GetFullPath(candidate);
        }

        private static string FileDigest(string path)
        {
            byte[] bytes = System.Security.Cryptography.SHA256.HashData(File.ReadAllBytes(path));
            return System.BitConverter.ToString(bytes).Replace("-", string.Empty, StringComparison.Ordinal).ToLowerInvariant();
        }

        private static JsonObject BuildPayload(ScenarioResult result)
        {
            JsonObject payload = JsonNode.Parse(result.ToReportJson()) as JsonObject
                ?? new JsonObject();
            bool passed = result.Status == ScenarioStatus.Passed;
            payload["ok"] = passed;
            if (!passed)
            {
                payload["error"] = result.SetupError ?? FirstFailureReason(result) ?? "scenario failed";
            }

            return payload;
        }

        private static string? FirstFailureReason(ScenarioResult result)
        {
            foreach (CheckpointResult checkpoint in result.Checkpoints)
            {
                if (checkpoint.Status != CheckpointStatus.Passed)
                {
                    return checkpoint.Reason ?? ("checkpoint failed: " + checkpoint.Id);
                }
            }

            return null;
        }

        private static string ResolveEmbeddedPaths(string json, string scenarioDirectory)
        {
            JsonNode? node;
            try
            {
                node = JsonNode.Parse(json);
            }
            catch (JsonException)
            {
                return json;
            }

            if (node != null)
            {
                ResolveNode(node, scenarioDirectory);
                return node.ToJsonString();
            }

            return json;
        }

        private static void ResolveNode(JsonNode node, string scenarioDirectory)
        {
            if (node is JsonObject map)
            {
                List<KeyValuePair<string, JsonNode?>> pairs = new List<KeyValuePair<string, JsonNode?>>();
                foreach (KeyValuePair<string, JsonNode?> pair in map)
                {
                    pairs.Add(pair);
                }

                foreach (KeyValuePair<string, JsonNode?> pair in pairs)
                {
                    if (TryResolvePathValue(pair.Value, scenarioDirectory, out JsonValue? resolved))
                    {
                        map[pair.Key] = resolved;
                    }
                    else if (pair.Value != null)
                    {
                        ResolveNode(pair.Value, scenarioDirectory);
                    }
                }

                return;
            }

            if (node is JsonArray array)
            {
                for (int index = 0; index < array.Count; index++)
                {
                    if (TryResolvePathValue(array[index], scenarioDirectory, out JsonValue? resolved))
                    {
                        array[index] = resolved;
                    }
                    else if (array[index] != null)
                    {
                        ResolveNode(array[index]!, scenarioDirectory);
                    }
                }
            }
        }

        private static bool TryResolvePathValue(JsonNode? node, string scenarioDirectory, out JsonValue? resolved)
        {
            resolved = null;
            if (!(node is JsonValue value) || !value.TryGetValue(out string? text))
            {
                return false;
            }

            if (string.IsNullOrWhiteSpace(text)
                || (text.IndexOf('/', StringComparison.Ordinal) < 0 && text.IndexOf('\\', StringComparison.Ordinal) < 0)
                || Path.IsPathRooted(text))
            {
                return false;
            }

            string? fullPath = ResolveExistingPath(text, scenarioDirectory);
            if (fullPath == null)
            {
                return false;
            }

            resolved = JsonValue.Create(fullPath);
            return true;
        }

        private static string? ResolveExistingPath(string path, string root)
        {
            string candidate = Path.GetFullPath(Path.Combine(root, path));
            if (File.Exists(candidate))
            {
                return candidate;
            }

            DirectoryInfo? directory = new DirectoryInfo(root);
            for (int depth = 0; depth < PathSearchDepth && directory != null; depth++)
            {
                candidate = Path.GetFullPath(Path.Combine(directory.FullName, path));
                if (File.Exists(candidate))
                {
                    return candidate;
                }

                directory = directory.Parent;
            }

            return null;
        }

        private static void PurgeStaleSceneObjects()
        {
            UnityEngine.Object[] objects = UnityEngine.Object.FindObjectsOfType(typeof(GameObject), true);
            foreach (UnityEngine.Object item in objects)
            {
                if (item is GameObject stale)
                {
                    UnityEngine.Object.Destroy(stale);
                }
            }
        }

        private static string RequireCallerSession(McpHost host)
        {
            string? sessionId = host.CurrentSessionId;
            if (string.IsNullOrEmpty(sessionId))
            {
                throw new McpError.InvalidRequest("tool invocation requires an active MCP session");
            }

            return sessionId!;
        }

        private static string RequireScenarioPath(JsonNode arguments)
        {
            string? scenarioPath = ExtractOptionalString(arguments, "scenarioPath");
            if (scenarioPath == null)
            {
                throw new McpError.InvalidParams("scenario.run requires a 'scenarioPath' argument");
            }

            return scenarioPath;
        }

        private static string? ExtractOptionalString(JsonNode arguments, string propertyName)
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

        private static void RegisterProjectGlue(ScenarioDocument document, string? projectPath)
        {
            string root = string.IsNullOrWhiteSpace(projectPath)
                ? Directory.GetCurrentDirectory()
                : Path.GetFullPath(projectPath);
            string? projectRoot = ResolveProjectRoot(document, root);
            if (projectRoot == null)
            {
                throw new McpError.InvalidRequest(
                    "scenario.run could not resolve the project that owns the scenario scene; pass 'projectPath'");
            }

            string assemblyPath = Path.Combine(
                projectRoot,
                "Library",
                ScriptAssemblyFolderName,
                ScriptAssemblyFileName);
            if (!File.Exists(assemblyPath))
            {
                throw new McpError.InvalidRequest(
                    "project script assembly not found: " + assemblyPath + "; compile the project before running scenarios");
            }

            List<KeyValuePair<string, string>> scripts = ReadProjectScripts(projectRoot);
            if (scripts.Count == 0)
            {
                throw new McpError.InvalidRequest(
                    "project script metadata not found under " + Path.Combine(projectRoot, "Assets"));
            }

            var assembly = AssemblyLoadContext.Default.LoadFromAssemblyPath(assemblyPath);
            LoadGeneratedGlue(projectRoot);
            Type?[] types;
            try
            {
                types = assembly.GetTypes();
            }
            catch (Exception)
            {
                types = Type.EmptyTypes;
            }

            List<string> unresolved = new List<string>();
            int registered = 0;
            foreach (KeyValuePair<string, string> script in scripts)
            {
                string scriptName = Path.GetFileNameWithoutExtension(script.Value);
                Type? scriptType = FindScriptType(types, scriptName);
                if (scriptType == null)
                {
                    if (FindTypeByName(types, scriptName) == null && DeclaresComponent(script.Value))
                    {
                        unresolved.Add(scriptName);
                    }

                    continue;
                }

                if (!typeof(MonoBehaviour).IsAssignableFrom(scriptType))
                {
                    continue;
                }

                Type componentType = scriptType;
                if (ScriptGlueRegistry.IsRegistered(script.Key))
                {
                    registered++;
                    continue;
                }

                ScriptGlueRegistry.Register(
                    script.Key,
                    () => (MonoBehaviour)Activator.CreateInstance(componentType)!);
                registered++;
            }

            if (unresolved.Count > 0)
            {
                throw new McpError.InvalidRequest(
                    "the project script assembly does not contain the project's script types: "
                    + string.Join(", ", unresolved));
            }

            if (registered == 0)
            {
                throw new McpError.InvalidRequest(
                    "the project script assembly contains no script components: " + assemblyPath);
            }
        }

        private static void LoadGeneratedGlue(string projectRoot)
        {
            string gluePath = Path.Combine(
                projectRoot,
                "Library",
                ScriptAssemblyFolderName,
                ScriptCompilation.GeneratedGlueAssemblyName + ".dll");
            if (!File.Exists(gluePath))
            {
                return;
            }

            ScriptGlueRegistry.LoadGlueAssembly(gluePath);
        }

        private static string? ResolveProjectRoot(ScenarioDocument document, string root)
        {
            foreach (string scenePath in EnumerateScenePaths(document))
            {
                string? resolved = ResolveExistingPath(scenePath, root);
                if (resolved == null)
                {
                    continue;
                }

                DirectoryInfo? directory = new DirectoryInfo(Path.GetDirectoryName(resolved) ?? root);
                while (directory != null)
                {
                    if (Directory.Exists(Path.Combine(directory.FullName, "Assets"))
                        && Directory.Exists(Path.Combine(directory.FullName, "ProjectSettings")))
                    {
                        return directory.FullName;
                    }

                    directory = directory.Parent;
                }
            }

            return null;
        }

        private static IEnumerable<string> EnumerateScenePaths(ScenarioDocument document)
        {
            if (!string.IsNullOrWhiteSpace(document.Scene))
            {
                yield return document.Scene;
            }

            foreach (ScenarioStep step in document.Steps)
            {
                if (step is LoadSceneStep loadSceneStep && !string.IsNullOrWhiteSpace(loadSceneStep.Path))
                {
                    yield return loadSceneStep.Path;
                }
            }
        }

        private static List<KeyValuePair<string, string>> ReadProjectScripts(string projectRoot)
        {
            List<KeyValuePair<string, string>> scripts = new List<KeyValuePair<string, string>>();
            string assets = Path.Combine(projectRoot, "Assets");
            if (!Directory.Exists(assets))
            {
                return scripts;
            }

            string[] metaPaths = Directory.GetFiles(assets, "*.cs.meta", SearchOption.AllDirectories);
            Array.Sort(metaPaths, StringComparer.Ordinal);
            foreach (string metaPath in metaPaths)
            {
                string? guid = ReadMetaGuid(metaPath);
                if (guid == null)
                {
                    continue;
                }

                scripts.Add(new KeyValuePair<string, string>(
                    guid,
                    metaPath.Substring(0, metaPath.Length - ".meta".Length)));
            }

            return scripts;
        }

        private static string? ReadMetaGuid(string metaPath)
        {
            string[] lines;
            try
            {
                lines = File.ReadAllLines(metaPath);
            }
            catch (IOException)
            {
                return null;
            }

            foreach (string line in lines)
            {
                string trimmed = line.Trim();
                if (trimmed.StartsWith("guid:", StringComparison.Ordinal))
                {
                    string guid = trimmed.Substring(5).Trim();
                    return guid.Length == 32 ? guid : null;
                }
            }

            return null;
        }

        private static Type? FindScriptType(Type?[] types, string scriptName)
        {
            foreach (Type? type in types)
            {
                if (type != null
                    && type.Name == scriptName
                    && !type.IsAbstract
                    && typeof(MonoBehaviour).IsAssignableFrom(type))
                {
                    return type;
                }
            }

            return null;
        }

        private static Type? FindTypeByName(Type?[] types, string typeName)
        {
            foreach (Type? type in types)
            {
                if (type != null && type.Name == typeName)
                {
                    return type;
                }
            }

            return null;
        }

        private static bool DeclaresComponent(string scriptPath)
        {
            try
            {
                return File.ReadAllText(scriptPath).Contains("MonoBehaviour", StringComparison.Ordinal);
            }
            catch (IOException)
            {
                return false;
            }
        }

    }
}
