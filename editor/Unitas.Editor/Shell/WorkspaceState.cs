using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Text.Json.Nodes;
using Unitas.Editor.Mcp;

namespace Unitas.Editor.Shell
{
    public sealed record WorkspaceHierarchyEntry(
        long FileId,
        string Name,
        long ParentFileId,
        IReadOnlyList<long> ChildFileIds,
        IReadOnlyList<long> ComponentFileIds);

    public sealed record WorkspacePropertyEntry(string Name, string Path, string Value);

    public sealed record WorkspaceComponentEntry(
        long FileId,
        string Type,
        IReadOnlyList<WorkspacePropertyEntry> Properties);

    public sealed record WorkspaceInspectorRow(
        string Text,
        long ComponentFileId,
        string Path,
        string Value,
        string Kind);

    public sealed class WorkspaceState
    {
        public const int MaxHierarchyRows = 12;
        public const int MaxPropertyRows = 8;

        private readonly List<WorkspaceHierarchyEntry> hierarchyEntries = new List<WorkspaceHierarchyEntry>();
        private readonly List<WorkspaceComponentEntry> inspectorComponents = new List<WorkspaceComponentEntry>();
        private readonly List<WorkspaceInspectorRow> inspectorRows = new List<WorkspaceInspectorRow>();
        private readonly List<string> hierarchyRows = new List<string>();
        private readonly List<string> propertyRows = new List<string>();

        public string ProjectPath { get; set; } = string.Empty;

        public string ActiveScenePath { get; set; } = string.Empty;

        public IReadOnlyList<WorkspaceHierarchyEntry> HierarchyEntries => hierarchyEntries;

        public string SelectedObjectId { get; set; } = string.Empty;

        public string SelectedObjectName { get; set; } = string.Empty;

        public bool SelectedObjectActive { get; set; } = true;

        public string InspectorSummary { get; set; } = string.Empty;

        public IReadOnlyList<WorkspaceComponentEntry> InspectorComponents => inspectorComponents;

        public int SelectedPropertyIndex { get; set; } = -1;

        public IReadOnlyList<string> HierarchyRows => hierarchyRows;

        public IReadOnlyList<string> PropertyRows => propertyRows;

        public string PlayState { get; set; } = ShellPlay.StateStopped;

        public string? ActivePlaySessionId { get; set; }

        public string TitleText { get; set; } = "unitas Editor";

        public string PlayStateText { get; set; } = "Play: Stopped";

        public void InitializeProject(McpHost host, string sessionId, string projectPath)
        {
            ArgumentNullException.ThrowIfNull(host);
            ArgumentNullException.ThrowIfNull(sessionId);
            ProjectPath = !string.IsNullOrWhiteSpace(projectPath)
                ? Path.GetFullPath(projectPath)
                : (!string.IsNullOrWhiteSpace(host.ProjectPath) ? host.ProjectPath! : Directory.GetCurrentDirectory());

            string? firstScene = LoadProjectStatusAndFirstScene(host, sessionId, ProjectPath);
            if (!string.IsNullOrWhiteSpace(firstScene))
            {
                OpenScene(host, sessionId, firstScene!);
            }
            else
            {
                RefreshFromOpenScene(host, sessionId);
            }

            RebuildFormattedTexts();
        }

        public void OpenScene(McpHost host, string sessionId, string sceneRelativePath)
        {
            ArgumentNullException.ThrowIfNull(host);
            ArgumentNullException.ThrowIfNull(sessionId);
            ArgumentNullException.ThrowIfNull(sceneRelativePath);
            try
            {
                JsonNode envelope = host.CallTool(
                    sessionId,
                    "scene.open",
                    new JsonObject { ["projectPath"] = ProjectPath, ["path"] = sceneRelativePath });
                ApplyHierarchyPayload(UnwrapContent(envelope));
            }
            catch (McpError)
            {
            }

            EnsureSelectionAndInspect(host, sessionId);
            RebuildFormattedTexts();
        }

        public void RefreshFromOpenScene(McpHost host, string sessionId)
        {
            ArgumentNullException.ThrowIfNull(host);
            ArgumentNullException.ThrowIfNull(sessionId);
            try
            {
                JsonObject args = new JsonObject();
                if (!string.IsNullOrWhiteSpace(ProjectPath))
                {
                    args["projectPath"] = ProjectPath;
                }

                ApplyHierarchyPayload(UnwrapContent(host.CallTool(sessionId, "scene.hierarchy", args)));
            }
            catch (McpError)
            {
            }

            EnsureSelectionAndInspect(host, sessionId);
            RebuildFormattedTexts();
        }

