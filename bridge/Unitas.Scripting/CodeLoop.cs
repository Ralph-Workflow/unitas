using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Security.Cryptography;
using System.Text;
using Unitas.Bridge;
using Unitas.Bridge.Scripting;
using UnityEngine;

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

    public sealed class ScriptAuthoringResult
    {
        public ScriptAuthoringResult(
            string projectRoot,
            string scriptPath,
            string relativeScriptPath,
            string metaPath,
            string guid)
        {
            ProjectRoot = projectRoot ?? string.Empty;
            ScriptPath = scriptPath ?? string.Empty;
            RelativeScriptPath = relativeScriptPath ?? string.Empty;
            MetaPath = metaPath ?? string.Empty;
            Guid = guid ?? string.Empty;
        }

        public string ProjectRoot { get; }

        public string ScriptPath { get; }

        public string RelativeScriptPath { get; }

        public string MetaPath { get; }

        public string Guid { get; }
    }

    public sealed class PrefabComponentAttachment
    {
        public PrefabComponentAttachment(
            string projectRoot,
            string prefabPath,
            string relativePrefabPath,
            string objectPath,
            long gameObjectFileId,
            long componentFileId,
            string scriptPath,
            string scriptGuid,
            IReadOnlyDictionary<string, string> serializedFields)
        {
            ProjectRoot = projectRoot ?? string.Empty;
            PrefabPath = prefabPath ?? string.Empty;
            RelativePrefabPath = relativePrefabPath ?? string.Empty;
            ObjectPath = objectPath ?? string.Empty;
            GameObjectFileId = gameObjectFileId;
            ComponentFileId = componentFileId;
            ScriptPath = scriptPath ?? string.Empty;
            ScriptGuid = scriptGuid ?? string.Empty;
            SerializedFields = serializedFields ?? new Dictionary<string, string>(StringComparer.Ordinal);
        }

        public string ProjectRoot { get; }

        public string PrefabPath { get; }

        public string RelativePrefabPath { get; }

        public string ObjectPath { get; }

        public long GameObjectFileId { get; }

        public long ComponentFileId { get; }

        public string ScriptPath { get; }

        public string ScriptGuid { get; }

        public IReadOnlyDictionary<string, string> SerializedFields { get; }

        public string ToJson()
        {
            StringBuilder sb = new StringBuilder();
            sb.AppendLine("{");
            sb.Append("  \"projectRoot\": \"").Append(DiagnoseReport.EscapeJson(ProjectRoot)).AppendLine("\",");
            sb.Append("  \"prefabPath\": \"").Append(DiagnoseReport.EscapeJson(PrefabPath)).AppendLine("\",");
            sb.Append("  \"relativePrefabPath\": \"").Append(DiagnoseReport.EscapeJson(RelativePrefabPath)).AppendLine("\",");
            sb.Append("  \"objectPath\": \"").Append(DiagnoseReport.EscapeJson(ObjectPath)).AppendLine("\",");
            sb.Append("  \"gameObjectFileId\": ").Append(GameObjectFileId.ToString(CultureInfo.InvariantCulture)).AppendLine(",");
            sb.Append("  \"componentFileId\": ").Append(ComponentFileId.ToString(CultureInfo.InvariantCulture)).AppendLine(",");
            sb.Append("  \"scriptPath\": \"").Append(DiagnoseReport.EscapeJson(ScriptPath)).AppendLine("\",");
            sb.Append("  \"scriptGuid\": \"").Append(DiagnoseReport.EscapeJson(ScriptGuid)).AppendLine("\",");
            AppendSerializedFieldsJson(sb, "  ", SerializedFields, false);
            sb.Append('}');
            return sb.ToString();
        }

        internal static void AppendSerializedFieldsJson(
            StringBuilder sb,
            string indent,
            IReadOnlyDictionary<string, string> fields,
            bool trailingComma)
        {
            sb.Append(indent).Append("\"serializedFields\": {");
            if (fields.Count > 0)
            {
                sb.AppendLine();
                List<string> keys = new List<string>(fields.Keys);
                keys.Sort(StringComparer.Ordinal);
                for (int i = 0; i < keys.Count; i++)
                {
                    string key = keys[i];
                    string val = fields[key];
                    sb.Append(indent)
                        .Append("  \"")
                        .Append(DiagnoseReport.EscapeJson(key))
                        .Append("\": \"")
                        .Append(DiagnoseReport.EscapeJson(val))
                        .Append('"');
                    if (i + 1 < keys.Count)
                    {
                        sb.Append(',');
                    }

                    sb.AppendLine();
                }

                sb.Append(indent);
            }

            sb.Append('}');
            if (trailingComma)
            {
                sb.Append(',');
            }

            sb.AppendLine();
        }
    }

    public sealed class ContentScriptUsageRecord
    {
        public ContentScriptUsageRecord(
            string assetPath,
            string relativeAssetPath,
            string assetKind,
            string objectPath,
            long gameObjectFileId,
            long componentFileId,
            string scriptGuid,
            string scriptTypeName,
            bool enabled,
            IReadOnlyDictionary<string, string> serializedFields)
        {
            AssetPath = assetPath ?? string.Empty;
            RelativeAssetPath = relativeAssetPath ?? string.Empty;
            AssetKind = assetKind ?? string.Empty;
            ObjectPath = objectPath ?? string.Empty;
            GameObjectFileId = gameObjectFileId;
            ComponentFileId = componentFileId;
            ScriptGuid = scriptGuid ?? string.Empty;
            ScriptTypeName = scriptTypeName ?? string.Empty;
            Enabled = enabled;
            SerializedFields = serializedFields ?? new Dictionary<string, string>(StringComparer.Ordinal);
        }

        public string AssetPath { get; }

        public string RelativeAssetPath { get; }

        public string AssetKind { get; }

        public string ObjectPath { get; }

        public long GameObjectFileId { get; }

        public long ComponentFileId { get; }

        public string ScriptGuid { get; }

        public string ScriptTypeName { get; }

        public bool Enabled { get; }

        public IReadOnlyDictionary<string, string> SerializedFields { get; }
    }

    public sealed class ContentUsageSearchResult
    {
        public ContentUsageSearchResult(
            string projectRoot,
            string query,
            string scriptGuid,
            string scriptTypeName,
            string scriptPath,
            IReadOnlyList<ContentScriptUsageRecord> usages)
        {
            ProjectRoot = projectRoot ?? string.Empty;
            Query = query ?? string.Empty;
            ScriptGuid = scriptGuid ?? string.Empty;
            ScriptTypeName = scriptTypeName ?? string.Empty;
            ScriptPath = scriptPath ?? string.Empty;
            Usages = usages ?? Array.Empty<ContentScriptUsageRecord>();
        }

        public string ProjectRoot { get; }

        public string Query { get; }

        public string ScriptGuid { get; }

        public string ScriptTypeName { get; }

        public string ScriptPath { get; }

        public IReadOnlyList<ContentScriptUsageRecord> Usages { get; }

        public string ToJson()
        {
            StringBuilder sb = new StringBuilder();
            AppendJsonBody(sb, string.Empty);
            return sb.ToString();
        }

        internal void AppendJsonBody(StringBuilder sb, string indent)
        {
            sb.Append(indent).AppendLine("{");
            sb.Append(indent).Append("  \"projectRoot\": \"").Append(DiagnoseReport.EscapeJson(ProjectRoot)).AppendLine("\",");
            sb.Append(indent).Append("  \"query\": \"").Append(DiagnoseReport.EscapeJson(Query)).AppendLine("\",");
            sb.Append(indent).Append("  \"scriptGuid\": \"").Append(DiagnoseReport.EscapeJson(ScriptGuid)).AppendLine("\",");
            sb.Append(indent).Append("  \"scriptTypeName\": \"").Append(DiagnoseReport.EscapeJson(ScriptTypeName)).AppendLine("\",");
            sb.Append(indent).Append("  \"scriptPath\": \"").Append(DiagnoseReport.EscapeJson(ScriptPath)).AppendLine("\",");
            sb.Append(indent).Append("  \"usageCount\": ").Append(Usages.Count.ToString(CultureInfo.InvariantCulture)).AppendLine(",");
            sb.Append(indent).Append("  \"usages\": [");
            if (Usages.Count > 0)
            {
                sb.AppendLine();
                for (int i = 0; i < Usages.Count; i++)
                {
                    ContentScriptUsageRecord u = Usages[i];
                    sb.Append(indent).AppendLine("    {");
                    sb.Append(indent).Append("      \"assetPath\": \"").Append(DiagnoseReport.EscapeJson(u.AssetPath)).AppendLine("\",");
                    sb.Append(indent).Append("      \"relativeAssetPath\": \"").Append(DiagnoseReport.EscapeJson(u.RelativeAssetPath)).AppendLine("\",");
                    sb.Append(indent).Append("      \"assetKind\": \"").Append(DiagnoseReport.EscapeJson(u.AssetKind)).AppendLine("\",");
                    sb.Append(indent).Append("      \"objectPath\": \"").Append(DiagnoseReport.EscapeJson(u.ObjectPath)).AppendLine("\",");
                    sb.Append(indent).Append("      \"gameObjectFileId\": ").Append(u.GameObjectFileId.ToString(CultureInfo.InvariantCulture)).AppendLine(",");
                    sb.Append(indent).Append("      \"componentFileId\": ").Append(u.ComponentFileId.ToString(CultureInfo.InvariantCulture)).AppendLine(",");
                    sb.Append(indent).Append("      \"scriptGuid\": \"").Append(DiagnoseReport.EscapeJson(u.ScriptGuid)).AppendLine("\",");
                    sb.Append(indent).Append("      \"scriptTypeName\": \"").Append(DiagnoseReport.EscapeJson(u.ScriptTypeName)).AppendLine("\",");
                    sb.Append(indent).Append("      \"enabled\": ").Append(u.Enabled ? "true" : "false").AppendLine(",");
                    PrefabComponentAttachment.AppendSerializedFieldsJson(sb, indent + "      ", u.SerializedFields, false);
                    sb.Append(indent).Append("    }");
                    if (i + 1 < Usages.Count)
                    {
                        sb.Append(',');
                    }

                    sb.AppendLine();
                }

                sb.Append(indent).Append("  ");
            }

            sb.AppendLine("]");
            sb.Append(indent).Append('}');
        }
    }

    public sealed class PlayModeBehaviourObservation
    {
        public PlayModeBehaviourObservation(
            ulong scriptHandle,
            string scriptTypeName,
            string fullTypeName,
            string gameObjectName,
            bool enabled,
            float positionX,
            float positionY,
            float positionZ)
        {
            ScriptHandle = scriptHandle;
            ScriptTypeName = scriptTypeName ?? string.Empty;
            FullTypeName = fullTypeName ?? string.Empty;
            GameObjectName = gameObjectName ?? string.Empty;
            Enabled = enabled;
            PositionX = positionX;
            PositionY = positionY;
            PositionZ = positionZ;
        }

        public ulong ScriptHandle { get; }

        public string ScriptTypeName { get; }

        public string FullTypeName { get; }

        public string GameObjectName { get; }

        public bool Enabled { get; }

        public float PositionX { get; }

        public float PositionY { get; }

        public float PositionZ { get; }
    }

    public sealed class PlayModeExecutionReport
    {
        public PlayModeExecutionReport(
            bool succeeded,
            string runningCompilation,
            string sceneOrPrefabPath,
            int ticksExecuted,
            double frameDeltaSeconds,
            int activeScriptCount,
            IReadOnlyList<PlayModeBehaviourObservation> executedBehaviours,
            string? errorMessage)
        {
            Succeeded = succeeded;
            RunningCompilation = runningCompilation ?? "none";
            SceneOrPrefabPath = sceneOrPrefabPath ?? string.Empty;
            TicksExecuted = ticksExecuted;
            FrameDeltaSeconds = frameDeltaSeconds;
            ActiveScriptCount = activeScriptCount;
            ExecutedBehaviours = executedBehaviours ?? Array.Empty<PlayModeBehaviourObservation>();
            ErrorMessage = errorMessage;
        }

        public bool Succeeded { get; }

        public string RunningCompilation { get; }

        public string SceneOrPrefabPath { get; }

        public int TicksExecuted { get; }

        public double FrameDeltaSeconds { get; }

        public int ActiveScriptCount { get; }

        public IReadOnlyList<PlayModeBehaviourObservation> ExecutedBehaviours { get; }

        public string? ErrorMessage { get; }

        public string ToJson()
        {
            StringBuilder sb = new StringBuilder();
            AppendJsonBody(sb, string.Empty);
            return sb.ToString();
        }

        internal void AppendJsonBody(StringBuilder sb, string indent)
        {
            sb.Append(indent).AppendLine("{");
            sb.Append(indent).Append("  \"succeeded\": ").Append(Succeeded ? "true" : "false").AppendLine(",");
            sb.Append(indent).Append("  \"runningCompilation\": \"").Append(DiagnoseReport.EscapeJson(RunningCompilation)).AppendLine("\",");
            sb.Append(indent).Append("  \"sceneOrPrefabPath\": \"").Append(DiagnoseReport.EscapeJson(SceneOrPrefabPath)).AppendLine("\",");
            sb.Append(indent).Append("  \"ticksExecuted\": ").Append(TicksExecuted.ToString(CultureInfo.InvariantCulture)).AppendLine(",");
            sb.Append(indent).Append("  \"frameDeltaSeconds\": ").Append(FrameDeltaSeconds.ToString("F4", CultureInfo.InvariantCulture)).AppendLine(",");
            sb.Append(indent).Append("  \"activeScriptCount\": ").Append(ActiveScriptCount.ToString(CultureInfo.InvariantCulture)).AppendLine(",");
            if (ErrorMessage != null)
            {
                sb.Append(indent).Append("  \"errorMessage\": \"").Append(DiagnoseReport.EscapeJson(ErrorMessage)).AppendLine("\",");
            }
            else
            {
                sb.Append(indent).AppendLine("  \"errorMessage\": null,");
            }

            sb.Append(indent).Append("  \"executedBehaviours\": [");
            if (ExecutedBehaviours.Count > 0)
            {
                sb.AppendLine();
                for (int i = 0; i < ExecutedBehaviours.Count; i++)
                {
                    PlayModeBehaviourObservation b = ExecutedBehaviours[i];
                    sb.Append(indent).AppendLine("    {");
                    sb.Append(indent).Append("      \"scriptHandle\": ").Append(b.ScriptHandle.ToString(CultureInfo.InvariantCulture)).AppendLine(",");
                    sb.Append(indent).Append("      \"scriptTypeName\": \"").Append(DiagnoseReport.EscapeJson(b.ScriptTypeName)).AppendLine("\",");
                    sb.Append(indent).Append("      \"fullTypeName\": \"").Append(DiagnoseReport.EscapeJson(b.FullTypeName)).AppendLine("\",");
                    sb.Append(indent).Append("      \"gameObjectName\": \"").Append(DiagnoseReport.EscapeJson(b.GameObjectName)).AppendLine("\",");
                    sb.Append(indent).Append("      \"enabled\": ").Append(b.Enabled ? "true" : "false").AppendLine(",");
                    sb.Append(indent).Append("      \"positionX\": ").Append(b.PositionX.ToString("F4", CultureInfo.InvariantCulture)).AppendLine(",");
                    sb.Append(indent).Append("      \"positionY\": ").Append(b.PositionY.ToString("F4", CultureInfo.InvariantCulture)).AppendLine(",");
                    sb.Append(indent).Append("      \"positionZ\": ").Append(b.PositionZ.ToString("F4", CultureInfo.InvariantCulture)).AppendLine();
                    sb.Append(indent).Append("    }");
                    if (i + 1 < ExecutedBehaviours.Count)
                    {
                        sb.Append(',');
                    }

                    sb.AppendLine();
                }

                sb.Append(indent).Append("  ");
            }

            sb.AppendLine("]");
            sb.Append(indent).Append('}');
        }
    }

    public sealed class CodeLoopReport
    {
        public CodeLoopReport(
            string projectRoot,
            DiagnoseReport diagnose,
            SymbolQueryResult? symbols,
            CodeLoopTestSummary? tests,
            ContentUsageSearchResult? usages = null,
            PlayModeExecutionReport? playMode = null)
        {
            ProjectRoot = projectRoot ?? string.Empty;
            Diagnose = diagnose ?? throw new ArgumentNullException(nameof(diagnose));
            Symbols = symbols;
            Tests = tests;
            Usages = usages;
            PlayMode = playMode;
        }

        public string ProjectRoot { get; }

        public DiagnoseReport Diagnose { get; }

        public SymbolQueryResult? Symbols { get; }

        public CodeLoopTestSummary? Tests { get; }

        public ContentUsageSearchResult? Usages { get; }

        public PlayModeExecutionReport? PlayMode { get; }

        public bool CompileSucceeded => Diagnose.Succeeded && Diagnose.ErrorCount == 0;

        public bool Succeeded =>
            CompileSucceeded
            && (Tests == null || Tests.Succeeded)
            && (PlayMode == null || PlayMode.Succeeded);

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
                sb.AppendLine("  \"tests\": null,");
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
                sb.AppendLine("  },");
            }

            if (Usages == null)
            {
                sb.AppendLine("  \"usages\": null,");
            }
            else
            {
                sb.AppendLine("  \"usages\":");
                Usages.AppendJsonBody(sb, "  ");
                sb.AppendLine(",");
            }

            if (PlayMode == null)
            {
                sb.AppendLine("  \"playMode\": null");
            }
            else
            {
                sb.AppendLine("  \"playMode\":");
                PlayMode.AppendJsonBody(sb, "  ");
                sb.AppendLine();
            }

            sb.Append('}');
            return sb.ToString();
        }
    }

    public static class CodeLoop
    {
        private static readonly UTF8Encoding Utf8NoBom = new UTF8Encoding(false);

        public static Func<string, CodeLoopTestSummary?>? DefaultTestRunner { get; set; }

        public static CodeLoopReport Run(
            string projectRoot,
            ScriptCompilationOptions? options = null,
            string? typeFilter = null,
            Func<string, CodeLoopTestSummary?>? testRunner = null,
            string? usageQuery = null,
            bool enterPlayMode = false,
            string? playSceneOrPrefabPath = null,
            int playTickCount = 2,
            double playFrameDeltaSeconds = 0.02)
        {
            if (string.IsNullOrEmpty(projectRoot))
            {
                throw new ArgumentException("Project root must be non-empty.", nameof(projectRoot));
            }

            string fullProjectRoot = Path.GetFullPath(projectRoot);
            ScriptCompilationOptions effectiveOptions = options ?? new ScriptCompilationOptions();
            if (enterPlayMode)
            {
                effectiveOptions.EmitGeneratedGlue = true;
                effectiveOptions.AutoLoadGeneratedGlue = true;
            }

            DiagnoseReport diagnose = ScriptCompilation.DiagnoseProject(fullProjectRoot, effectiveOptions);

            SymbolQueryResult? symbols = null;
            CodeLoopTestSummary? tests = null;
            ContentUsageSearchResult? usages = null;
            PlayModeExecutionReport? playMode = null;

            if (diagnose.Succeeded && diagnose.ErrorCount == 0)
            {
                symbols = SymbolQuery.QueryProject(fullProjectRoot, typeFilter, effectiveOptions);
                Func<string, CodeLoopTestSummary?>? runner = testRunner ?? DefaultTestRunner;
                if (runner != null)
                {
                    tests = runner(fullProjectRoot);
                }

                if (!string.IsNullOrWhiteSpace(usageQuery))
                {
                    usages = FindScriptUsages(fullProjectRoot, usageQuery!, effectiveOptions, diagnose.CompilationResult, symbols);
                }

                if (enterPlayMode || !string.IsNullOrWhiteSpace(playSceneOrPrefabPath))
                {
                    playMode = ExecutePlayModeOnCompilation(
                        fullProjectRoot,
                        diagnose,
                        playSceneOrPrefabPath,
                        playTickCount,
                        playFrameDeltaSeconds);
                }
            }

            return new CodeLoopReport(fullProjectRoot, diagnose, symbols, tests, usages, playMode);
        }

        public static ScriptAuthoringResult AddOrUpdateScript(
            string projectRoot,
            string relativeOrFullScriptPath,
            string sourceContent,
            string? explicitGuid = null)
        {
            if (string.IsNullOrEmpty(projectRoot))
            {
                throw new ArgumentException("Project root must be non-empty.", nameof(projectRoot));
            }

            if (string.IsNullOrEmpty(relativeOrFullScriptPath))
            {
                throw new ArgumentException("Script path must be non-empty.", nameof(relativeOrFullScriptPath));
            }

            string fullProjectRoot = Path.GetFullPath(projectRoot);
            string fullScriptPath = Path.IsPathRooted(relativeOrFullScriptPath)
                ? Path.GetFullPath(relativeOrFullScriptPath)
                : Path.GetFullPath(Path.Combine(fullProjectRoot, relativeOrFullScriptPath.Replace('/', Path.DirectorySeparatorChar)));

            string? dir = Path.GetDirectoryName(fullScriptPath);
            if (!string.IsNullOrEmpty(dir))
            {
                Directory.CreateDirectory(dir);
            }

            File.WriteAllText(fullScriptPath, sourceContent ?? string.Empty, Utf8NoBom);

            string metaPath = fullScriptPath + ".meta";
            string relativePath = ProjectMetadataIndex.GetRelativePathNormalized(fullProjectRoot, fullScriptPath);
            string guid = string.Empty;

            if (!string.IsNullOrWhiteSpace(explicitGuid))
            {
                guid = NormalizeOrComputeGuid(explicitGuid!, relativePath);
            }
            else if (File.Exists(metaPath))
            {
                guid = AssemblyDiscovery.TryReadGuidFromMetaFile(metaPath);
            }

            if (string.IsNullOrEmpty(guid))
            {
                guid = ComputeDeterministicGuid(relativePath);
            }

            if (!File.Exists(metaPath) || !string.IsNullOrWhiteSpace(explicitGuid))
            {
                string metaContent =
                    "fileFormatVersion: 2\n"
                    + "guid: " + guid + "\n"
                    + "MonoImporter:\n"
                    + "  serializedVersion: 2\n";
                File.WriteAllText(metaPath, metaContent, Utf8NoBom);
            }

            return new ScriptAuthoringResult(
                fullProjectRoot,
                fullScriptPath,
                relativePath,
                metaPath,
                guid);
        }

        public static PrefabComponentAttachment AttachComponentToPrefab(
            string projectRoot,
            string prefabRelativeOrFullPath,
            string scriptIdentifierOrPath,
            string? targetObjectName = null,
            IReadOnlyDictionary<string, string>? serializedFields = null)
        {
            if (string.IsNullOrEmpty(projectRoot))
            {
                throw new ArgumentException("Project root must be non-empty.", nameof(projectRoot));
            }

            if (string.IsNullOrEmpty(prefabRelativeOrFullPath))
            {
                throw new ArgumentException("Prefab path must be non-empty.", nameof(prefabRelativeOrFullPath));
            }

            if (string.IsNullOrEmpty(scriptIdentifierOrPath))
            {
                throw new ArgumentException("Script identifier or path must be non-empty.", nameof(scriptIdentifierOrPath));
            }

            string fullProjectRoot = Path.GetFullPath(projectRoot);
            string fullPrefabPath = Path.IsPathRooted(prefabRelativeOrFullPath)
                ? Path.GetFullPath(prefabRelativeOrFullPath)
                : Path.GetFullPath(Path.Combine(fullProjectRoot, prefabRelativeOrFullPath.Replace('/', Path.DirectorySeparatorChar)));

            if (!File.Exists(fullPrefabPath))
            {
                throw new FileNotFoundException("Prefab file was not found.", fullPrefabPath);
            }

            ResolveScriptIdentity(
                fullProjectRoot,
                scriptIdentifierOrPath,
                null,
                null,
                null,
                out string resolvedGuid,
                out string _,
                out string resolvedScriptPath);

            if (string.IsNullOrEmpty(resolvedGuid))
            {
                throw new InvalidOperationException("Unable to resolve script GUID for '" + scriptIdentifierOrPath + "'.");
            }

            string rawText = File.ReadAllText(fullPrefabPath, Encoding.UTF8);
            List<string> lines = SplitLinesPreserveOrder(rawText);

            ParsedYamlAsset parsed = ParseYamlAssetLines(lines);
            if (parsed.GameObjects.Count == 0)
            {
                throw new InvalidOperationException("Prefab '" + fullPrefabPath + "' contains no GameObject document.");
            }

            ParsedGameObjectDoc targetGo = SelectTargetGameObject(parsed, targetObjectName);
            long newComponentFileId = AllocateNextComponentFileId(parsed);

            InsertComponentReferenceIntoGameObject(lines, targetGo, newComponentFileId);

            Dictionary<string, string> effectiveFields = new Dictionary<string, string>(StringComparer.Ordinal);
            if (serializedFields != null)
            {
                foreach (KeyValuePair<string, string> kv in serializedFields)
                {
                    if (!string.IsNullOrWhiteSpace(kv.Key))
                    {
                        effectiveFields[kv.Key.Trim()] = kv.Value ?? string.Empty;
                    }
                }
            }

            AppendMonoBehaviourDocument(lines, newComponentFileId, targetGo.FileId, resolvedGuid, effectiveFields);

            string updatedYaml = string.Join("\n", lines) + "\n";
            File.WriteAllText(fullPrefabPath, updatedYaml, Utf8NoBom);

            string objectPath = BuildGameObjectHierarchyPath(targetGo.FileId, parsed);
            string relativePrefabPath = ProjectMetadataIndex.GetRelativePathNormalized(fullProjectRoot, fullPrefabPath);

            return new PrefabComponentAttachment(
                fullProjectRoot,
                fullPrefabPath,
                relativePrefabPath,
                objectPath,
                targetGo.FileId,
                newComponentFileId,
                resolvedScriptPath,
                resolvedGuid,
                effectiveFields);
        }

        public static ContentUsageSearchResult FindScriptUsages(
            string projectRoot,
            string scriptQuery,
            ScriptCompilationOptions? options = null)
        {
            return FindScriptUsages(projectRoot, scriptQuery, options, null, null);
        }

        internal static ContentUsageSearchResult FindScriptUsages(
            string projectRoot,
            string scriptQuery,
            ScriptCompilationOptions? options,
            ScriptCompilationResult? knownCompilation,
            SymbolQueryResult? knownSymbols)
        {
            if (string.IsNullOrEmpty(projectRoot))
            {
                throw new ArgumentException("Project root must be non-empty.", nameof(projectRoot));
            }

            if (string.IsNullOrWhiteSpace(scriptQuery))
            {
                throw new ArgumentException("Script query must be non-empty.", nameof(scriptQuery));
            }

            string fullProjectRoot = Path.GetFullPath(projectRoot);
            ResolveScriptIdentity(
                fullProjectRoot,
                scriptQuery,
                options,
                knownCompilation,
                knownSymbols,
                out string resolvedGuid,
                out string resolvedTypeName,
                out string resolvedScriptPath);

            List<ContentScriptUsageRecord> usages = new List<ContentScriptUsageRecord>();
            if (string.IsNullOrEmpty(resolvedGuid))
            {
                return new ContentUsageSearchResult(
                    fullProjectRoot,
                    scriptQuery,
                    string.Empty,
                    resolvedTypeName,
                    resolvedScriptPath,
                    usages);
            }

            List<string> contentFiles = new List<string>();
            string assetsDir = Path.Combine(fullProjectRoot, "Assets");
            if (Directory.Exists(assetsDir))
            {
                CollectContentAssetFiles(assetsDir, contentFiles);
            }

            contentFiles.Sort(StringComparer.Ordinal);

            Dictionary<string, string> prefabGuidToPath = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase);
            for (int i = 0; i < contentFiles.Count; i++)
            {
                string file = contentFiles[i];
                if (file.EndsWith(".prefab", StringComparison.OrdinalIgnoreCase))
                {
                    string metaFile = file + ".meta";
                    if (File.Exists(metaFile))
                    {
                        string pGuid = AssemblyDiscovery.TryReadGuidFromMetaFile(metaFile);
                        if (!string.IsNullOrEmpty(pGuid))
                        {
                            prefabGuidToPath[pGuid] = file;
                        }
                    }
                }
            }

            for (int i = 0; i < contentFiles.Count; i++)
            {
                string assetPath = contentFiles[i];
                ScanAssetForScriptUsages(
                    fullProjectRoot,
                    assetPath,
                    resolvedGuid,
                    resolvedTypeName,
                    prefabGuidToPath,
                    usages,
                    new HashSet<string>(StringComparer.OrdinalIgnoreCase));
            }

            return new ContentUsageSearchResult(
                fullProjectRoot,
                scriptQuery,
                resolvedGuid,
                resolvedTypeName,
                resolvedScriptPath,
                usages);
        }

        public static PlayModeExecutionReport EnterPlayMode(
            string projectRoot,
            string? sceneOrPrefabPath = null,
            int tickCount = 2,
            double frameDeltaSeconds = 0.02,
            ScriptCompilationOptions? options = null)
        {
            if (string.IsNullOrEmpty(projectRoot))
            {
                throw new ArgumentException("Project root must be non-empty.", nameof(projectRoot));
            }

            string fullProjectRoot = Path.GetFullPath(projectRoot);
            ScriptCompilationOptions effectiveOptions = options ?? new ScriptCompilationOptions();
            effectiveOptions.EmitGeneratedGlue = true;
            effectiveOptions.AutoLoadGeneratedGlue = true;

            DiagnoseReport diagnose = ScriptCompilation.DiagnoseProject(fullProjectRoot, effectiveOptions);
            return ExecutePlayModeOnCompilation(
                fullProjectRoot,
                diagnose,
                sceneOrPrefabPath,
                tickCount,
                frameDeltaSeconds);
        }

        private static PlayModeExecutionReport ExecutePlayModeOnCompilation(
            string fullProjectRoot,
            DiagnoseReport diagnose,
            string? sceneOrPrefabPath,
            int tickCount,
            double frameDeltaSeconds)
        {
            string runningComp = diagnose.RunningCompilation;
            string? gluePath = diagnose.CompilationResult.GeneratedGlueAssemblyPath;
            if (!string.IsNullOrEmpty(gluePath) && File.Exists(gluePath))
            {
                ScriptGlueRegistry.LoadGlueAssembly(gluePath);
            }

            string resolvedTargetPath = ResolvePlayTargetPath(fullProjectRoot, sceneOrPrefabPath);
            if (string.IsNullOrEmpty(resolvedTargetPath) || !File.Exists(resolvedTargetPath))
            {
                return new PlayModeExecutionReport(
                    false,
                    runningComp,
                    resolvedTargetPath,
                    0,
                    frameDeltaSeconds,
                    0,
                    Array.Empty<PlayModeBehaviourObservation>(),
                    "Play target scene or prefab was not found.");
            }

            int clampedTicks = Math.Max(1, tickCount);
            double clampedDelta = frameDeltaSeconds > 0.0 ? frameDeltaSeconds : 0.02;

            Dictionary<(ulong Hi, ulong Lo), ScriptGuidBinding> bindingByGuidPair =
                new Dictionary<(ulong Hi, ulong Lo), ScriptGuidBinding>();
            IReadOnlyList<ScriptGuidBinding> bindings = diagnose.CompilationResult.ScriptBindings;
            for (int i = 0; i < bindings.Count; i++)
            {
                ScriptGuidBinding b = bindings[i];
                bindingByGuidPair[(b.GuidHi, b.GuidLo)] = b;
            }

            string? tempWrapperScene = null;
            string sceneToLoad = resolvedTargetPath;

            try
            {
                if (resolvedTargetPath.EndsWith(".prefab", StringComparison.OrdinalIgnoreCase))
                {
                    sceneToLoad = CreateTempSceneForPrefabOrCopy(fullProjectRoot, resolvedTargetPath, out tempWrapperScene);
                }

                List<PlayModeBehaviourObservation> observations = new List<PlayModeBehaviourObservation>();
                int activeCount = 0;

                using (BridgeHost host = new BridgeHost())
                {
                    if (!string.IsNullOrEmpty(gluePath) && File.Exists(gluePath))
                    {
                        host.LoadGlueAssembly(gluePath);
                    }

                    host.LoadScene(sceneToLoad);
                    host.Tick(0.0);
                    for (int t = 0; t < clampedTicks; t++)
                    {
                        host.Tick(clampedDelta);
                    }

                    activeCount = host.ScriptCount;
                    ulong maxHandle = (ulong)Math.Max(activeCount + 16, 32);
                    for (ulong h = 1; h <= maxHandle; h++)
                    {
                        MonoBehaviour? script = host.GetScript(h);
                        if (script == null)
                        {
                            continue;
                        }

                        ResolveBehaviourNames(script, bindings, out string simpleTypeName, out string fullTypeName);
                        GameObject? go = script.gameObject;
                        string goName = go != null ? go.name : script.name;
                        Vector3 pos = go != null ? go.transform.position : Vector3.zero;
                        observations.Add(new PlayModeBehaviourObservation(
                            h,
                            simpleTypeName,
                            fullTypeName,
                            goName,
                            script.enabled,
                            pos.x,
                            pos.y,
                            pos.z));
                    }
                }

                return new PlayModeExecutionReport(
                    true,
                    runningComp,
                    resolvedTargetPath,
                    clampedTicks,
                    clampedDelta,
                    activeCount,
                    observations,
                    null);
            }
            catch (Exception ex) when (!(ex is OutOfMemoryException))
            {
                return new PlayModeExecutionReport(
                    false,
                    runningComp,
                    resolvedTargetPath,
                    0,
                    clampedDelta,
                    0,
                    Array.Empty<PlayModeBehaviourObservation>(),
                    ex.Message);
            }
            finally
            {
                if (!string.IsNullOrEmpty(tempWrapperScene) && File.Exists(tempWrapperScene))
                {
                    try
                    {
                        File.Delete(tempWrapperScene);
                    }
                    catch (IOException)
                    {
                    }
                }
            }
        }

        private static void ResolveBehaviourNames(
            MonoBehaviour script,
            IReadOnlyList<ScriptGuidBinding> bindings,
            out string simpleName,
            out string fullName)
        {
            if (ScriptGlueRegistry.TryGetRegisteredGuid(script, out ulong guidHi, out ulong guidLo))
            {
                for (int i = 0; i < bindings.Count; i++)
                {
                    ScriptGuidBinding b = bindings[i];
                    if (b.GuidHi == guidHi && b.GuidLo == guidLo)
                    {
                        simpleName = b.ClassName;
                        fullName = b.FullTypeName;
                        return;
                    }
                }
            }

            string raw = script.ToString() ?? nameof(MonoBehaviour);
            for (int i = 0; i < bindings.Count; i++)
            {
                ScriptGuidBinding b = bindings[i];
                if (string.Equals(raw, b.FullTypeName, StringComparison.Ordinal)
                    || string.Equals(raw, b.ClassName, StringComparison.Ordinal)
                    || raw.EndsWith("." + b.ClassName, StringComparison.Ordinal)
                    || raw.Contains("(" + b.FullTypeName + ")", StringComparison.Ordinal)
                    || raw.Contains("(" + b.ClassName + ")", StringComparison.Ordinal))
                {
                    simpleName = b.ClassName;
                    fullName = b.FullTypeName;
                    return;
                }
            }

            if (bindings.Count == 1)
            {
                simpleName = bindings[0].ClassName;
                fullName = bindings[0].FullTypeName;
                return;
            }

            int lastDot = raw.LastIndexOf('.');
            simpleName = lastDot >= 0 ? raw.Substring(lastDot + 1) : raw;
            fullName = raw;
        }

        private static string CreateTempSceneForPrefabOrCopy(
            string fullProjectRoot,
            string fullPrefabPath,
            out string? tempScenePath)
        {
            _ = fullProjectRoot;
            string? prefabDir = Path.GetDirectoryName(fullPrefabPath);
            string targetDir = !string.IsNullOrEmpty(prefabDir) && Directory.Exists(prefabDir)
                ? prefabDir
                : Path.GetTempPath();
            string tempFile = Path.Combine(
                targetDir,
                "__aut005_play_prefab_" + Guid.NewGuid().ToString("N", CultureInfo.InvariantCulture) + ".unity");
            tempScenePath = tempFile;
            File.Copy(fullPrefabPath, tempFile, true);
            return tempFile;
        }

        private static string ResolvePlayTargetPath(string fullProjectRoot, string? sceneOrPrefabPath)
        {
            if (!string.IsNullOrWhiteSpace(sceneOrPrefabPath))
            {
                string trimmed = sceneOrPrefabPath!.Trim();
                string candidate = Path.IsPathRooted(trimmed)
                    ? Path.GetFullPath(trimmed)
                    : Path.GetFullPath(Path.Combine(fullProjectRoot, trimmed.Replace('/', Path.DirectorySeparatorChar)));
                if (File.Exists(candidate))
                {
                    return candidate;
                }
            }

            string assetsDir = Path.Combine(fullProjectRoot, "Assets");
            if (!Directory.Exists(assetsDir))
            {
                return string.Empty;
            }

            List<string> scenes = new List<string>();
            List<string> prefabs = new List<string>();
            CollectFilesByExtensionSimple(assetsDir, ".unity", scenes);
            if (scenes.Count > 0)
            {
                scenes.Sort(StringComparer.Ordinal);
                return scenes[0];
            }

            CollectFilesByExtensionSimple(assetsDir, ".prefab", prefabs);
            if (prefabs.Count > 0)
            {
                prefabs.Sort(StringComparer.Ordinal);
                return prefabs[0];
            }

            return string.Empty;
        }

        private static void ResolveScriptIdentity(
            string fullProjectRoot,
            string scriptQuery,
            ScriptCompilationOptions? options,
            ScriptCompilationResult? knownCompilation,
            SymbolQueryResult? knownSymbols,
            out string resolvedGuid,
            out string resolvedTypeName,
            out string resolvedScriptPath)
        {
            resolvedGuid = string.Empty;
            resolvedTypeName = string.Empty;
            resolvedScriptPath = string.Empty;

            string trimmed = scriptQuery.Trim();
            Dictionary<string, string> guidToPath = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase);
            Dictionary<string, string> pathToGuid = new Dictionary<string, string>(StringComparer.Ordinal);
            List<ParsedPluginMeta> plugins = new List<ParsedPluginMeta>();
            ProjectMetadataIndex.ScanProjectMeta(fullProjectRoot, guidToPath, pathToGuid, plugins);

            if (ProjectMetadataIndex.TrySplitGuid(trimmed, out ulong _, out ulong _))
            {
                resolvedGuid = trimmed.ToLowerInvariant();
                if (guidToPath.TryGetValue(resolvedGuid, out string? foundPath))
                {
                    resolvedScriptPath = foundPath;
                }
            }

            if (string.IsNullOrEmpty(resolvedScriptPath))
            {
                string candidatePath = Path.IsPathRooted(trimmed)
                    ? Path.GetFullPath(trimmed)
                    : Path.GetFullPath(Path.Combine(fullProjectRoot, trimmed.Replace('/', Path.DirectorySeparatorChar)));

                if (File.Exists(candidatePath))
                {
                    resolvedScriptPath = candidatePath;
                }
                else if (!trimmed.EndsWith(".cs", StringComparison.OrdinalIgnoreCase))
                {
                    string withCs = candidatePath + ".cs";
                    if (File.Exists(withCs))
                    {
                        resolvedScriptPath = withCs;
                    }
                }
            }

            if (string.IsNullOrEmpty(resolvedScriptPath))
            {
                string simpleClassName = trimmed;
                int lastDot = simpleClassName.LastIndexOf('.');
                if (lastDot >= 0 && !simpleClassName.EndsWith(".cs", StringComparison.OrdinalIgnoreCase))
                {
                    simpleClassName = simpleClassName.Substring(lastDot + 1);
                }
                else if (simpleClassName.EndsWith(".cs", StringComparison.OrdinalIgnoreCase))
                {
                    simpleClassName = Path.GetFileNameWithoutExtension(simpleClassName);
                }

                string assetsDir = Path.Combine(fullProjectRoot, "Assets");
                if (Directory.Exists(assetsDir))
                {
                    List<string> csFiles = new List<string>();
                    CollectFilesByExtensionSimple(assetsDir, ".cs", csFiles);
                    csFiles.Sort(StringComparer.Ordinal);
                    for (int i = 0; i < csFiles.Count; i++)
                    {
                        string stem = Path.GetFileNameWithoutExtension(csFiles[i]);
                        if (string.Equals(stem, simpleClassName, StringComparison.Ordinal)
                            || string.Equals(stem, simpleClassName, StringComparison.OrdinalIgnoreCase))
                        {
                            resolvedScriptPath = Path.GetFullPath(csFiles[i]);
                            break;
                        }
                    }
                }
            }

            if (!string.IsNullOrEmpty(resolvedScriptPath) && string.IsNullOrEmpty(resolvedGuid))
            {
                if (!pathToGuid.TryGetValue(resolvedScriptPath, out string? mappedGuid) || string.IsNullOrEmpty(mappedGuid))
                {
                    string metaPath = resolvedScriptPath + ".meta";
                    if (File.Exists(metaPath))
                    {
                        mappedGuid = AssemblyDiscovery.TryReadGuidFromMetaFile(metaPath);
                    }
                    else
                    {
                        string rel = ProjectMetadataIndex.GetRelativePathNormalized(fullProjectRoot, resolvedScriptPath);
                        mappedGuid = ComputeDeterministicGuid(rel);
                        File.WriteAllText(
                            metaPath,
                            "fileFormatVersion: 2\nguid: " + mappedGuid + "\nMonoImporter:\n  serializedVersion: 2\n",
                            Utf8NoBom);
                    }
                }

                resolvedGuid = mappedGuid ?? string.Empty;
            }

            if (knownCompilation != null)
            {
                for (int i = 0; i < knownCompilation.ScriptBindings.Count; i++)
                {
                    ScriptGuidBinding b = knownCompilation.ScriptBindings[i];
                    if ((!string.IsNullOrEmpty(resolvedGuid) && string.Equals(b.Guid, resolvedGuid, StringComparison.OrdinalIgnoreCase))
                        || string.Equals(b.ClassName, trimmed, StringComparison.Ordinal)
                        || string.Equals(b.FullTypeName, trimmed, StringComparison.Ordinal))
                    {
                        if (string.IsNullOrEmpty(resolvedGuid))
                        {
                            resolvedGuid = b.Guid;
                        }

                        if (string.IsNullOrEmpty(resolvedScriptPath))
                        {
                            resolvedScriptPath = b.ScriptPath;
                        }

                        resolvedTypeName = b.FullTypeName;
                        break;
                    }
                }
            }

            if (string.IsNullOrEmpty(resolvedTypeName))
            {
                SymbolQueryResult symbols = knownSymbols ?? SymbolQuery.QueryProject(fullProjectRoot, null, options);
                for (int i = 0; i < symbols.Types.Count; i++)
                {
                    SymbolTypeRecord t = symbols.Types[i];
                    if ((!string.IsNullOrEmpty(resolvedScriptPath)
                            && string.Equals(Path.GetFullPath(t.FilePath), Path.GetFullPath(resolvedScriptPath), StringComparison.Ordinal))
                        || string.Equals(t.Name, trimmed, StringComparison.Ordinal)
                        || string.Equals(t.FullName, trimmed, StringComparison.Ordinal))
                    {
                        resolvedTypeName = t.FullName;
                        if (string.IsNullOrEmpty(resolvedScriptPath) && !string.IsNullOrEmpty(t.FilePath))
                        {
                            resolvedScriptPath = Path.GetFullPath(t.FilePath);
                        }

                        break;
                    }
                }
            }

            if (!string.IsNullOrEmpty(resolvedScriptPath) && string.IsNullOrEmpty(resolvedGuid))
            {
                string metaPath = resolvedScriptPath + ".meta";
                if (File.Exists(metaPath))
                {
                    resolvedGuid = AssemblyDiscovery.TryReadGuidFromMetaFile(metaPath);
                }
            }

            if (string.IsNullOrEmpty(resolvedTypeName) && !string.IsNullOrEmpty(resolvedScriptPath))
            {
                resolvedTypeName = Path.GetFileNameWithoutExtension(resolvedScriptPath);
            }
        }

        private static void ScanAssetForScriptUsages(
            string fullProjectRoot,
            string assetPath,
            string targetScriptGuid,
            string targetScriptTypeName,
            Dictionary<string, string> prefabGuidToPath,
            List<ContentScriptUsageRecord> usages,
            HashSet<string> activePrefabChain)
        {
            if (!File.Exists(assetPath))
            {
                return;
            }

            string rawText;
            try
            {
                rawText = File.ReadAllText(assetPath, Encoding.UTF8);
            }
            catch (IOException)
            {
                return;
            }

            List<string> lines = SplitLinesPreserveOrder(rawText);
            ParsedYamlAsset parsed = ParseYamlAssetLines(lines);

            string ext = Path.GetExtension(assetPath).ToLowerInvariant();
            string assetKind = ext switch
            {
                ".unity" => "scene",
                ".prefab" => "prefab",
                _ => "asset"
            };
            string relativeAssetPath = ProjectMetadataIndex.GetRelativePathNormalized(fullProjectRoot, assetPath);

            for (int i = 0; i < parsed.MonoBehaviours.Count; i++)
            {
                ParsedMonoBehaviourDoc mb = parsed.MonoBehaviours[i];
                if (!string.Equals(mb.ScriptGuid, targetScriptGuid, StringComparison.OrdinalIgnoreCase))
                {
                    continue;
                }

                string objectPath = mb.GameObjectFileId != 0
                    ? BuildGameObjectHierarchyPath(mb.GameObjectFileId, parsed)
                    : Path.GetFileNameWithoutExtension(assetPath);

                usages.Add(new ContentScriptUsageRecord(
                    assetPath,
                    relativeAssetPath,
                    assetKind,
                    objectPath,
                    mb.GameObjectFileId,
                    mb.FileId,
                    targetScriptGuid,
                    targetScriptTypeName,
                    mb.Enabled,
                    mb.SerializedFields));
            }

            for (int i = 0; i < parsed.PrefabInstances.Count; i++)
            {
                ParsedPrefabInstanceDoc pi = parsed.PrefabInstances[i];
                if (string.IsNullOrEmpty(pi.SourcePrefabGuid)
                    || !prefabGuidToPath.TryGetValue(pi.SourcePrefabGuid, out string? sourcePrefabPath))
                {
                    continue;
                }

                if (!activePrefabChain.Add(sourcePrefabPath))
                {
                    continue;
                }

                List<ContentScriptUsageRecord> nested = new List<ContentScriptUsageRecord>();
                ScanAssetForScriptUsages(
                    fullProjectRoot,
                    sourcePrefabPath,
                    targetScriptGuid,
                    targetScriptTypeName,
                    prefabGuidToPath,
                    nested,
                    activePrefabChain);
                activePrefabChain.Remove(sourcePrefabPath);

                string parentPrefix = pi.TransformParentFileId != 0
                    ? BuildTransformHierarchyPath(pi.TransformParentFileId, parsed)
                    : string.Empty;

                for (int n = 0; n < nested.Count; n++)
                {
                    ContentScriptUsageRecord childUsage = nested[n];
                    string combinedObjectPath = string.IsNullOrEmpty(parentPrefix)
                        ? childUsage.ObjectPath
                        : (string.IsNullOrEmpty(childUsage.ObjectPath)
                            ? parentPrefix
                            : parentPrefix + "/" + childUsage.ObjectPath);

                    Dictionary<string, string> mergedFields =
                        new Dictionary<string, string>(childUsage.SerializedFields, StringComparer.Ordinal);
                    for (int m = 0; m < pi.Modifications.Count; m++)
                    {
                        (long targetId, string propPath, string propVal) = pi.Modifications[m];
                        if (targetId == childUsage.ComponentFileId && !IsBuiltInMonoBehaviourField(propPath))
                        {
                            mergedFields[propPath] = propVal;
                        }
                    }

                    usages.Add(new ContentScriptUsageRecord(
                        assetPath,
                        relativeAssetPath,
                        assetKind,
                        combinedObjectPath,
                        childUsage.GameObjectFileId,
                        childUsage.ComponentFileId,
                        targetScriptGuid,
                        targetScriptTypeName,
                        childUsage.Enabled,
                        mergedFields));
                }
            }
        }

        private static string NormalizeOrComputeGuid(string candidate, string fallbackKey)
        {
            string trimmed = candidate.Trim().Replace("-", string.Empty, StringComparison.Ordinal).ToLowerInvariant();
            if (trimmed.Length == 32 && ProjectMetadataIndex.TrySplitGuid(trimmed, out ulong _, out ulong _))
            {
                return trimmed;
            }

            return ComputeDeterministicGuid(fallbackKey);
        }

        private static string ComputeDeterministicGuid(string key)
        {
            byte[] bytes = Encoding.UTF8.GetBytes("unitas_script_guid:" + key);
            byte[] hash = SHA256.HashData(bytes);
            StringBuilder sb = new StringBuilder(32);
            for (int i = 0; i < 16; i++)
            {
                sb.Append(hash[i].ToString("x2", CultureInfo.InvariantCulture));
            }

            return sb.ToString();
        }

        private static List<string> SplitLinesPreserveOrder(string text)
        {
            List<string> lines = new List<string>();
            using StringReader reader = new StringReader(text);
            string? line;
            while ((line = reader.ReadLine()) != null)
            {
                lines.Add(line);
            }

            return lines;
        }

        private static void CollectContentAssetFiles(string dir, List<string> results)
        {
            string[] files;
            try
            {
                files = Directory.GetFiles(dir);
            }
            catch (IOException)
            {
                return;
            }

            for (int i = 0; i < files.Length; i++)
            {
                string f = files[i];
                if (f.EndsWith(".prefab", StringComparison.OrdinalIgnoreCase)
                    || f.EndsWith(".unity", StringComparison.OrdinalIgnoreCase)
                    || f.EndsWith(".asset", StringComparison.OrdinalIgnoreCase))
                {
                    results.Add(Path.GetFullPath(f));
                }
            }

            string[] subdirs;
            try
            {
                subdirs = Directory.GetDirectories(dir);
            }
            catch (IOException)
            {
                return;
            }

            Array.Sort(subdirs, StringComparer.Ordinal);
            for (int i = 0; i < subdirs.Length; i++)
            {
                CollectContentAssetFiles(subdirs[i], results);
            }
        }

        private static void CollectFilesByExtensionSimple(string dir, string ext, List<string> results)
        {
            string[] files;
            try
            {
                files = Directory.GetFiles(dir);
            }
            catch (IOException)
            {
                return;
            }

            for (int i = 0; i < files.Length; i++)
            {
                if (files[i].EndsWith(ext, StringComparison.OrdinalIgnoreCase))
                {
                    results.Add(Path.GetFullPath(files[i]));
                }
            }

            string[] subdirs;
            try
            {
                subdirs = Directory.GetDirectories(dir);
            }
            catch (IOException)
            {
                return;
            }

            Array.Sort(subdirs, StringComparer.Ordinal);
            for (int i = 0; i < subdirs.Length; i++)
            {
                CollectFilesByExtensionSimple(subdirs[i], ext, results);
            }
        }

        private sealed class ParsedGameObjectDoc
        {
            public long FileId { get; set; }

            public string Name { get; set; } = "GameObject";

            public int StartLineIndex { get; set; }

            public int EndLineIndex { get; set; }

            public int ComponentSequenceHeaderLine { get; set; } = -1;

            public int LastComponentEntryLine { get; set; } = -1;
        }

        private sealed class ParsedTransformDoc
        {
            public long FileId { get; set; }

            public long GameObjectFileId { get; set; }

            public long FatherTransformFileId { get; set; }
        }

        private sealed class ParsedMonoBehaviourDoc
        {
            public long FileId { get; set; }

            public long GameObjectFileId { get; set; }

            public string ScriptGuid { get; set; } = string.Empty;

            public bool Enabled { get; set; } = true;

            public Dictionary<string, string> SerializedFields { get; } =
                new Dictionary<string, string>(StringComparer.Ordinal);
        }

        private sealed class ParsedPrefabInstanceDoc
        {
            public long FileId { get; set; }

            public string SourcePrefabGuid { get; set; } = string.Empty;

            public long TransformParentFileId { get; set; }

            public List<(long TargetFileId, string PropertyPath, string Value)> Modifications { get; } =
                new List<(long TargetFileId, string PropertyPath, string Value)>();
        }

        private sealed class ParsedYamlAsset
        {
            public List<ParsedGameObjectDoc> GameObjects { get; } = new List<ParsedGameObjectDoc>();

            public Dictionary<long, ParsedGameObjectDoc> GameObjectById { get; } =
                new Dictionary<long, ParsedGameObjectDoc>();

            public Dictionary<long, ParsedTransformDoc> TransformById { get; } =
                new Dictionary<long, ParsedTransformDoc>();

            public Dictionary<long, ParsedTransformDoc> TransformByGameObjectId { get; } =
                new Dictionary<long, ParsedTransformDoc>();

            public List<ParsedMonoBehaviourDoc> MonoBehaviours { get; } = new List<ParsedMonoBehaviourDoc>();

            public List<ParsedPrefabInstanceDoc> PrefabInstances { get; } = new List<ParsedPrefabInstanceDoc>();

            public HashSet<long> AllFileIds { get; } = new HashSet<long>();
        }

        private static ParsedYamlAsset ParseYamlAssetLines(IReadOnlyList<string> lines)
        {
            ParsedYamlAsset asset = new ParsedYamlAsset();
            List<(int ClassId, long FileId, bool Stripped, int StartLine, int EndLine)> docs =
                new List<(int ClassId, long FileId, bool Stripped, int StartLine, int EndLine)>();

            int currentClass = 0;
            long currentId = 0;
            bool currentStripped = false;
            int currentStart = -1;

            for (int i = 0; i < lines.Count; i++)
            {
                string trimmed = lines[i].Trim();
                if (trimmed.StartsWith("--- !u!", StringComparison.Ordinal))
                {
                    if (currentStart >= 0)
                    {
                        docs.Add((currentClass, currentId, currentStripped, currentStart, i - 1));
                    }

                    TryParseYamlDocHeader(trimmed, out currentClass, out currentId, out currentStripped);
                    currentStart = i;
                    if (currentId != 0)
                    {
                        asset.AllFileIds.Add(currentId);
                    }
                }
            }

            if (currentStart >= 0)
            {
                docs.Add((currentClass, currentId, currentStripped, currentStart, lines.Count - 1));
            }

            for (int d = 0; d < docs.Count; d++)
            {
                (int classId, long fileId, bool stripped, int startLine, int endLine) = docs[d];
                if (stripped)
                {
                    continue;
                }

                if (classId == 1)
                {
                    ParsedGameObjectDoc go = ParseGameObjectDoc(lines, fileId, startLine, endLine);
                    asset.GameObjects.Add(go);
                    asset.GameObjectById[fileId] = go;
                }
                else if (classId == 4)
                {
                    ParsedTransformDoc tr = ParseTransformDoc(lines, fileId, startLine, endLine);
                    asset.TransformById[fileId] = tr;
                    if (tr.GameObjectFileId != 0)
                    {
                        asset.TransformByGameObjectId[tr.GameObjectFileId] = tr;
                    }
                }
                else if (classId == 114)
                {
                    ParsedMonoBehaviourDoc mb = ParseMonoBehaviourDoc(lines, fileId, startLine, endLine);
                    asset.MonoBehaviours.Add(mb);
                }
                else if (classId == 1001)
                {
                    ParsedPrefabInstanceDoc pi = ParsePrefabInstanceDoc(lines, fileId, startLine, endLine);
                    asset.PrefabInstances.Add(pi);
                }
            }

            return asset;
        }

        private static void TryParseYamlDocHeader(
            string headerLine,
            out int classId,
            out long fileId,
            out bool stripped)
        {
            classId = 0;
            fileId = 0;
            stripped = headerLine.EndsWith("stripped", StringComparison.Ordinal);

            int tagPos = headerLine.IndexOf("!u!", StringComparison.Ordinal);
            int ampPos = headerLine.IndexOf('&', StringComparison.Ordinal);
            if (tagPos < 0 || ampPos <= tagPos + 3)
            {
                return;
            }

            string classText = headerLine.Substring(tagPos + 3, ampPos - (tagPos + 3)).Trim();
            _ = int.TryParse(classText, NumberStyles.Integer, CultureInfo.InvariantCulture, out classId);

            string afterAmp = headerLine.Substring(ampPos + 1).Trim();
            int spaceIdx = afterAmp.IndexOf(' ');
            string fileIdText = spaceIdx >= 0 ? afterAmp.Substring(0, spaceIdx) : afterAmp;
            _ = long.TryParse(fileIdText, NumberStyles.Integer, CultureInfo.InvariantCulture, out fileId);
        }

        private static ParsedGameObjectDoc ParseGameObjectDoc(
            IReadOnlyList<string> lines,
            long fileId,
            int startLine,
            int endLine)
        {
            ParsedGameObjectDoc doc = new ParsedGameObjectDoc
            {
                FileId = fileId,
                StartLineIndex = startLine,
                EndLineIndex = endLine
            };

            bool inComponents = false;
            for (int i = startLine + 1; i <= endLine; i++)
            {
                string raw = lines[i];
                string trimmed = raw.Trim();
                if (string.IsNullOrEmpty(trimmed))
                {
                    continue;
                }

                if (trimmed.StartsWith("m_Component:", StringComparison.Ordinal))
                {
                    doc.ComponentSequenceHeaderLine = i;
                    inComponents = true;
                    continue;
                }

                if (inComponents)
                {
                    if (trimmed.StartsWith("- component:", StringComparison.Ordinal)
                        || trimmed.StartsWith("- ", StringComparison.Ordinal))
                    {
                        doc.LastComponentEntryLine = i;
                        continue;
                    }

                    inComponents = false;
                }

                if (trimmed.StartsWith("m_Name:", StringComparison.Ordinal))
                {
                    string nameVal = ExtractColonValue(trimmed);
                    if (!string.IsNullOrEmpty(nameVal))
                    {
                        doc.Name = nameVal;
                    }
                }
            }

            return doc;
        }

        private static ParsedTransformDoc ParseTransformDoc(
            IReadOnlyList<string> lines,
            long fileId,
            int startLine,
            int endLine)
        {
            ParsedTransformDoc doc = new ParsedTransformDoc
            {
                FileId = fileId
            };

            for (int i = startLine + 1; i <= endLine; i++)
            {
                string trimmed = lines[i].Trim();
                if (trimmed.StartsWith("m_GameObject:", StringComparison.Ordinal))
                {
                    doc.GameObjectFileId = ExtractFlowFileId(trimmed);
                }
                else if (trimmed.StartsWith("m_Father:", StringComparison.Ordinal))
                {
                    doc.FatherTransformFileId = ExtractFlowFileId(trimmed);
                }
            }

            return doc;
        }

        private static ParsedMonoBehaviourDoc ParseMonoBehaviourDoc(
            IReadOnlyList<string> lines,
            long fileId,
            int startLine,
            int endLine)
        {
            ParsedMonoBehaviourDoc doc = new ParsedMonoBehaviourDoc
            {
                FileId = fileId
            };

            for (int i = startLine + 1; i <= endLine; i++)
            {
                string raw = lines[i];
                string trimmed = raw.Trim();
                if (string.IsNullOrEmpty(trimmed) || string.Equals(trimmed, "MonoBehaviour:", StringComparison.Ordinal))
                {
                    continue;
                }

                int indent = raw.Length - raw.TrimStart(' ').Length;
                if (indent != 2)
                {
                    continue;
                }

                int colon = trimmed.IndexOf(':');
                if (colon <= 0)
                {
                    continue;
                }

                string key = trimmed.Substring(0, colon).Trim();
                string val = UnquoteYamlScalar(trimmed.Substring(colon + 1).Trim());

                if (string.Equals(key, "m_GameObject", StringComparison.Ordinal))
                {
                    doc.GameObjectFileId = ExtractFlowFileId(trimmed);
                }
                else if (string.Equals(key, "m_Enabled", StringComparison.Ordinal))
                {
                    doc.Enabled = !string.Equals(val, "0", StringComparison.Ordinal)
                        && !string.Equals(val, "false", StringComparison.OrdinalIgnoreCase);
                }
                else if (string.Equals(key, "m_Script", StringComparison.Ordinal))
                {
                    doc.ScriptGuid = ExtractFlowGuid(trimmed);
                }
                else if (!IsBuiltInMonoBehaviourField(key))
                {
                    doc.SerializedFields[key] = val;
                }
            }

            return doc;
        }

        private static ParsedPrefabInstanceDoc ParsePrefabInstanceDoc(
            IReadOnlyList<string> lines,
            long fileId,
            int startLine,
            int endLine)
        {
            ParsedPrefabInstanceDoc doc = new ParsedPrefabInstanceDoc
            {
                FileId = fileId
            };

            long pendingTargetId = 0;
            string? pendingPropertyPath = null;
            string? pendingValue = null;

            void FlushPendingModification()
            {
                if (pendingTargetId != 0 && !string.IsNullOrEmpty(pendingPropertyPath) && pendingValue != null)
                {
                    doc.Modifications.Add((pendingTargetId, pendingPropertyPath, pendingValue));
                }

                pendingTargetId = 0;
                pendingPropertyPath = null;
                pendingValue = null;
            }

            for (int i = startLine + 1; i <= endLine; i++)
            {
                string trimmed = lines[i].Trim();
                if (trimmed.StartsWith("m_SourcePrefab:", StringComparison.Ordinal))
                {
                    doc.SourcePrefabGuid = ExtractFlowGuid(trimmed);
                }
                else if (trimmed.StartsWith("m_TransformParent:", StringComparison.Ordinal))
                {
                    doc.TransformParentFileId = ExtractFlowFileId(trimmed);
                }
                else if (trimmed.StartsWith("- target:", StringComparison.Ordinal)
                    || trimmed.StartsWith("target:", StringComparison.Ordinal))
                {
                    FlushPendingModification();
                    pendingTargetId = ExtractFlowFileId(trimmed);
                }
                else if (trimmed.StartsWith("propertyPath:", StringComparison.Ordinal))
                {
                    pendingPropertyPath = ExtractColonValue(trimmed);
                }
                else if (trimmed.StartsWith("value:", StringComparison.Ordinal))
                {
                    pendingValue = ExtractColonValue(trimmed);
                }
            }

            FlushPendingModification();
            return doc;
        }

        private static bool IsBuiltInMonoBehaviourField(string key)
        {
            return string.Equals(key, "m_ObjectHideFlags", StringComparison.Ordinal)
                || string.Equals(key, "m_CorrespondingSourceObject", StringComparison.Ordinal)
                || string.Equals(key, "m_PrefabInstance", StringComparison.Ordinal)
                || string.Equals(key, "m_PrefabAsset", StringComparison.Ordinal)
                || string.Equals(key, "m_GameObject", StringComparison.Ordinal)
                || string.Equals(key, "m_Enabled", StringComparison.Ordinal)
                || string.Equals(key, "m_EditorHideFlags", StringComparison.Ordinal)
                || string.Equals(key, "m_Script", StringComparison.Ordinal)
                || string.Equals(key, "m_Name", StringComparison.Ordinal)
                || string.Equals(key, "m_EditorClassIdentifier", StringComparison.Ordinal)
                || string.Equals(key, "serializedVersion", StringComparison.Ordinal);
        }

        private static ParsedGameObjectDoc SelectTargetGameObject(
            ParsedYamlAsset parsed,
            string? targetObjectName)
        {
            if (!string.IsNullOrWhiteSpace(targetObjectName))
            {
                string desired = targetObjectName!.Trim();
                for (int i = 0; i < parsed.GameObjects.Count; i++)
                {
                    ParsedGameObjectDoc go = parsed.GameObjects[i];
                    string fullPath = BuildGameObjectHierarchyPath(go.FileId, parsed);
                    if (string.Equals(go.Name, desired, StringComparison.Ordinal)
                        || string.Equals(fullPath, desired, StringComparison.Ordinal))
                    {
                        return go;
                    }
                }
            }

            for (int i = 0; i < parsed.GameObjects.Count; i++)
            {
                ParsedGameObjectDoc go = parsed.GameObjects[i];
                if (parsed.TransformByGameObjectId.TryGetValue(go.FileId, out ParsedTransformDoc? tr)
                    && tr.FatherTransformFileId == 0)
                {
                    return go;
                }
            }

            return parsed.GameObjects[0];
        }

        private static long AllocateNextComponentFileId(ParsedYamlAsset parsed)
        {
            long candidate = 11400000;
            while (parsed.AllFileIds.Contains(candidate))
            {
                candidate++;
            }

            parsed.AllFileIds.Add(candidate);
            return candidate;
        }

        private static void InsertComponentReferenceIntoGameObject(
            List<string> lines,
            ParsedGameObjectDoc targetGo,
            long componentFileId)
        {
            string entryLine = "  - component: {fileID: " + componentFileId.ToString(CultureInfo.InvariantCulture) + "}";
            if (targetGo.LastComponentEntryLine >= 0)
            {
                lines.Insert(targetGo.LastComponentEntryLine + 1, entryLine);
                return;
            }

            if (targetGo.ComponentSequenceHeaderLine >= 0)
            {
                string headerLine = lines[targetGo.ComponentSequenceHeaderLine];
                if (headerLine.Trim().EndsWith("[]", StringComparison.Ordinal))
                {
                    lines[targetGo.ComponentSequenceHeaderLine] = "  m_Component:";
                }

                lines.Insert(targetGo.ComponentSequenceHeaderLine + 1, entryLine);
                return;
            }

            int insertAt = Math.Min(targetGo.StartLineIndex + 2, lines.Count);
            lines.Insert(insertAt, "  m_Component:");
            lines.Insert(insertAt + 1, entryLine);
        }

        private static void AppendMonoBehaviourDocument(
            List<string> lines,
            long componentFileId,
            long gameObjectFileId,
            string scriptGuid,
            Dictionary<string, string> serializedFields)
        {
            lines.Add("--- !u!114 &" + componentFileId.ToString(CultureInfo.InvariantCulture));
            lines.Add("MonoBehaviour:");
            lines.Add("  m_ObjectHideFlags: 0");
            lines.Add("  m_CorrespondingSourceObject: {fileID: 0}");
            lines.Add("  m_PrefabInstance: {fileID: 0}");
            lines.Add("  m_PrefabAsset: {fileID: 0}");
            lines.Add("  m_GameObject: {fileID: " + gameObjectFileId.ToString(CultureInfo.InvariantCulture) + "}");
            lines.Add("  m_Enabled: 1");
            lines.Add("  m_EditorHideFlags: 0");
            lines.Add("  m_Script: {fileID: 11500000, guid: " + scriptGuid + ", type: 3}");
            lines.Add("  m_Name:");
            lines.Add("  m_EditorClassIdentifier:");

            List<string> fieldKeys = new List<string>(serializedFields.Keys);
            fieldKeys.Sort(StringComparer.Ordinal);
            for (int i = 0; i < fieldKeys.Count; i++)
            {
                string k = fieldKeys[i];
                string v = serializedFields[k];
                lines.Add("  " + k + ": " + FormatYamlScalarValue(v));
            }
        }

        private static string FormatYamlScalarValue(string value)
        {
            if (string.IsNullOrEmpty(value))
            {
                return "\"\"";
            }

            bool needsQuotes = value.IndexOfAny(new[] { ':', '{', '}', '[', ']', ',', '"', '\'', '\n', '\r' }) >= 0
                || char.IsWhiteSpace(value[0])
                || char.IsWhiteSpace(value[value.Length - 1]);
            if (!needsQuotes)
            {
                return value;
            }

            return "\"" + DiagnoseReport.EscapeJson(value) + "\"";
        }

        private static string BuildGameObjectHierarchyPath(long gameObjectFileId, ParsedYamlAsset parsed)
        {
            List<string> segments = new List<string>();
            HashSet<long> visited = new HashSet<long>();
            long currentGoId = gameObjectFileId;

            while (currentGoId != 0 && visited.Add(currentGoId))
            {
                if (!parsed.GameObjectById.TryGetValue(currentGoId, out ParsedGameObjectDoc? goDoc))
                {
                    break;
                }

                segments.Add(goDoc.Name);
                if (!parsed.TransformByGameObjectId.TryGetValue(currentGoId, out ParsedTransformDoc? trDoc)
                    || trDoc.FatherTransformFileId == 0
                    || !parsed.TransformById.TryGetValue(trDoc.FatherTransformFileId, out ParsedTransformDoc? parentTr))
                {
                    break;
                }

                currentGoId = parentTr.GameObjectFileId;
            }

            segments.Reverse();
            return string.Join("/", segments);
        }

        private static string BuildTransformHierarchyPath(long transformFileId, ParsedYamlAsset parsed)
        {
            if (parsed.TransformById.TryGetValue(transformFileId, out ParsedTransformDoc? tr)
                && tr.GameObjectFileId != 0)
            {
                return BuildGameObjectHierarchyPath(tr.GameObjectFileId, parsed);
            }

            return string.Empty;
        }

        private static long ExtractFlowFileId(string line)
        {
            int idx = line.IndexOf("fileID:", StringComparison.Ordinal);
            if (idx < 0)
            {
                return 0;
            }

            string rest = line.Substring(idx + "fileID:".Length).Trim();
            int end = rest.IndexOfAny(new[] { ',', '}', ' ' });
            string token = end >= 0 ? rest.Substring(0, end).Trim() : rest;
            _ = long.TryParse(token, NumberStyles.Integer, CultureInfo.InvariantCulture, out long id);
            return id;
        }

        private static string ExtractFlowGuid(string line)
        {
            int idx = line.IndexOf("guid:", StringComparison.Ordinal);
            if (idx < 0)
            {
                return string.Empty;
            }

            string rest = line.Substring(idx + "guid:".Length).Trim();
            int end = rest.IndexOfAny(new[] { ',', '}', ' ' });
            string token = end >= 0 ? rest.Substring(0, end).Trim() : rest;
            return UnquoteYamlScalar(token).ToLowerInvariant();
        }

        private static string ExtractColonValue(string line)
        {
            int colon = line.IndexOf(':');
            if (colon < 0)
            {
                return string.Empty;
            }

            return UnquoteYamlScalar(line.Substring(colon + 1).Trim());
        }

        private static string UnquoteYamlScalar(string text)
        {
            string trimmed = text.Trim();
            if (trimmed.Length >= 2
                && ((trimmed[0] == '"' && trimmed[trimmed.Length - 1] == '"')
                    || (trimmed[0] == '\'' && trimmed[trimmed.Length - 1] == '\'')))
            {
                return trimmed.Substring(1, trimmed.Length - 2);
            }

            return trimmed;
        }
    }
}
