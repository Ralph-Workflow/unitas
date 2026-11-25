using System;
using System.Collections.Generic;
using System.Globalization;
using System.Text;

namespace Unitas.Scripting
{
    public sealed class DiagnoseEntry
    {
        public DiagnoseEntry(
            string identifier,
            string severity,
            string message,
            string assembly,
            string file,
            int line,
            int column)
        {
            Identifier = identifier ?? string.Empty;
            Severity = severity ?? string.Empty;
            Message = message ?? string.Empty;
            Assembly = assembly ?? string.Empty;
            File = file ?? string.Empty;
            Line = line;
            Column = column;
        }

        public string Identifier { get; }

        public string Severity { get; }

        public string Message { get; }

        public string Assembly { get; }

        public string File { get; }

        public int Line { get; }

        public int Column { get; }
    }

    public sealed class DiagnoseReport
    {
        public DiagnoseReport(
            bool succeeded,
            string runningCompilation,
            string outputDirectory,
            IReadOnlyList<DiagnoseEntry> diagnostics,
            IReadOnlyList<string> activeAssemblyPaths,
            ScriptCompilationResult compilationResult)
        {
            Succeeded = succeeded;
            RunningCompilation = runningCompilation ?? "none";
            OutputDirectory = outputDirectory ?? string.Empty;
            Diagnostics = diagnostics;
            ActiveAssemblyPaths = activeAssemblyPaths;
            CompilationResult = compilationResult;
        }

        public bool Succeeded { get; }

        public string RunningCompilation { get; }

        public string OutputDirectory { get; }

        public IReadOnlyList<DiagnoseEntry> Diagnostics { get; }

        public IReadOnlyList<string> ActiveAssemblyPaths { get; }

        public ScriptCompilationResult CompilationResult { get; }

        public int ErrorCount
        {
            get
            {
                int count = 0;
                for (int i = 0; i < Diagnostics.Count; i++)
                {
                    if (string.Equals(Diagnostics[i].Severity, "error", StringComparison.OrdinalIgnoreCase))
                    {
                        count++;
                    }
                }

                return count;
            }
        }

        public int WarningCount
        {
            get
            {
                int count = 0;
                for (int i = 0; i < Diagnostics.Count; i++)
                {
                    if (string.Equals(Diagnostics[i].Severity, "warning", StringComparison.OrdinalIgnoreCase))
                    {
                        count++;
                    }
                }

                return count;
            }
        }

        public static DiagnoseReport FromCompilationResult(ScriptCompilationResult result)
        {
            if (result == null)
            {
                throw new ArgumentNullException(nameof(result));
            }

            string running = result.ActiveState switch
            {
                ActiveAssemblyState.Current => "new",
                ActiveAssemblyState.LastValidFallback => "lastValid",
                _ => "none"
            };

            List<DiagnoseEntry> entries = new List<DiagnoseEntry>(result.Diagnostics.Count);
            for (int i = 0; i < result.Diagnostics.Count; i++)
            {
                ScriptDiagnostic d = result.Diagnostics[i];
                string sev = d.Severity switch
                {
                    ScriptDiagnosticSeverity.Error => "error",
                    ScriptDiagnosticSeverity.Warning => "warning",
                    _ => "info"
                };

                entries.Add(new DiagnoseEntry(
                    d.Id,
                    sev,
                    d.Message,
                    d.Assembly,
                    d.File,
                    d.Line,
                    d.Column));
            }

            return new DiagnoseReport(
                result.Succeeded,
                running,
                result.OutputDirectory,
                entries,
                result.ActiveAssemblyPaths,
                result);
        }

        public string ToJson()
        {
            StringBuilder sb = new StringBuilder();
            sb.AppendLine("{");
            sb.Append("  \"succeeded\": ").Append(Succeeded ? "true" : "false").AppendLine(",");
            sb.Append("  \"runningCompilation\": \"").Append(EscapeJson(RunningCompilation)).AppendLine("\",");
            sb.Append("  \"errorCount\": ").Append(ErrorCount.ToString(CultureInfo.InvariantCulture)).AppendLine(",");
            sb.Append("  \"warningCount\": ").Append(WarningCount.ToString(CultureInfo.InvariantCulture)).AppendLine(",");
            sb.Append("  \"outputDirectory\": \"").Append(EscapeJson(OutputDirectory)).AppendLine("\",");

            sb.Append("  \"activeAssemblyPaths\": [");
            if (ActiveAssemblyPaths.Count > 0)
            {
                sb.AppendLine();
                for (int i = 0; i < ActiveAssemblyPaths.Count; i++)
                {
                    sb.Append("    \"").Append(EscapeJson(ActiveAssemblyPaths[i])).Append('"');
                    if (i + 1 < ActiveAssemblyPaths.Count)
                    {
                        sb.Append(',');
                    }

                    sb.AppendLine();
                }

                sb.Append("  ");
            }

            sb.AppendLine("],");

            sb.Append("  \"diagnostics\": [");
            if (Diagnostics.Count > 0)
            {
                sb.AppendLine();
                for (int i = 0; i < Diagnostics.Count; i++)
                {
                    DiagnoseEntry d = Diagnostics[i];
                    sb.AppendLine("    {");
                    sb.Append("      \"identifier\": \"").Append(EscapeJson(d.Identifier)).AppendLine("\",");
                    sb.Append("      \"severity\": \"").Append(EscapeJson(d.Severity)).AppendLine("\",");
                    sb.Append("      \"message\": \"").Append(EscapeJson(d.Message)).AppendLine("\",");
                    sb.Append("      \"assembly\": \"").Append(EscapeJson(d.Assembly)).AppendLine("\",");
                    sb.Append("      \"file\": \"").Append(EscapeJson(d.File)).AppendLine("\",");
                    sb.Append("      \"line\": ").Append(d.Line.ToString(CultureInfo.InvariantCulture)).AppendLine(",");
                    sb.Append("      \"column\": ").Append(d.Column.ToString(CultureInfo.InvariantCulture)).AppendLine();
                    sb.Append("    }");
                    if (i + 1 < Diagnostics.Count)
                    {
                        sb.Append(',');
                    }

                    sb.AppendLine();
                }

                sb.Append("  ");
            }

            sb.AppendLine("]");
            sb.Append('}');
            return sb.ToString();
        }

        internal static string EscapeJson(string value)
        {
            if (string.IsNullOrEmpty(value))
            {
                return string.Empty;
            }

            StringBuilder sb = new StringBuilder(value.Length);
            for (int i = 0; i < value.Length; i++)
            {
                char c = value[i];
                switch (c)
                {
                    case '\\':
                        sb.Append("\\\\");
                        break;
                    case '"':
                        sb.Append("\\\"");
                        break;
                    case '\n':
                        sb.Append("\\n");
                        break;
                    case '\r':
                        sb.Append("\\r");
                        break;
                    case '\t':
                        sb.Append("\\t");
                        break;
                    default:
                        if (c < ' ')
                        {
                            sb.Append("\\u").Append(((int)c).ToString("X4", CultureInfo.InvariantCulture));
                        }
                        else
                        {
                            sb.Append(c);
                        }

                        break;
                }
            }

            return sb.ToString();
        }
    }
}
