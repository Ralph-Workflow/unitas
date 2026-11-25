using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Text;

namespace Unitas.Scripting
{
    public sealed class CodeLoopTestCaseRecord
    {
        public CodeLoopTestCaseRecord(
            string fullName,
            string outcome,
            double durationSeconds,
            string? message)
        {
            FullName = fullName ?? string.Empty;
            Outcome = outcome ?? string.Empty;
            DurationSeconds = durationSeconds;
            Message = message;
        }

        public string FullName { get; }

        public string Outcome { get; }

        public double DurationSeconds { get; }

        public string? Message { get; }
    }

    public sealed class CodeLoopTestSummary
    {
        public CodeLoopTestSummary(
            bool succeeded,
            int selectedCount,
            int completedCount,
            int passedCount,
            int failedCount,
            int skippedCount,
            IReadOnlyList<CodeLoopTestCaseRecord> results)
        {
            Succeeded = succeeded;
            SelectedCount = selectedCount;
            CompletedCount = completedCount;
            PassedCount = passedCount;
            FailedCount = failedCount;
            SkippedCount = skippedCount;
            Results = results ?? Array.Empty<CodeLoopTestCaseRecord>();
        }

        public bool Succeeded { get; }

        public int SelectedCount { get; }

        public int CompletedCount { get; }

        public int PassedCount { get; }

        public int FailedCount { get; }

        public int SkippedCount { get; }

        public IReadOnlyList<CodeLoopTestCaseRecord> Results { get; }
    }

    public sealed class CodeLoopReport
    {
        public CodeLoopReport(
            string projectRoot,
            DiagnoseReport diagnose,
            SymbolQueryResult? symbols,
            CodeLoopTestSummary? tests)
        {
            ProjectRoot = projectRoot ?? string.Empty;
            Diagnose = diagnose ?? throw new ArgumentNullException(nameof(diagnose));
            Symbols = symbols;
            Tests = tests;
        }

        public string ProjectRoot { get; }

        public DiagnoseReport Diagnose { get; }

        public SymbolQueryResult? Symbols { get; }

        public CodeLoopTestSummary? Tests { get; }

        public bool CompileSucceeded => Diagnose.Succeeded && Diagnose.ErrorCount == 0;

        public bool Succeeded => CompileSucceeded && (Tests == null || Tests.Succeeded);

        public string RunningCompilation => Diagnose.RunningCompilation;

        public int ErrorCount => Diagnose.ErrorCount;

        public int WarningCount => Diagnose.WarningCount;

        public IReadOnlyList<DiagnoseEntry> Diagnostics => Diagnose.Diagnostics;

        public IReadOnlyList<string> ActiveAssemblyPaths => Diagnose.ActiveAssemblyPaths;

