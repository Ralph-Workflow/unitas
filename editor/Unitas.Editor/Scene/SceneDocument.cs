using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Text;

namespace Unitas.Editor.Scene
{
    /// <summary>
    /// A Unity tagged YAML scene or prefab held as documents. Every line is kept verbatim, so an
    /// unmodified Save is byte identical to the file that was read.
    /// </summary>
    public sealed class SceneDocument
    {
        private const int GameObjectClassId = 1;
        private const int TransformClassId = 4;
        private const int SceneRootsClassId = 1660057539;

        private readonly string path;
        private readonly List<string> preamble = new List<string>();
        private readonly List<SceneNode> documents = new List<SceneNode>();
        private readonly List<SceneObject> objects = new List<SceneObject>();
        private readonly Dictionary<long, SceneObject> objectsByFileId = new Dictionary<long, SceneObject>();
        private string lineEnding = "\n";

        private SceneDocument(string filePath)
        {
            path = filePath;
        }

        public static SceneDocument Open(string filePath)
        {
            ArgumentNullException.ThrowIfNull(filePath);

            string text = Encoding.UTF8.GetString(File.ReadAllBytes(filePath));
            return Parse(filePath, text);
        }

        public static SceneDocument Parse(string filePath, string text)
        {
            ArgumentNullException.ThrowIfNull(filePath);
            ArgumentNullException.ThrowIfNull(text);

            SceneDocument document = new SceneDocument(filePath);
            document.Load(text);
            return document;
        }

        public IReadOnlyList<SceneObject> Objects => objects;

        /// <summary>The GameObject with that file id; unknown ids raise SceneParseException.</summary>
        public SceneObject Find(long fileId)
        {
            SceneObject? found;
            if (!objectsByFileId.TryGetValue(fileId, out found))
            {
                throw new SceneParseException(path, 0, "no GameObject with fileID " + Format(fileId));
            }

            return found;
        }

        /// <summary>
        /// Applies the op and returns its inverse. The inverse carries the document text from
        /// before the edit, so applying it restores that text byte for byte.
        /// </summary>
        public SceneOp Apply(SceneOp op)
        {
            ArgumentNullException.ThrowIfNull(op);

            string before = Save();
            if (op.DocumentSnapshot.Length > 0)
            {
                Load(op.DocumentSnapshot);
                return op.WithSnapshot(before);
            }

            switch (op.Kind)
            {
                case SceneOpKind.SetProperty:
                    {
                        string previous = ApplySetProperty(op);
                        return SceneOp.SetProperty(op.FileId, op.PropertyPath, previous).WithSnapshot(before);
                    }

                case SceneOpKind.CreateGameObject:
                    {
                        long created = ApplyCreateGameObject(op);
                        return SceneOp.DestroyGameObject(created).WithSnapshot(before);
                    }

                case SceneOpKind.DestroyGameObject:
                    {
                        long parentFileId;
                        string name = ApplyDestroyGameObject(op, out parentFileId);
                        return SceneOp.CreateGameObject(name, parentFileId).WithSnapshot(before);
                    }

                default:
                    throw new SceneParseException(path, 0, "unsupported scene op kind " + op.Kind.ToString());
            }
        }

        public string Save()
        {
            StringBuilder builder = new StringBuilder();
            foreach (string line in preamble)
            {
                builder.Append(line);
            }

            foreach (SceneNode node in documents)
            {
                builder.Append(node.Header);
                foreach (string line in node.Body)
                {
                    builder.Append(line);
                }
            }

            return builder.ToString();
        }

        public void SaveTo(string filePath)
        {
            ArgumentNullException.ThrowIfNull(filePath);

            File.WriteAllText(filePath, Save(), new UTF8Encoding(false));
        }

