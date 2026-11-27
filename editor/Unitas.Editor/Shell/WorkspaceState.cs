using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Text;
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

    public sealed record WorkspacePropertyEntry(
        string Name,
        string Value);

    public sealed record WorkspaceComponentEntry(
        long FileId,
        string Type,
        IReadOnlyList<WorkspacePropertyEntry> Properties);

    public sealed class WorkspaceState
    {
        private readonly List<WorkspaceHierarchyEntry> hierarchyEntries =
            new List<WorkspaceHierarchyEntry>();
        private readonly List<WorkspaceComponentEntry> inspectorComponents =
            new List<WorkspaceComponentEntry>();

        public string ProjectPath { get; set; } = string.Empty;

        public string ActiveScenePath { get; set; } = string.Empty;

        public IReadOnlyList<WorkspaceHierarchyEntry> HierarchyEntries => hierarchyEntries;

        public string SelectedObjectId { get; set; } = string.Empty;

        public string SelectedObjectName { get; set; } = string.Empty;

        public bool SelectedObjectActive { get; set; } = true;

        public string InspectorSummary { get; set; } = string.Empty;

        public IReadOnlyList<WorkspaceComponentEntry> InspectorComponents => inspectorComponents;

        public string PlayState { get; set; } = ShellPlay.StateStopped;

        public string? ActivePlaySessionId { get; set; }

        public string TitleText { get; set; } = "unitas Editor";

        public string HierarchyText { get; set; } = "Hierarchy: (none)";

        public string InspectorText { get; set; } = "Inspector: (no selection)";

        public string PlayStateText { get; set; } = "Play: Stopped";

        public void InitializeProject(McpHost host, string sessionId, string projectPath)
        {
            ArgumentNullException.ThrowIfNull(host);
            ArgumentNullException.ThrowIfNull(sessionId);
            if (!string.IsNullOrWhiteSpace(projectPath))
            {
                ProjectPath = Path.GetFullPath(projectPath);
            }
            else if (!string.IsNullOrWhiteSpace(host.ProjectPath))
            {
                ProjectPath = host.ProjectPath!;
            }
            else
            {
                ProjectPath = Directory.GetCurrentDirectory();
            }

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
                    new JsonObject
                    {
                        ["projectPath"] = ProjectPath,
                        ["path"] = sceneRelativePath,
                    });
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

                JsonNode envelope = host.CallTool(sessionId, "scene.hierarchy", args);
                ApplyHierarchyPayload(UnwrapContent(envelope));
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
            InspectObject(host, sessionId, entry.FileId);
            RebuildFormattedTexts();
        }

        public void SelectById(McpHost host, string sessionId, string targetId)
        {
            ArgumentNullException.ThrowIfNull(host);
            ArgumentNullException.ThrowIfNull(sessionId);
            if (string.IsNullOrWhiteSpace(targetId))
            {
                return;
            }

            if (!long.TryParse(targetId, NumberStyles.Integer, CultureInfo.InvariantCulture, out long fileId))
            {
                return;
            }

            SelectedObjectId = fileId.ToString(CultureInfo.InvariantCulture);
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
                if (string.Equals(
                    hierarchyEntries[i].FileId.ToString(CultureInfo.InvariantCulture),
                    SelectedObjectId,
                    StringComparison.Ordinal))
                {
                    currentIndex = i;
                    break;
                }
            }

            SelectByIndex(host, sessionId, currentIndex + 1);
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
                ["hierarchyText"] = HierarchyText,
                ["inspectorText"] = InspectorText,
                ["playStateText"] = PlayStateText,
                ["selectedObjectId"] = SelectedObjectId,
                ["selectedObjectName"] = SelectedObjectName,
                ["playMode"] = PlayState,
                ["hierarchyCount"] = hierarchyEntries.Count,
                ["componentCount"] = inspectorComponents.Count,
            };
        }

        public string ToJson()
        {
            return ToJsonObject().ToJsonString();
        }

        private static string? LoadProjectStatusAndFirstScene(
            McpHost host,
            string sessionId,
            string projectPath)
        {
            try
            {
                JsonNode envelope = host.CallTool(
                    sessionId,
                    "project.status",
                    new JsonObject
                    {
                        ["projectPath"] = projectPath,
                    });
                JsonObject? content = UnwrapContent(envelope);
                JsonArray? scenes = content?["scenes"] as JsonArray;
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
            if (!(content["objects"] is JsonArray objects))
            {
                return;
            }

            foreach (JsonNode? node in objects)
            {
                if (!(node is JsonObject obj))
                {
                    continue;
                }

                long fileId = obj["fileId"]?.GetValue<long>() ?? 0L;
                string name = obj["name"]?.GetValue<string>() ?? "GameObject";
                long parentFileId = obj["parentFileId"]?.GetValue<long>() ?? 0L;
                List<long> children = ReadLongArray(obj["children"] as JsonArray);
                List<long> components = ReadLongArray(obj["componentFileIds"] as JsonArray);
                hierarchyEntries.Add(new WorkspaceHierarchyEntry(
                    fileId,
                    name,
                    parentFileId,
                    children,
                    components));
            }
        }

        private void EnsureSelectionAndInspect(McpHost host, string sessionId)
        {
            if (hierarchyEntries.Count == 0)
            {
                SelectedObjectId = string.Empty;
                SelectedObjectName = string.Empty;
                inspectorComponents.Clear();
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
                    new JsonObject
                    {
                        ["fileId"] = fileId,
                    });
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
            string position = content["position"]?.GetValue<string>() ?? string.Empty;
            string rotation = content["rotation"]?.GetValue<string>() ?? string.Empty;
            string scale = content["scale"]?.GetValue<string>() ?? string.Empty;
            InspectorSummary = SelectedObjectName
                + (SelectedObjectActive ? " [Active]" : " [Inactive]")
                + " Pos=" + position
                + " Rot=" + rotation
                + " Scale=" + scale;

            inspectorComponents.Clear();
            if (!(content["components"] is JsonArray components))
            {
                return;
            }

            foreach (JsonNode? compNode in components)
            {
                if (compNode is JsonObject compObj)
                {
                    inspectorComponents.Add(ParseComponentEntry(compObj));
                }
            }
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
                        string propVal = propObj["value"]?.GetValue<string>() ?? string.Empty;
                        props.Add(new WorkspacePropertyEntry(propName, propVal));
                    }
                }
            }

            return new WorkspaceComponentEntry(compId, compType, props);
        }

        public void RebuildFormattedTexts()
        {
            string sceneDisplay = string.IsNullOrWhiteSpace(ActiveScenePath)
                ? "Untitled"
                : Path.GetFileName(ActiveScenePath);
            TitleText = "unitas Editor - " + sceneDisplay;
            HierarchyText = BuildHierarchyText();
            InspectorText = BuildInspectorText();
            PlayStateText = "Play: " + PlayState;
        }

        private string BuildHierarchyText()
        {
            StringBuilder hierarchyBuilder = new StringBuilder();
            hierarchyBuilder.Append("Hierarchy (").Append(hierarchyEntries.Count).Append("):");
            int shown = Math.Min(hierarchyEntries.Count, 12);
            for (int i = 0; i < shown; i++)
            {
                WorkspaceHierarchyEntry entry = hierarchyEntries[i];
                string idText = entry.FileId.ToString(CultureInfo.InvariantCulture);
                bool selected = string.Equals(idText, SelectedObjectId, StringComparison.Ordinal);
                hierarchyBuilder
                    .Append('\n')
                    .Append(selected ? "> " : "  ")
                    .Append(entry.Name)
                    .Append(" (")
                    .Append(idText)
                    .Append(')');
            }

            return hierarchyBuilder.ToString();
        }

        private string BuildInspectorText()
        {
            if (string.IsNullOrWhiteSpace(SelectedObjectId))
            {
                return "Inspector: (no selection)";
            }

            StringBuilder inspectorBuilder = new StringBuilder();
            inspectorBuilder
                .Append("Inspector: ")
                .Append(SelectedObjectName)
                .Append(" (")
                .Append(SelectedObjectId)
                .Append(')');
            if (!string.IsNullOrWhiteSpace(InspectorSummary))
            {
                inspectorBuilder.Append('\n').Append(InspectorSummary);
            }

            foreach (WorkspaceComponentEntry comp in inspectorComponents)
            {
                inspectorBuilder
                    .Append("\n[")
                    .Append(comp.Type)
                    .Append(']');
            }

            return inspectorBuilder.ToString();
        }

        private static JsonObject? UnwrapContent(JsonNode? envelope)
        {
            if (envelope is JsonObject obj)
            {
                if (obj["content"] is JsonObject content)
                {
                    return content;
                }

                return obj;
            }

            return null;
        }

        private static List<long> ReadLongArray(JsonArray? array)
        {
            List<long> items = new List<long>();
            if (array == null)
            {
                return items;
            }

            foreach (JsonNode? item in array)
            {
                if (item is JsonValue val && val.TryGetValue(out long number))
                {
                    items.Add(number);
                }
            }

            return items;
        }
    }
}