        public void SelectByIndex(McpHost host, string sessionId, int index)
        {
            ArgumentNullException.ThrowIfNull(host);
            ArgumentNullException.ThrowIfNull(sessionId);
            if (hierarchyEntries.Count == 0)
            {
                return;
            }

            int clamped = ((index % hierarchyEntries.Count) + hierarchyEntries.Count) % hierarchyEntries.Count;
            WorkspaceHierarchyEntry entry = hierarchyEntries[clamped];
            SelectedObjectId = entry.FileId.ToString(CultureInfo.InvariantCulture);
            SelectedObjectName = entry.Name;
            SelectedPropertyIndex = -1;
            InspectObject(host, sessionId, entry.FileId);
            RebuildFormattedTexts();
        }

        public void SelectById(McpHost host, string sessionId, string targetId)
        {
            ArgumentNullException.ThrowIfNull(host);
            ArgumentNullException.ThrowIfNull(sessionId);
            if (string.IsNullOrWhiteSpace(targetId)
                || !long.TryParse(targetId, NumberStyles.Integer, CultureInfo.InvariantCulture, out long fileId))
            {
                return;
            }

            string normalized = fileId.ToString(CultureInfo.InvariantCulture);
            if (!string.Equals(SelectedObjectId, normalized, StringComparison.Ordinal))
            {
                SelectedPropertyIndex = -1;
            }

            SelectedObjectId = normalized;
            foreach (WorkspaceHierarchyEntry entry in hierarchyEntries)
            {
                if (entry.FileId == fileId)
                {
                    SelectedObjectName = entry.Name;
                    break;
                }
            }

            InspectObject(host, sessionId, fileId);
            RebuildFormattedTexts();
        }

        public void SelectNext(McpHost host, string sessionId)
        {
            ArgumentNullException.ThrowIfNull(host);
            ArgumentNullException.ThrowIfNull(sessionId);
            if (hierarchyEntries.Count == 0)
            {
                return;
            }

            int currentIndex = -1;
            for (int i = 0; i < hierarchyEntries.Count; i++)
            {
                string idText = hierarchyEntries[i].FileId.ToString(CultureInfo.InvariantCulture);
                if (string.Equals(idText, SelectedObjectId, StringComparison.Ordinal))
                {
                    currentIndex = i;
                    break;
                }
            }

            SelectByIndex(host, sessionId, currentIndex + 1);
        }

        public void SelectProperty(int index)
        {
            if (inspectorRows.Count == 0 || index < 0 || index >= inspectorRows.Count)
            {
                return;
            }

            for (int i = index; i < inspectorRows.Count; i++)
            {
                if (inspectorRows[i].ComponentFileId != 0)
                {
                    SelectedPropertyIndex = i;
                    RebuildFormattedTexts();
                    return;
                }
            }
        }

        public void StepPropertyValue(McpHost host, string sessionId, double delta)
        {
            ArgumentNullException.ThrowIfNull(host);
            ArgumentNullException.ThrowIfNull(sessionId);
            WorkspaceInspectorRow? selected = GetSelectedPropertyRow();
            if (selected != null && TryStepNumericValue(selected.Value, delta, out string nextValue))
            {
                ApplyPropertyMutation(host, sessionId, selected.ComponentFileId, selected.Path, nextValue);
            }
        }

        public void TogglePropertyValue(McpHost host, string sessionId)
        {
            ArgumentNullException.ThrowIfNull(host);
            ArgumentNullException.ThrowIfNull(sessionId);
            WorkspaceInspectorRow? selected = GetSelectedPropertyRow();
            if (selected != null && TryToggleBoolValue(selected.Value, out string nextValue))
            {
                ApplyPropertyMutation(host, sessionId, selected.ComponentFileId, selected.Path, nextValue);
            }
        }

        public void ToggleActive(McpHost host, string sessionId)
        {
            ArgumentNullException.ThrowIfNull(host);
            ArgumentNullException.ThrowIfNull(sessionId);
            if (!long.TryParse(SelectedObjectId, NumberStyles.Integer, CultureInfo.InvariantCulture, out long fileId))
            {
                return;
            }

            try
            {
                host.CallTool(
                    sessionId,
                    "scene.set_active",
                    new JsonObject { ["fileId"] = fileId, ["active"] = !SelectedObjectActive });
                RefreshFromOpenScene(host, sessionId);
            }
            catch (McpError)
            {
            }
        }