        private void Load(string text)
        {
            preamble.Clear();
            documents.Clear();
            List<string> lines = SceneYaml.SplitLines(text);
            lineEnding = DetectLineEnding(lines);
            int index = 0;
            while (index < lines.Count && !SceneYaml.IsHeaderStart(lines[index]))
            {
                preamble.Add(lines[index]);
                index++;
            }

            while (index < lines.Count)
            {
                SceneNode node = new SceneNode();
                node.Header = lines[index];
                int classId;
                long fileId;
                bool stripped;
                if (!SceneYaml.TryParseHeader(SceneYaml.Content(lines[index]), out classId, out fileId, out stripped))
                {
                    throw new SceneParseException(
                        path,
                        index + 1,
                        "malformed document header '" + SceneYaml.Content(lines[index]).Trim() + "'");
                }

                node.ClassId = classId;
                node.FileId = fileId;
                node.Stripped = stripped;
                index++;
                while (index < lines.Count && !SceneYaml.IsHeaderStart(lines[index]))
                {
                    node.Body.Add(lines[index]);
                    index++;
                }

                documents.Add(node);
            }

            RebuildModel();
        }

        private void RebuildModel()
        {
            objects.Clear();
            objectsByFileId.Clear();

            Dictionary<long, SceneNode> transforms = new Dictionary<long, SceneNode>();
            Dictionary<long, long> ownerOfTransform = new Dictionary<long, long>();
            Dictionary<long, long> transformOfGameObject = new Dictionary<long, long>();

            foreach (SceneNode node in documents)
            {
                if (node.ClassId != TransformClassId)
                {
                    continue;
                }

                transforms[node.FileId] = node;
                long owner;
                if (TryGetFileId(node, "m_GameObject", out owner))
                {
                    ownerOfTransform[node.FileId] = owner;
                    transformOfGameObject[owner] = node.FileId;
                }
            }

            foreach (SceneNode node in documents)
            {
                if (node.ClassId != GameObjectClassId)
                {
                    continue;
                }

                long transformFileId;
                if (!transformOfGameObject.TryGetValue(node.FileId, out transformFileId))
                {
                    transformFileId = 0;
                }

                long parentFileId = 0;
                if (transformFileId != 0)
                {
                    long fatherTransformFileId;
                    if (TryGetFileId(transforms[transformFileId], "m_Father", out fatherTransformFileId))
                    {
                        long owner;
                        if (ownerOfTransform.TryGetValue(fatherTransformFileId, out owner))
                        {
                            parentFileId = owner;
                        }
                    }
                }

                SceneObject sceneObject = new SceneObject(node.FileId, ReadScalar(node, "m_Name"), parentFileId);
                foreach (long componentFileId in ReadComponentIds(node))
                {
                    sceneObject.AddComponent(componentFileId);
                }

                objects.Add(sceneObject);
                objectsByFileId[node.FileId] = sceneObject;
            }

            foreach (SceneObject sceneObject in objects)
            {
                SceneObject? parent;
                if (sceneObject.ParentFileId != 0 && objectsByFileId.TryGetValue(sceneObject.ParentFileId, out parent))
                {
                    parent.AddChild(sceneObject);
                }
            }
        }

        private string ApplySetProperty(SceneOp op)
        {
            SceneNode? node = FindNode(op.FileId);
            if (node == null)
            {
                throw new SceneParseException(path, 0, "no document with fileID " + Format(op.FileId));
            }

            string propertyPath = op.PropertyPath;
            int separator = propertyPath.IndexOf('.', StringComparison.Ordinal);
            string key = separator < 0 ? propertyPath : propertyPath[..separator];
            string? member = separator < 0 ? null : propertyPath[(separator + 1)..];
            int lineIndex;
            int keyIndent;
            if (!TryFindKeyLine(node, key, out lineIndex, out keyIndent))
            {
                throw new SceneParseException(path, 0, "document " + Format(op.FileId) + " has no property '" + key + "'");
            }

            string content = SceneYaml.Content(node.Body[lineIndex]);
            int colon = content.IndexOf(':', StringComparison.Ordinal);
            string prefix = content[..(colon + 1)];
            string rest = content[(colon + 1)..];
            string previous;
            string? replaced;
            if (member == null)
            {
                previous = rest.Trim();
                replaced = ReplaceScalar(rest, op.Value);
            }
            else
            {
                string flowMember;
                replaced = ReplaceFlowMember(rest, member, op.Value, out flowMember);
                if (replaced == null)
                {
                    throw new SceneParseException(path, 0, "property '" + propertyPath + "' has no member '" + member + "'");
                }

                previous = flowMember;
            }

            node.Body[lineIndex] = prefix + replaced + SceneYaml.Terminator(node.Body[lineIndex]);
            return previous;
        }

