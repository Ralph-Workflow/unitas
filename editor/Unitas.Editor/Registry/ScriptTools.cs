using System;
using System.IO;
using System.Text.Json.Nodes;
using Unitas.Scripting;

namespace Unitas.Editor.Mcp
{
    public static class ScriptTools
    {
        public const string CompileOrigin = "unitas://scripts/compile";

        public static void Register(McpHost host)
        {
            ArgumentNullException.ThrowIfNull(host);

            host.RegisterTool(
                new ToolDefinition(
                    "scripts.compile",
                    "Compile the project's scripts and return the structured diagnostics",
                    new JsonObject
                    {
                        ["type"] = "object",
                        ["properties"] = new JsonObject
                        {
                            ["projectPath"] = new JsonObject
                            {
                                ["type"] = "string",
                            },
                        },
                    },
                    Scope.EditScripts),
                arguments => HandleCompile(host, arguments));
        }

        private static ToolOutput HandleCompile(McpHost host, JsonNode arguments)
        {
            string root = ProjectAccess.ResolveRoot(host, arguments, "scripts.compile");
            ScriptCompilationResult result;
            try
            {
                result = ScriptCompilation.CompileProject(
                    root,
                    new ScriptCompilationOptions
                    {
                        IsEditor = true,
                        EmitGeneratedGlue = false,
                    });
            }
            catch (Exception error) when (error is IOException
                || error is InvalidOperationException
                || error is ArgumentException)
            {
                throw new McpError.InvalidRequest("scripts.compile failed: " + error.Message);
            }

            JsonArray diagnostics = new JsonArray();
            int errorCount = 0;
            int warningCount = 0;
            foreach (ScriptDiagnostic diagnostic in result.Diagnostics)
            {
                if (diagnostic.Severity == ScriptDiagnosticSeverity.Error)
                {
                    errorCount++;
                }
                else if (diagnostic.Severity == ScriptDiagnosticSeverity.Warning)
                {
                    warningCount++;
                }

                diagnostics.Add(new JsonObject
                {
                    ["identifier"] = diagnostic.Id,
                    ["severity"] = SeverityLabel(diagnostic.Severity),
                    ["message"] = diagnostic.Message,
                    ["assembly"] = diagnostic.Assembly,
                    ["file"] = RelativeDiagnosticFile(root, diagnostic.File),
                    ["line"] = diagnostic.Line,
                    ["column"] = diagnostic.Column,
                });
            }

            JsonArray activeAssemblies = new JsonArray();
            foreach (string assembly in result.ActiveAssemblyPaths)
            {
                activeAssemblies.Add(assembly);
            }

            JsonObject payload = new JsonObject
            {
                ["ok"] = result.Succeeded,
                ["running"] = result.ActiveState == ActiveAssemblyState.Current ? "new" : "last-valid",
                ["errorCount"] = errorCount,
                ["warningCount"] = warningCount,
                ["outputDirectory"] = result.OutputDirectory,
                ["activeAssemblyPaths"] = activeAssemblies,
                ["diagnostics"] = diagnostics,
            };
            return ToolOutput.Untrusted(CompileOrigin, payload);
        }

        private static string SeverityLabel(ScriptDiagnosticSeverity severity)
        {
            switch (severity)
            {
                case ScriptDiagnosticSeverity.Error: return "error";
                case ScriptDiagnosticSeverity.Warning: return "warning";
                default: return "info";
            }
        }

        private static string RelativeDiagnosticFile(string root, string file)
        {
            if (string.IsNullOrEmpty(file) || !AuditStore.IsPathInsideDirectory(file, root))
            {
                return file;
            }

            return Path.GetRelativePath(root, file).Replace(Path.DirectorySeparatorChar, '/');
        }
    }
}