        public void HandlePlayCommand(McpHost host, string sessionId, string command, JsonObject? arguments)
        {
            ShellPlay.ExecuteCommand(host, sessionId, this, command, arguments);
        }

        public JsonObject ToJsonObject()
        {
            return new JsonObject
            {
                ["projectPath"] = ProjectPath,
                ["activeScenePath"] = ActiveScenePath,
                ["titleText"] = TitleText,
                ["hierarchyRows"] = BuildStringJsonArray(hierarchyRows),
                ["propertyRows"] = BuildStringJsonArray(propertyRows),
                ["playStateText"] = PlayStateText,
                ["selectedObjectId"] = SelectedObjectId,
                ["selectedObjectName"] = SelectedObjectName,
                ["selectedObjectActive"] = SelectedObjectActive,
                ["selectedPropertyIndex"] = SelectedPropertyIndex,
                ["playMode"] = PlayState,
                ["hierarchyCount"] = hierarchyEntries.Count,
                ["componentCount"] = inspectorComponents.Count,
            };
        }

        public string ToJson() => ToJsonObject().ToJsonString();

        public void RebuildFormattedTexts()
        {
            string sceneDisplay = string.IsNullOrWhiteSpace(ActiveScenePath)
                ? "Untitled"
                : Path.GetFileName(ActiveScenePath);
            TitleText = "unitas Editor - " + sceneDisplay;
            PlayStateText = "Play: " + PlayState;
            RebuildHierarchyRows();
            RebuildPropertyRows();
        }

        private void ApplyPropertyMutation(
            McpHost host,
            string sessionId,
            long componentFileId,
            string propertyPath,
            string nextValue)
        {
            int preservedIndex = SelectedPropertyIndex;
            try
            {
                host.CallTool(
                    sessionId,
                    "scene.set_property",
                    new JsonObject { ["fileId"] = componentFileId, ["path"] = propertyPath, ["value"] = nextValue });
                RefreshFromOpenScene(host, sessionId);
                SelectedPropertyIndex = preservedIndex;
                RebuildFormattedTexts();
            }
            catch (McpError)
            {
            }
        }

        private WorkspaceInspectorRow? GetSelectedPropertyRow()
        {
            return SelectedPropertyIndex >= 0
                && SelectedPropertyIndex < inspectorRows.Count
                && inspectorRows[SelectedPropertyIndex].ComponentFileId != 0
                ? inspectorRows[SelectedPropertyIndex]
                : null;
        }

        private static bool TryStepNumericValue(string current, double delta, out string formatted)
        {
            if (long.TryParse(current, NumberStyles.Integer, CultureInfo.InvariantCulture, out long integral)
                && Math.Abs(delta) >= 1.0)
            {
                formatted = (integral + (long)Math.Round(delta)).ToString(CultureInfo.InvariantCulture);
                return true;
            }

            if (double.TryParse(current, NumberStyles.Float, CultureInfo.InvariantCulture, out double number))
            {
                double increment = Math.Abs(delta) >= 1.0 ? delta * 0.1 : delta;
                formatted = Math.Round(number + increment, 6).ToString(CultureInfo.InvariantCulture);
                return true;
            }

            formatted = string.Empty;
            return false;
        }

        private static bool TryToggleBoolValue(string current, out string toggled)
        {
            toggled = string.Equals(current, "1", StringComparison.Ordinal) ? "0"
                : string.Equals(current, "0", StringComparison.Ordinal) ? "1"
                : string.Equals(current, "true", StringComparison.OrdinalIgnoreCase) ? "false"
                : string.Equals(current, "false", StringComparison.OrdinalIgnoreCase) ? "true"
                : string.Empty;
            return toggled.Length > 0;
        }

        private static string ClassifyPropertyKind(string value)
        {
            return TryToggleBoolValue(value, out _) ? "bool"
                : (double.TryParse(value, NumberStyles.Float, CultureInfo.InvariantCulture, out _) ? "numeric" : "read-only");
        }

        private static bool IsInspectorBoilerplateKey(string key)
        {
            return string.Equals(key, "m_ObjectHideFlags", StringComparison.Ordinal)
                || string.Equals(key, "m_CorrespondingSourceObject", StringComparison.Ordinal)
                || string.Equals(key, "m_PrefabInstance", StringComparison.Ordinal)
                || string.Equals(key, "m_PrefabAsset", StringComparison.Ordinal)
                || string.Equals(key, "m_GameObject", StringComparison.Ordinal)
                || string.Equals(key, "serializedVersion", StringComparison.Ordinal)
                || string.Equals(key, "m_Children", StringComparison.Ordinal)
                || string.Equals(key, "m_Father", StringComparison.Ordinal)
                || string.Equals(key, "m_EditorHideFlags", StringComparison.Ordinal)
                || string.Equals(key, "m_Script", StringComparison.Ordinal)
                || string.Equals(key, "m_EditorClassIdentifier", StringComparison.Ordinal)
                || string.Equals(key, "m_IncludeLayers", StringComparison.Ordinal)
                || string.Equals(key, "m_ExcludeLayers", StringComparison.Ordinal);
        }