        private long ApplyCreateGameObject(SceneOp op)
        {
            string name = op.ObjectName;
            if (name.Contains('\n', StringComparison.Ordinal) || name.Contains('\r', StringComparison.Ordinal))
            {
                throw new SceneParseException(path, 0, "object name must not contain a line break");
            }

            SceneNode? parentTransform = null;
            if (op.ParentFileId != 0)
            {
                if (FindNode(op.ParentFileId, GameObjectClassId) == null)
                {
                    throw new SceneParseException(path, 0, "no GameObject with fileID " + Format(op.ParentFileId));
                }

                parentTransform = FindTransformForGameObject(op.ParentFileId);
                if (parentTransform == null)
                {
                    throw new SceneParseException(path, 0, "GameObject " + Format(op.ParentFileId) + " has no Transform");
                }
            }

            long gameObjectFileId = NextFreeFileId(MaxFileId());
            long transformFileId = NextFreeFileId(gameObjectFileId);
            AppendNode(BuildGameObjectNode(gameObjectFileId, transformFileId, name));
            AppendNode(BuildTransformNode(transformFileId, gameObjectFileId, parentTransform == null ? 0 : parentTransform.FileId));
            if (parentTransform != null)
            {
                AddFileIdToList(parentTransform, "m_Children", transformFileId);
            }
            else
            {
                SceneNode? roots = FindNodeByClass(SceneRootsClassId);
                if (roots != null)
                {
                    AddFileIdToList(roots, "m_Roots", transformFileId);
                }
            }

            RebuildModel();
            return gameObjectFileId;
        }

        private string ApplyDestroyGameObject(SceneOp op, out long parentFileId)
        {
            SceneNode? gameObject = FindNode(op.FileId, GameObjectClassId);
            if (gameObject == null)
            {
                throw new SceneParseException(path, 0, "no GameObject with fileID " + Format(op.FileId));
            }

            string name = ReadScalar(gameObject, "m_Name");
            List<long> componentFileIds = ReadComponentIds(gameObject);
            SceneNode? transform = FindTransformForGameObject(op.FileId);
            long transformFileId = transform == null ? 0 : transform.FileId;
            long fatherFileId = 0;
            if (transform != null)
            {
                TryGetFileId(transform, "m_Father", out fatherFileId);
            }

            parentFileId = 0;
            if (fatherFileId != 0)
            {
                SceneNode? parentTransform = FindNode(fatherFileId, TransformClassId);
                if (parentTransform != null)
                {
                    long owner;
                    if (TryGetFileId(parentTransform, "m_GameObject", out owner))
                    {
                        parentFileId = owner;
                    }

                    RemoveFileIdFromList(parentTransform, "m_Children", transformFileId);
                }
            }
            else if (transformFileId != 0)
            {
                SceneNode? roots = FindNodeByClass(SceneRootsClassId);
                if (roots != null)
                {
                    RemoveFileIdFromList(roots, "m_Roots", transformFileId);
                }
            }

            foreach (long componentFileId in componentFileIds)
            {
                if (componentFileId == transformFileId || IsComponentShared(componentFileId, op.FileId))
                {
                    continue;
                }

                RemoveNode(componentFileId);
            }

            RemoveNode(op.FileId);
            if (transformFileId != 0)
            {
                RemoveNode(transformFileId);
            }

            RebuildModel();
            return name;
        }

