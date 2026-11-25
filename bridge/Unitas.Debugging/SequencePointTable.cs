using System;
using System.Collections.Generic;
using System.IO;
using System.Text.Json.Nodes;

namespace Unitas.Debugging
{
    public sealed class SequencePoint
    {
        public SequencePoint(int id, string file, int line, string method, IReadOnlyList<string> localNames)
        {
            Id = id;
            File = file ?? string.Empty;
            Line = line;
            Method = method ?? string.Empty;
            LocalNames = localNames ?? Array.Empty<string>();
        }

        public int Id { get; }
        public string File { get; }
        public int Line { get; }
        public string Method { get; }
        public IReadOnlyList<string> LocalNames { get; }
    }

    public sealed class SequencePointTable
    {
        private readonly Dictionary<int, SequencePoint> _points = new Dictionary<int, SequencePoint>();

        public int Count => _points.Count;

        public void Add(int id, string file, int line, string method)
        {
            Add(id, file, line, method, Array.Empty<string>());
        }

        public void Add(int id, string file, int line, string method, IReadOnlyList<string> localNames)
        {
            _points[id] = new SequencePoint(id, file, line, method, localNames);
        }

        public SequencePoint? Find(int id)
        {
            return _points.TryGetValue(id, out SequencePoint? point) ? point : null;
        }

        public string ToJson()
        {
            JsonArray array = new JsonArray();
            foreach (SequencePoint point in _points.Values)
            {
                JsonArray names = new JsonArray();
                foreach (string name in point.LocalNames)
                {
                    names.Add(name);
                }
                array.Add(new JsonObject
                {
                    ["id"] = point.Id,
                    ["file"] = point.File,
                    ["line"] = point.Line,
                    ["method"] = point.Method,
                    ["localNames"] = names
                });
            }
            return array.ToJsonString();
        }

        public static SequencePointTable Parse(string json)
        {
            SequencePointTable table = new SequencePointTable();
            if (JsonNode.Parse(json) is JsonArray array)
            {
                foreach (JsonNode? node in array)
                {
                    if (node is JsonObject entry)
                    {
                        List<string> names = new List<string>();
                        if (entry["localNames"] is JsonArray nameArray)
                        {
                            foreach (JsonNode? name in nameArray)
                            {
                                names.Add(name?.ToString() ?? string.Empty);
                            }
                        }
                        table.Add(
                            ReadInt(entry["id"]),
                            ReadString(entry["file"]),
                            ReadInt(entry["line"]),
                            ReadString(entry["method"]),
                            names);
                    }
                }
            }
            return table;
        }

        public static SequencePointTable Load(string path)
        {
            return Parse(File.ReadAllText(path));
        }

        private static int ReadInt(JsonNode? node)
        {
            if (node is JsonValue value && value.TryGetValue<int>(out int number))
            {
                return number;
            }
            return 0;
        }

        private static string ReadString(JsonNode? node)
        {
            if (node is JsonValue value && value.TryGetValue<string>(out string? text))
            {
                return text;
            }
            return string.Empty;
        }
    }
}