        private void RebuildHierarchyRows()
        {
            hierarchyRows.Clear();
            bool truncated = hierarchyEntries.Count > MaxHierarchyRows;
            int shown = truncated ? MaxHierarchyRows - 1 : hierarchyEntries.Count;
            for (int i = 0; i < shown; i++)
            {
                WorkspaceHierarchyEntry entry = hierarchyEntries[i];
                string idText = entry.FileId.ToString(CultureInfo.InvariantCulture);
                bool selected = string.Equals(idText, SelectedObjectId, StringComparison.Ordinal);
                hierarchyRows.Add((selected ? "> " : "  ") + entry.Name + " (" + idText + ")");
            }

            if (truncated)
            {
                hierarchyRows.Add("...");
            }
        }

        private void RebuildInspectorRows()
        {
            int previousSelection = SelectedPropertyIndex;
            inspectorRows.Clear();
            foreach (WorkspaceComponentEntry comp in inspectorComponents)
            {
                inspectorRows.Add(new WorkspaceInspectorRow("[" + comp.Type + "]", 0L, string.Empty, string.Empty, "header"));
                foreach (WorkspacePropertyEntry prop in comp.Properties)
                {
                    if (!IsInspectorBoilerplateKey(prop.Name))
                    {
                        inspectorRows.Add(new WorkspaceInspectorRow(
                            prop.Name + " = " + prop.Value,
                            comp.FileId,
                            prop.Path,
                            prop.Value,
                            ClassifyPropertyKind(prop.Value)));
                    }
                }
            }

            SelectedPropertyIndex = previousSelection >= 0
                && previousSelection < inspectorRows.Count
                && inspectorRows[previousSelection].ComponentFileId != 0
                ? previousSelection
                : FindDefaultPropertyIndex();
        }

        private int FindDefaultPropertyIndex()
        {
            int firstAny = -1;
            for (int i = 0; i < inspectorRows.Count; i++)
            {
                if (inspectorRows[i].ComponentFileId != 0)
                {
                    if (firstAny < 0)
                    {
                        firstAny = i;
                    }

                    if (!string.Equals(inspectorRows[i].Kind, "read-only", StringComparison.Ordinal))
                    {
                        return i;
                    }
                }
            }

            return firstAny;
        }

        private void RebuildPropertyRows()
        {
            propertyRows.Clear();
            bool truncated = inspectorRows.Count > MaxPropertyRows;
            int shown = truncated ? MaxPropertyRows - 1 : inspectorRows.Count;
            for (int i = 0; i < shown; i++)
            {
                WorkspaceInspectorRow row = inspectorRows[i];
                propertyRows.Add(row.ComponentFileId == 0
                    ? row.Text
                    : (i == SelectedPropertyIndex ? "> " : "  ") + row.Text);
            }

            if (truncated)
            {
                propertyRows.Add("...");
            }
        }

        private static string? LoadProjectStatusAndFirstScene(McpHost host, string sessionId, string projectPath)
        {
            try
            {
                JsonNode envelope = host.CallTool(
                    sessionId,
                    "project.status",
                    new JsonObject { ["projectPath"] = projectPath });
                JsonArray? scenes = UnwrapContent(envelope)?["scenes"] as JsonArray;
                if (scenes != null && scenes.Count > 0 && scenes[0] is JsonObject first)
                {
                    return first["path"]?.GetValue<string>();
                }
            }
            catch (McpError)
            {
            }

            return null;
        }

        private void ApplyHierarchyPayload(JsonObject? content)
        {
            if (content == null)
            {
                return;
            }

            string? path = content["path"]?.GetValue<string>();
            if (!string.IsNullOrWhiteSpace(path))
            {
                ActiveScenePath = path!;
            }

            hierarchyEntries.Clear();
            if (content["objects"] is JsonArray objects)
            {
                foreach (JsonNode? node in objects)
                {
                    if (node is JsonObject obj)
                    {
                        hierarchyEntries.Add(new WorkspaceHierarchyEntry(
                            obj["fileId"]?.GetValue<long>() ?? 0L,
                            obj["name"]?.GetValue<string>() ?? "GameObject",
                            obj["parentFileId"]?.GetValue<long>() ?? 0L,
                            ReadLongArray(obj["children"] as JsonArray),
                            ReadLongArray(obj["componentFileIds"] as JsonArray)));
                    }
                }
            }
        }

