using System;
using System.IO;
using System.Text.Json.Nodes;
using Unitas.Scripting;

namespace Unitas.Editor.Mcp
{
    public static class ScriptTools
    {
        public const string CompileOrigin = "unitas://scripts/compile";
        public const string DiagnoseOrigin = "unitas://scripts/diagnose";

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

            host.RegisterTool(
                new ToolDefinition(
                    "scripts.diagnose",
                    "Diagnose the project's scripts and return the structured diagnostics report",
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
                arguments => HandleDiagnose(host, arguments));
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

        private static ToolOutput HandleDiagnose(McpHost host, JsonNode arguments)
        {
            string root = ProjectAccess.ResolveRoot(host, arguments, "scripts.diagnose");
            DiagnoseReport report;
            try
            {
                report = ScriptCompilation.DiagnoseProject(
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
                throw new McpError.InvalidRequest("scripts.diagnose failed: " + error.Message);
            }

            JsonArray diagnostics = new JsonArray();
            foreach (DiagnoseEntry entry in report.Diagnostics)
            {
                diagnostics.Add(new JsonObject
                {
                    ["identifier"] = entry.Identifier,
                    ["severity"] = entry.Severity,
                    ["message"] = entry.Message,
                    ["assembly"] = entry.Assembly,
                    ["file"] = RelativeDiagnosticFile(root, entry.File),
                    ["line"] = entry.Line,
                    ["column"] = entry.Column,
                });
            }

            JsonArray activeAssemblies = new JsonArray();
            foreach (string assembly in report.ActiveAssemblyPaths)
            {
                activeAssemblies.Add(assembly);
            }

            JsonObject payload = new JsonObject
            {
                ["succeeded"] = report.Succeeded,
                ["runningCompilation"] = report.RunningCompilation,
                ["errorCount"] = report.ErrorCount,
                ["warningCount"] = report.WarningCount,
                ["outputDirectory"] = report.OutputDirectory,
                ["activeAssemblyPaths"] = activeAssemblies,
                ["diagnostics"] = diagnostics,
            };
            return ToolOutput.Untrusted(DiagnoseOrigin, payload);
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