        private SceneNode BuildGameObjectNode(long gameObjectFileId, long transformFileId, string name)
        {
            SceneNode node = new SceneNode();
            node.Header = "--- !u!" + GameObjectClassId.ToString(CultureInfo.InvariantCulture) + " &" + Format(gameObjectFileId) + lineEnding;
            node.ClassId = GameObjectClassId;
            node.FileId = gameObjectFileId;
            node.Body.Add("GameObject:" + lineEnding);
            node.Body.Add("  m_ObjectHideFlags: 0" + lineEnding);
            node.Body.Add("  m_CorrespondingSourceObject: {fileID: 0}" + lineEnding);
            node.Body.Add("  m_PrefabInstance: {fileID: 0}" + lineEnding);
            node.Body.Add("  m_PrefabAsset: {fileID: 0}" + lineEnding);
            node.Body.Add("  serializedVersion: 6" + lineEnding);
            node.Body.Add("  m_Component:" + lineEnding);
            node.Body.Add("  - component: {fileID: " + Format(transformFileId) + "}" + lineEnding);
            node.Body.Add("  m_Layer: 0" + lineEnding);
            node.Body.Add("  m_Name: " + YamlScalar(name) + lineEnding);
            node.Body.Add("  m_TagString: Untagged" + lineEnding);
            node.Body.Add("  m_Icon: {fileID: 0}" + lineEnding);
            node.Body.Add("  m_NavMeshLayer: 0" + lineEnding);
            node.Body.Add("  m_StaticEditorFlags: 0" + lineEnding);
            node.Body.Add("  m_IsActive: 1" + lineEnding);
            return node;
        }

        private SceneNode BuildTransformNode(long transformFileId, long gameObjectFileId, long fatherFileId)
        {
            SceneNode node = new SceneNode();
            node.Header = "--- !u!" + TransformClassId.ToString(CultureInfo.InvariantCulture) + " &" + Format(transformFileId) + lineEnding;
            node.ClassId = TransformClassId;
            node.FileId = transformFileId;
            node.Body.Add("Transform:" + lineEnding);
            node.Body.Add("  m_ObjectHideFlags: 0" + lineEnding);
            node.Body.Add("  m_CorrespondingSourceObject: {fileID: 0}" + lineEnding);
            node.Body.Add("  m_PrefabInstance: {fileID: 0}" + lineEnding);
            node.Body.Add("  m_PrefabAsset: {fileID: 0}" + lineEnding);
            node.Body.Add("  m_GameObject: {fileID: " + Format(gameObjectFileId) + "}" + lineEnding);
            node.Body.Add("  serializedVersion: 2" + lineEnding);
            node.Body.Add("  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}" + lineEnding);
            node.Body.Add("  m_LocalPosition: {x: 0, y: 0, z: 0}" + lineEnding);
            node.Body.Add("  m_LocalScale: {x: 1, y: 1, z: 1}" + lineEnding);
            node.Body.Add("  m_Children: []" + lineEnding);
            node.Body.Add("  m_Father: {fileID: " + Format(fatherFileId) + "}" + lineEnding);
            return node;
        }

        private void AppendNode(SceneNode node)
        {
            EnsureTrailingLineBreak();
            documents.Add(node);
        }

        private void EnsureTrailingLineBreak()
        {
            if (documents.Count > 0)
            {
                SceneNode previous = documents[documents.Count - 1];
                if (previous.Body.Count > 0)
                {
                    int last = previous.Body.Count - 1;
                    if (SceneYaml.Terminator(previous.Body[last]).Length == 0)
                    {
                        previous.Body[last] = previous.Body[last] + lineEnding;
                    }
                }
                else if (SceneYaml.Terminator(previous.Header).Length == 0)
                {
                    previous.Header = previous.Header + lineEnding;
                }

                return;
            }

            if (preamble.Count > 0)
            {
                int last = preamble.Count - 1;
                if (SceneYaml.Terminator(preamble[last]).Length == 0)
                {
                    preamble[last] = preamble[last] + lineEnding;
                }
            }
        }

        private void AddFileIdToList(SceneNode node, string key, long fileId)
        {
            int keyIndex;
            int keyIndent;
            if (!TryFindKeyLine(node, key, out keyIndex, out keyIndent))
            {
                throw new SceneParseException(path, 0, "document " + Format(node.FileId) + " has no '" + key + "' list");
            }

            string content = SceneYaml.Content(node.Body[keyIndex]);
            int colon = content.IndexOf(':', StringComparison.Ordinal);
            string value = content[(colon + 1)..].Trim();
            string indent = new string(' ', keyIndent);
            string terminator = SceneYaml.Terminator(node.Body[keyIndex]);
            string item = indent + "- {fileID: " + Format(fileId) + "}";
            if (value.StartsWith('{'))
            {
                List<long> listed = ParseFlowFileIds(value);
                node.Body[keyIndex] = indent + key + ":" + terminator;
                int insertAt = keyIndex + 1;
                foreach (long listedFileId in listed)
                {
                    node.Body.Insert(insertAt, indent + "- {fileID: " + Format(listedFileId) + "}" + terminator);
                    insertAt++;
                }

                node.Body.Insert(insertAt, item + terminator);
                return;
            }

            if (value.Length == 0 || string.Equals(value, "[]", StringComparison.Ordinal))
            {
                if (value.Length > 0)
                {
                    node.Body[keyIndex] = indent + key + ":" + terminator;
                }

                node.Body.Insert(keyIndex + 1, item + terminator);
                return;
            }

            int last = keyIndex + 1;
            while (last < node.Body.Count && IsListItem(node.Body[last], keyIndent))
            {
                last++;
            }

            node.Body.Insert(last, item + lineEnding);
        }