        private void EnsureSelectionAndInspect(McpHost host, string sessionId)
        {
            if (hierarchyEntries.Count == 0)
            {
                SelectedObjectId = string.Empty;
                SelectedObjectName = string.Empty;
                SelectedPropertyIndex = -1;
                inspectorComponents.Clear();
                inspectorRows.Clear();
                InspectorSummary = "No objects in scene";
                return;
            }

            long targetFileId = hierarchyEntries[0].FileId;
            string targetName = hierarchyEntries[0].Name;
            if (long.TryParse(SelectedObjectId, NumberStyles.Integer, CultureInfo.InvariantCulture, out long existingId))
            {
                foreach (WorkspaceHierarchyEntry entry in hierarchyEntries)
                {
                    if (entry.FileId == existingId)
                    {
                        targetFileId = entry.FileId;
                        targetName = entry.Name;
                        break;
                    }
                }
            }

            SelectedObjectId = targetFileId.ToString(CultureInfo.InvariantCulture);
            SelectedObjectName = targetName;
            InspectObject(host, sessionId, targetFileId);
        }

        private void InspectObject(McpHost host, string sessionId, long fileId)
        {
            try
            {
                JsonNode envelope = host.CallTool(
                    sessionId,
                    "scene.inspect_object",
                    new JsonObject { ["fileId"] = fileId });
                ApplyInspectionPayload(UnwrapContent(envelope));
            }
            catch (McpError)
            {
            }
        }

        private void ApplyInspectionPayload(JsonObject? content)
        {
            if (content == null)
            {
                return;
            }

            SelectedObjectName = content["name"]?.GetValue<string>() ?? SelectedObjectName;
            SelectedObjectActive = content["active"]?.GetValue<bool>() ?? true;
            InspectorSummary = SelectedObjectName
                + (SelectedObjectActive ? " [Active]" : " [Inactive]")
                + " Pos=" + (content["position"]?.GetValue<string>() ?? string.Empty)
                + " Rot=" + (content["rotation"]?.GetValue<string>() ?? string.Empty)
                + " Scale=" + (content["scale"]?.GetValue<string>() ?? string.Empty);

            inspectorComponents.Clear();
            if (content["components"] is JsonArray components)
            {
                foreach (JsonNode? compNode in components)
                {
                    if (compNode is JsonObject compObj)
                    {
                        inspectorComponents.Add(ParseComponentEntry(compObj));
                    }
                }
            }

            RebuildInspectorRows();
        }

        private static WorkspaceComponentEntry ParseComponentEntry(JsonObject compObj)
        {
            long compId = compObj["fileId"]?.GetValue<long>() ?? 0L;
            string compType = compObj["type"]?.GetValue<string>() ?? "Component";
            List<WorkspacePropertyEntry> props = new List<WorkspacePropertyEntry>();
            if (compObj["properties"] is JsonArray propArray)
            {
                foreach (JsonNode? propNode in propArray)
                {
                    if (propNode is JsonObject propObj)
                    {
                        string propName = propObj["name"]?.GetValue<string>() ?? string.Empty;
                        string propPath = propObj["path"]?.GetValue<string>() ?? propName;
                        string propVal = propObj["value"]?.GetValue<string>() ?? string.Empty;
                        props.Add(new WorkspacePropertyEntry(propName, propPath, propVal));
                    }
                }
            }

            return new WorkspaceComponentEntry(compId, compType, props);
        }

        private static JsonArray BuildStringJsonArray(IReadOnlyList<string> items)
        {
            JsonArray arr = new JsonArray();
            foreach (string item in items)
            {
                arr.Add(item);
            }

            return arr;
        }

        private static JsonObject? UnwrapContent(JsonNode? envelope) =>
            envelope is JsonObject obj ? (obj["content"] as JsonObject ?? obj) : null;

        private static List<long> ReadLongArray(JsonArray? array)
        {
            List<long> items = new List<long>();
            if (array != null)
            {
                foreach (JsonNode? item in array)
                {
                    if (item is JsonValue val && val.TryGetValue(out long number))
                    {
                        items.Add(number);
                    }
                }
            }

            return items;
        }
    }
}
