using System;
using System.Globalization;
using System.IO;
using System.Runtime.Loader;
using System.Text.Json.Nodes;
using Unitas.Bridge.Scripting;
using Unitas.Scripting;

namespace Unitas.Editor.Mcp
{
    public static class ExecuteMethodTools
    {
        public const string ExecuteMethodOrigin = "unitas://editor/execute_method";

        // The main project script assembly; mirrors the loading path
        // ScenarioTools uses for scenario glue (Library/UnitasScriptAssemblies).
        private const string ScriptAssemblyFileName = "Assembly-CSharp.dll";

        public static void Register(McpHost host)
        {
            ArgumentNullException.ThrowIfNull(host);

            host.RegisterTool(
                new ToolDefinition(
                    "editor.execute_method",
                    "Invoke a parameterless static method marked [EditorCallableMethod] in the open project's compiled script assembly and return its result; name the method by its fully qualified name, such as 'Namespace.Type.MethodName'",
                    new JsonObject
                    {
                        ["type"] = "object",
                        ["properties"] = new JsonObject
                        {
                            ["projectPath"] = new JsonObject
                            {
                                ["type"] = "string",
                            },
                            ["method"] = new JsonObject
                            {
                                ["type"] = "string",
                            },
                        },
                        ["required"] = new JsonArray("method"),
                    },
                    Scope.ExecuteProjectCode),
                arguments => HandleExecuteMethod(host, arguments));
        }

        private static ToolOutput HandleExecuteMethod(McpHost host, JsonNode arguments)
        {
            string root = ProjectAccess.ResolveRoot(host, arguments, "editor.execute_method");
            string methodReference = ProjectAccess.OptionalString(arguments, "method")
                ?? throw new McpError.InvalidParams(
                    "editor.execute_method requires a non-empty 'method' argument naming Type.Method");

            EnsureScriptAssemblyLoaded(root);

            bool found;
            object? result;
            try
            {
                found = EditorMethodRegistry.TryInvoke(methodReference, out result);
            }
            catch (Exception error)
            {
                throw new McpError.InvalidRequest("editor.execute_method failed: " + error.Message);
            }

            if (!found)
            {
                throw new McpError.InvalidRequest(
                    "editor.execute_method refuses '" + methodReference
                    + "': no editor-callable method is registered under that name; "
                    + "mark the method with [EditorCallableMethod] and recompile the project");
            }

            string? resultText = result == null
                ? null
                : Convert.ToString(result, CultureInfo.InvariantCulture);
            JsonObject payload = new JsonObject
            {
                ["ok"] = true,
                ["method"] = methodReference,
                ["declaringType"] = DeclaringTypeOf(methodReference),
                ["result"] = resultText == null ? null : JsonValue.Create(resultText),
            };
            return ToolOutput.Untrusted(ExecuteMethodOrigin, payload);
        }

        // The project must be compiled before a method can run; loading the
        // script assembly keeps the refusal honest about the current build.
        private static void EnsureScriptAssemblyLoaded(string root)
        {
            string assemblyPath = Path.Combine(
                root,
                "Library",
                ScriptCompilation.DefaultOutputFolderName,
                ScriptAssemblyFileName);
            if (!File.Exists(assemblyPath))
            {
                throw new McpError.InvalidRequest(
                    "project script assembly not found: " + assemblyPath
                    + "; compile the project before executing methods");
            }

            AssemblyLoadContext.Default.LoadFromAssemblyPath(assemblyPath);
        }

        private static string DeclaringTypeOf(string methodReference)
        {
            int split = methodReference.LastIndexOf('.');
            return split <= 0 ? string.Empty : methodReference.Substring(0, split);
        }
    }
}