        private void RemoveFileIdFromList(SceneNode node, string key, long fileId)
        {
            int keyIndex;
            int keyIndent;
            if (!TryFindKeyLine(node, key, out keyIndex, out keyIndent))
            {
                return;
            }

            int first = keyIndex + 1;
            int last = first;
            while (last < node.Body.Count && IsListItem(node.Body[last], keyIndent))
            {
                last++;
            }

            int target = -1;
            for (int index = first; index < last; index++)
            {
                long listedFileId;
                if (TryParseFileId(SceneYaml.Content(node.Body[index]), out listedFileId) && listedFileId == fileId)
                {
                    target = index;
                    break;
                }
            }

            if (target < 0)
            {
                return;
            }

            node.Body.RemoveAt(target);
            last--;
            if (last == first)
            {
                string terminator = SceneYaml.Terminator(node.Body[keyIndex]);
                node.Body[keyIndex] = new string(' ', keyIndent) + key + ": []" + terminator;
            }
        }

        private static bool IsListItem(string line, int indent)
        {
            string content = SceneYaml.Content(line);
            if (SceneYaml.Indent(content) != indent)
            {
                return false;
            }

            return content.TrimStart().StartsWith('-');
        }

        private static List<long> ParseFlowFileIds(string value)
        {
            List<long> fileIds = new List<long>();
            int index = 0;
            while (index < value.Length)
            {
                int brace = value.IndexOf('{', index);
                if (brace < 0)
                {
                    break;
                }

                int close = value.IndexOf('}', brace);
                if (close < 0)
                {
                    break;
                }

                long fileId;
                if (TryParseFileId(value[brace..(close + 1)], out fileId))
                {
                    fileIds.Add(fileId);
                }

                index = close + 1;
            }

            return fileIds;
        }

        private static bool TryParseFileId(string value, out long fileId)
        {
            fileId = 0;
            int marker = value.IndexOf("fileID:", StringComparison.Ordinal);
            if (marker < 0)
            {
                return false;
            }

            int index = marker + "fileID:".Length;
            while (index < value.Length && (value[index] == ' ' || value[index] == '\t'))
            {
                index++;
            }

            int start = index;
            if (index < value.Length && (value[index] == '-' || value[index] == '+'))
            {
                index++;
            }

            while (index < value.Length && value[index] >= '0' && value[index] <= '9')
            {
                index++;
            }

            if (index == start)
            {
                return false;
            }

            return long.TryParse(
                value[start..index],
                NumberStyles.AllowLeadingSign,
                CultureInfo.InvariantCulture,
                out fileId);
        }

        private static bool TryFindKeyLine(SceneNode node, string key, out int lineIndex, out int keyIndent)
        {
            int baseIndent = BodyIndent(node);
            for (int index = 0; index < node.Body.Count; index++)
            {
                string content = SceneYaml.Content(node.Body[index]);
                if (SceneYaml.Indent(content) != baseIndent + 2)
                {
                    continue;
                }

                int colon = content.IndexOf(':', StringComparison.Ordinal);
                if (colon < baseIndent + 2)
                {
                    continue;
                }

                if (!string.Equals(content[(baseIndent + 2)..colon].Trim(), key, StringComparison.Ordinal))
                {
                    continue;
                }

                lineIndex = index;
                keyIndent = baseIndent + 2;
                return true;
            }

            lineIndex = -1;
            keyIndent = 0;
            return false;
        }