        public string ToJson()
        {
            StringBuilder sb = new StringBuilder();
            sb.AppendLine("{");
            sb.Append("  \"projectRoot\": \"").Append(DiagnoseReport.EscapeJson(ProjectRoot)).AppendLine("\",");
            sb.Append("  \"succeeded\": ").Append(Succeeded ? "true" : "false").AppendLine(",");
            sb.Append("  \"compileSucceeded\": ").Append(CompileSucceeded ? "true" : "false").AppendLine(",");
            sb.Append("  \"runningCompilation\": \"").Append(DiagnoseReport.EscapeJson(RunningCompilation)).AppendLine("\",");
            sb.Append("  \"errorCount\": ").Append(ErrorCount.ToString(CultureInfo.InvariantCulture)).AppendLine(",");
            sb.Append("  \"warningCount\": ").Append(WarningCount.ToString(CultureInfo.InvariantCulture)).AppendLine(",");
            sb.Append("  \"outputDirectory\": \"").Append(DiagnoseReport.EscapeJson(Diagnose.OutputDirectory)).AppendLine("\",");

            sb.Append("  \"activeAssemblyPaths\": [");
            if (ActiveAssemblyPaths.Count > 0)
            {
                sb.AppendLine();
                for (int i = 0; i < ActiveAssemblyPaths.Count; i++)
                {
                    sb.Append("    \"").Append(DiagnoseReport.EscapeJson(ActiveAssemblyPaths[i])).Append('"');
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
                    sb.Append("      \"identifier\": \"").Append(DiagnoseReport.EscapeJson(d.Identifier)).AppendLine("\",");
                    sb.Append("      \"severity\": \"").Append(DiagnoseReport.EscapeJson(d.Severity)).AppendLine("\",");
                    sb.Append("      \"message\": \"").Append(DiagnoseReport.EscapeJson(d.Message)).AppendLine("\",");
                    sb.Append("      \"assembly\": \"").Append(DiagnoseReport.EscapeJson(d.Assembly)).AppendLine("\",");
                    sb.Append("      \"file\": \"").Append(DiagnoseReport.EscapeJson(d.File)).AppendLine("\",");
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

            sb.AppendLine("],");

            int symbolTypeCount = Symbols != null ? Symbols.Types.Count : 0;
            sb.Append("  \"symbolCount\": ").Append(symbolTypeCount.ToString(CultureInfo.InvariantCulture)).AppendLine(",");
            sb.Append("  \"symbols\": [");
            if (Symbols != null && Symbols.Types.Count > 0)
            {
                sb.AppendLine();
                for (int i = 0; i < Symbols.Types.Count; i++)
                {
                    SymbolTypeRecord t = Symbols.Types[i];
                    sb.AppendLine("    {");
                    sb.Append("      \"name\": \"").Append(DiagnoseReport.EscapeJson(t.Name)).AppendLine("\",");
                    sb.Append("      \"fullName\": \"").Append(DiagnoseReport.EscapeJson(t.FullName)).AppendLine("\",");
                    sb.Append("      \"assembly\": \"").Append(DiagnoseReport.EscapeJson(t.Assembly)).AppendLine("\",");
                    sb.Append("      \"file\": \"").Append(DiagnoseReport.EscapeJson(t.FilePath)).AppendLine("\",");
                    sb.Append("      \"line\": ").Append(t.Line.ToString(CultureInfo.InvariantCulture)).AppendLine(",");
                    sb.Append("      \"baseType\": \"").Append(DiagnoseReport.EscapeJson(t.BaseType)).AppendLine("\",");
                    sb.Append("      \"isMonoBehaviour\": ").Append(t.IsMonoBehaviour ? "true" : "false").AppendLine(",");
                    sb.Append("      \"isScriptableObject\": ").Append(t.IsScriptableObject ? "true" : "false").AppendLine(",");
                    sb.Append("      \"serializedFieldCount\": ").Append(t.SerializedFields.Count.ToString(CultureInfo.InvariantCulture)).AppendLine();
                    sb.Append("    }");
                    if (i + 1 < Symbols.Types.Count)
                    {
                        sb.Append(',');
                    }

                    sb.AppendLine();
                }

                sb.Append("  ");
            }

            sb.AppendLine("],");

            if (Tests == null)
            {
                sb.AppendLine("  \"tests\": null");
            }
            else
            {
                sb.AppendLine("  \"tests\": {");
                sb.Append("    \"succeeded\": ").Append(Tests.Succeeded ? "true" : "false").AppendLine(",");
                sb.Append("    \"selectedCount\": ").Append(Tests.SelectedCount.ToString(CultureInfo.InvariantCulture)).AppendLine(",");
                sb.Append("    \"completedCount\": ").Append(Tests.CompletedCount.ToString(CultureInfo.InvariantCulture)).AppendLine(",");
                sb.Append("    \"passedCount\": ").Append(Tests.PassedCount.ToString(CultureInfo.InvariantCulture)).AppendLine(",");
                sb.Append("    \"failedCount\": ").Append(Tests.FailedCount.ToString(CultureInfo.InvariantCulture)).AppendLine(",");
                sb.Append("    \"skippedCount\": ").Append(Tests.SkippedCount.ToString(CultureInfo.InvariantCulture)).AppendLine(",");
                sb.Append("    \"results\": [");
                if (Tests.Results.Count > 0)
                {
                    sb.AppendLine();
                    for (int i = 0; i < Tests.Results.Count; i++)
                    {
                        CodeLoopTestCaseRecord tc = Tests.Results[i];
                        sb.AppendLine("      {");
                        sb.Append("        \"fullName\": \"").Append(DiagnoseReport.EscapeJson(tc.FullName)).AppendLine("\",");
                        sb.Append("        \"outcome\": \"").Append(DiagnoseReport.EscapeJson(tc.Outcome)).AppendLine("\",");
                        sb.Append("        \"durationSeconds\": ").Append(tc.DurationSeconds.ToString("F6", CultureInfo.InvariantCulture)).AppendLine(",");
                        if (tc.Message != null)
                        {
                            sb.Append("        \"message\": \"").Append(DiagnoseReport.EscapeJson(tc.Message)).AppendLine("\"");
                        }
                        else
                        {
                            sb.AppendLine("        \"message\": null");
                        }

                        sb.Append("      }");
                        if (i + 1 < Tests.Results.Count)
                        {
                            sb.Append(',');
                        }

                        sb.AppendLine();
                    }

                    sb.Append("    ");
                }

                sb.AppendLine("]");
                sb.AppendLine("  }");
            }

            sb.Append('}');
            return sb.ToString();
        }
    }

    public static class CodeLoop
    {
        public static Func<string, CodeLoopTestSummary?>? DefaultTestRunner { get; set; }

        public static CodeLoopReport Run(
            string projectRoot,
            ScriptCompilationOptions? options = null,
            string? typeFilter = null,
            Func<string, CodeLoopTestSummary?>? testRunner = null)
        {
            if (string.IsNullOrEmpty(projectRoot))
            {
                throw new ArgumentException("Project root must be non-empty.", nameof(projectRoot));
            }

            string fullProjectRoot = Path.GetFullPath(projectRoot);
            ScriptCompilationOptions effectiveOptions = options ?? new ScriptCompilationOptions();
            DiagnoseReport diagnose = ScriptCompilation.DiagnoseProject(fullProjectRoot, effectiveOptions);

            SymbolQueryResult? symbols = null;
            CodeLoopTestSummary? tests = null;

            if (diagnose.Succeeded && diagnose.ErrorCount == 0)
            {
                symbols = SymbolQuery.QueryProject(fullProjectRoot, typeFilter, effectiveOptions);
                Func<string, CodeLoopTestSummary?>? runner = testRunner ?? DefaultTestRunner;
                if (runner != null)
                {
                    tests = runner(fullProjectRoot);
                }
            }

            return new CodeLoopReport(fullProjectRoot, diagnose, symbols, tests);
        }
    }
}
