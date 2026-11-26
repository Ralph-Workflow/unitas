using System;
using System.Globalization;
using System.IO;
using System.Reflection;
using System.Runtime.Loader;
using System.Text.Json.Nodes;
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
                    "Invoke a parameterless static method in the open project's compiled script assembly and return its result",
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

            MethodInfo method = ResolveMethod(root, methodReference);

            object? result;
            try
            {
                result = method.Invoke(null, null);
            }
            catch (TargetInvocationException error)
            {
                throw new McpError.InvalidRequest(
                    "editor.execute_method: method threw: "
                    + (error.InnerException?.Message ?? error.Message));
            }
            catch (Exception error) when (error is ArgumentException
                || error is MethodAccessException
                || error is InvalidOperationException)
            {
                throw new McpError.InvalidRequest("editor.execute_method failed: " + error.Message);
            }

            string? resultText = result == null
                ? null
                : Convert.ToString(result, CultureInfo.InvariantCulture);
            JsonObject payload = new JsonObject
            {
                ["ok"] = true,
                ["method"] = methodReference,
                ["declaringType"] = method.DeclaringType?.FullName,
                ["result"] = resultText == null ? null : JsonValue.Create(resultText),
            };
            return ToolOutput.Untrusted(ExecuteMethodOrigin, payload);
        }

        private static MethodInfo ResolveMethod(string root, string methodReference)
        {
            int split = methodReference.LastIndexOf('.');
            if (split <= 0 || split == methodReference.Length - 1)
            {
                throw new McpError.InvalidRequest(
                    "editor.execute_method: 'method' must name a type and method like 'TypeName.MethodName': "
                    + methodReference);
            }

            string typeName = methodReference.Substring(0, split);
            string methodName = methodReference.Substring(split + 1);

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

            var assembly = AssemblyLoadContext.Default.LoadFromAssemblyPath(assemblyPath);
            Type?[] types;
            try
            {
                types = assembly.GetTypes();
            }
            catch (Exception)
            {
                types = Type.EmptyTypes;
            }

            Type? type = null;
            foreach (Type? candidate in types)
            {
                if (candidate != null
                    && (string.Equals(candidate.FullName, typeName, StringComparison.Ordinal)
                        || string.Equals(candidate.Name, typeName, StringComparison.Ordinal)))
                {
                    type = candidate;
                    break;
                }
            }

            if (type == null)
            {
                throw new McpError.InvalidRequest(
                    "unknown method: no type named '" + typeName
                    + "' was found in the project script assembly");
            }

            MethodInfo? match = null;
            bool sawInstance = false;
            foreach (MethodInfo candidate in type.GetMethods(BindingFlags.Public | BindingFlags.Static | BindingFlags.Instance))
            {
                if (!string.Equals(candidate.Name, methodName, StringComparison.Ordinal))
                {
                    continue;
                }

                if (!candidate.IsStatic)
                {
                    sawInstance = true;
                    continue;
                }

                match = candidate;
                break;
            }

            if (match == null)
            {
                throw new McpError.InvalidRequest(
                    sawInstance
                        ? "editor.execute_method refuses '" + methodReference + "': the method is not static"
                        : "unknown method: no static method '" + methodName + "' was found on type '" + type.FullName + "'");
            }

            if (match.GetParameters().Length != 0)
            {
                throw new McpError.InvalidRequest(
                    "editor.execute_method refuses '" + methodReference
                    + "': expected a parameterless static method");
            }

            return match;
        }
    }
}