        private static int BodyIndent(SceneNode node)
        {
            foreach (string line in node.Body)
            {
                string content = SceneYaml.Content(line);
                if (content.Trim().Length == 0)
                {
                    continue;
                }

                return SceneYaml.Indent(content);
            }

            return 0;
        }

        private static string ReadScalar(SceneNode node, string key)
        {
            int lineIndex;
            int keyIndent;
            if (!TryFindKeyLine(node, key, out lineIndex, out keyIndent))
            {
                return string.Empty;
            }

            string content = SceneYaml.Content(node.Body[lineIndex]);
            int colon = content.IndexOf(':', StringComparison.Ordinal);
            if (colon < 0)
            {
                return string.Empty;
            }

            return content[(colon + 1)..].Trim();
        }

        private static bool TryGetFileId(SceneNode node, string key, out long fileId)
        {
            fileId = 0;
            int lineIndex;
            int keyIndent;
            if (!TryFindKeyLine(node, key, out lineIndex, out keyIndent))
            {
                return false;
            }

            string content = SceneYaml.Content(node.Body[lineIndex]);
            int colon = content.IndexOf(':', StringComparison.Ordinal);
            if (colon < 0)
            {
                return false;
            }

            return TryParseFileId(content[(colon + 1)..], out fileId);
        }

        private static List<long> ReadComponentIds(SceneNode node)
        {
            List<long> fileIds = new List<long>();
            int keyIndex;
            int keyIndent;
            if (!TryFindKeyLine(node, "m_Component", out keyIndex, out keyIndent))
            {
                return fileIds;
            }

            for (int index = keyIndex + 1; index < node.Body.Count; index++)
            {
                string content = SceneYaml.Content(node.Body[index]);
                if (!IsListItem(node.Body[index], keyIndent))
                {
                    break;
                }

                long fileId;
                if (TryParseFileId(content, out fileId))
                {
                    fileIds.Add(fileId);
                }
            }

            return fileIds;
        }

        private static string ReplaceScalar(string rest, string value)
        {
            if (rest.Trim().Length == 0)
            {
                return " " + value;
            }

            int spaces = SceneYaml.CountLeadingSpaces(rest);
            return rest[..spaces] + value;
        }

        private static string? ReplaceFlowMember(string rest, string memberKey, string value, out string previous)
        {
            previous = string.Empty;
            int start = rest.IndexOf('{', StringComparison.Ordinal);
            if (start < 0)
            {
                return null;
            }

            int end = rest.LastIndexOf('}');
            if (end < start)
            {
                return null;
            }

            string? replaced = ReplaceFlowMemberInMapping(rest[start..(end + 1)], memberKey, value, out previous);
            if (replaced == null)
            {
                return null;
            }

            return rest[..start] + replaced + rest[(end + 1)..];
        }

        private static string? ReplaceFlowMemberInMapping(string mapping, string memberKey, string value, out string previous)
        {
            previous = string.Empty;
            int index = 1;
            int depth = 0;
            while (index < mapping.Length)
            {
                while (index < mapping.Length && char.IsWhiteSpace(mapping[index]))
                {
                    index++;
                }

                if (index >= mapping.Length || mapping[index] == '}')
                {
                    return null;
                }

                int memberStart = index;
                while (index < mapping.Length)
                {
                    char current = mapping[index];
                    if (current == '{' || current == '[')
                    {
                        depth++;
                    }
                    else if (current == '}' || current == ']')
                    {
                        if (depth == 0)
                        {
                            break;
                        }

                        depth--;
                    }
                    else if (current == ',' && depth == 0)
                    {
                        break;
                    }

                    index++;
                }

                int memberEnd = index;
                string member = mapping[memberStart..memberEnd];
                int colon = member.IndexOf(':', StringComparison.Ordinal);
                if (colon >= 0)
                {
                    string name = member[..colon].Trim();
                    if (string.Equals(name, memberKey, StringComparison.Ordinal))
                    {
                        string keyPart = member[..(colon + 1)];
                        string valuePart = member[(colon + 1)..];
                        previous = valuePart.Trim();
                        int spaces = SceneYaml.CountLeadingSpaces(valuePart);
                        string replaced = previous.Length == 0
                            ? keyPart + " " + value
                            : keyPart + valuePart[..spaces] + value;
                        return mapping[..memberStart] + replaced + mapping[memberEnd..];
                    }
                }

                if (index < mapping.Length && mapping[index] == ',')
                {
                    index++;
                }
            }

            return null;
        }

