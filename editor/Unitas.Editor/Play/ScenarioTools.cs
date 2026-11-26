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
                ScenarioDocument document = LoadDocument(scenarioPath, projectPath);
                RegisterProjectGlue(document, projectPath);
                PurgeStaleSceneObjects();
                ScenarioResult result = new ScenarioRunner().Run(document);
                JsonObject payload = BuildPayload(result);
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

        private static ScenarioDocument LoadDocument(string scenarioPath, string? projectPath)
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
            string json = ResolveEmbeddedPaths(text, scenarioDirectory);
            try
            {
                return ScenarioDocument.Parse(json);
            }
            catch (ScenarioFormatException error)
            {
                throw new McpError.InvalidRequest("scenario document is invalid: " + error.Message);
            }
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