        private static string YamlScalar(string value)
        {
            if (value.Length == 0)
            {
                return "\"\"";
            }

            bool plain = !value.Contains(':', StringComparison.Ordinal)
                && !value.Contains('#', StringComparison.Ordinal)
                && !value.Contains('"', StringComparison.Ordinal)
                && !value.Contains('\'', StringComparison.Ordinal)
                && "-[]{}*&!%@`,".IndexOf(value[0], StringComparison.Ordinal) < 0
                && value[0] != ' '
                && value[value.Length - 1] != ' ';
            if (plain)
            {
                return value;
            }

            return "\"" + value.Replace("\\", "\\\\", StringComparison.Ordinal).Replace("\"", "\\\"", StringComparison.Ordinal) + "\"";
        }

        private SceneNode? FindNode(long fileId)
        {
            foreach (SceneNode node in documents)
            {
                if (node.FileId == fileId)
                {
                    return node;
                }
            }

            return null;
        }

        private SceneNode? FindNode(long fileId, int classId)
        {
            foreach (SceneNode node in documents)
            {
                if (node.ClassId == classId && node.FileId == fileId)
                {
                    return node;
                }
            }

            return null;
        }

        private SceneNode? FindNodeByClass(int classId)
        {
            foreach (SceneNode node in documents)
            {
                if (node.ClassId == classId)
                {
                    return node;
                }
            }

            return null;
        }

        private SceneNode? FindTransformForGameObject(long gameObjectFileId)
        {
            foreach (SceneNode node in documents)
            {
                if (node.ClassId != TransformClassId)
                {
                    continue;
                }

                long owner;
                if (TryGetFileId(node, "m_GameObject", out owner) && owner == gameObjectFileId)
                {
                    return node;
                }
            }

            return null;
        }

        private bool IsComponentShared(long componentFileId, long gameObjectFileId)
        {
            foreach (SceneNode node in documents)
            {
                if (node.ClassId != GameObjectClassId || node.FileId == gameObjectFileId)
                {
                    continue;
                }

                foreach (long listedFileId in ReadComponentIds(node))
                {
                    if (listedFileId == componentFileId)
                    {
                        return true;
                    }
                }
            }

            return false;
        }

        private void RemoveNode(long fileId)
        {
            for (int index = 0; index < documents.Count; index++)
            {
                if (documents[index].FileId != fileId)
                {
                    continue;
                }

                documents.RemoveAt(index);
                return;
            }
        }

        private long MaxFileId()
        {
            long max = 0;
            foreach (SceneNode node in documents)
            {
                if (node.FileId > max)
                {
                    max = node.FileId;
                }
            }

            return max;
        }

        private long NextFreeFileId(long after)
        {
            long candidate = after;
            for (int attempt = 0; attempt < 10000; attempt++)
            {
                candidate = candidate == long.MaxValue ? 1 : candidate + 1;
                if (!IsFileIdUsed(candidate))
                {
                    return candidate;
                }
            }

            throw new SceneParseException(path, 0, "no free fileID available");
        }

        private bool IsFileIdUsed(long fileId)
        {
            foreach (SceneNode node in documents)
            {
                if (node.FileId == fileId)
                {
                    return true;
                }
            }

            return false;
        }

        private static string DetectLineEnding(List<string> lines)
        {
            foreach (string line in lines)
            {
                if (line.EndsWith("\r\n", StringComparison.Ordinal))
                {
                    return "\r\n";
                }
            }

            return "\n";
        }

        private static string Format(long fileId)
        {
            return fileId.ToString(CultureInfo.InvariantCulture);
        }

        private sealed class SceneNode
        {
            public string Header { get; set; } = string.Empty;

            public int ClassId { get; set; }

            public long FileId { get; set; }

            public bool Stripped { get; set; }

            public List<string> Body { get; } = new List<string>();
        }
    }
}
