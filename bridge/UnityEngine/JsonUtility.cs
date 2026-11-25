using System;
using System.Collections.Generic;
using System.Globalization;
using System.Text;

namespace UnityEngine
{
    [AttributeUsage(AttributeTargets.Field, AllowMultiple = false, Inherited = true)]
    public sealed class SerializeField : Attribute
    {
    }

    [AttributeUsage(AttributeTargets.Field, AllowMultiple = false, Inherited = true)]
    public sealed class SerializeReference : Attribute
    {
    }

    [AttributeUsage(AttributeTargets.Field, AllowMultiple = false, Inherited = true)]
    public sealed class HideInInspector : Attribute
    {
    }

    public interface ISerializationCallbackReceiver
    {
        void OnBeforeSerialize();

        void OnAfterDeserialize();
    }

    public delegate void JsonFieldWriter<T>(T value, JsonSerializeWriter writer);

    public delegate void JsonFieldReader<T>(ref T target, JsonParseNode node);

    public sealed class JsonSerializeWriter
    {
        private readonly StringBuilder _builder;
        private readonly bool _prettyPrint;
        private readonly int _indentDepth;
        private bool _hasField;

        public JsonSerializeWriter(StringBuilder builder, bool prettyPrint, int indentDepth = 0)
        {
            _builder = builder;
            _prettyPrint = prettyPrint;
            _indentDepth = indentDepth;
        }

        internal void BeginObject()
        {
            _builder.Append('{');
            _hasField = false;
        }

        internal void EndObject()
        {
            if (_prettyPrint && _hasField)
            {
                _builder.Append('\n');
                AppendIndent(_indentDepth);
            }

            _builder.Append('}');
        }

        public void WriteString(string name, string? value)
        {
            WriteFieldPrefix(name);
            AppendEscapedString(value ?? string.Empty);
        }

        public void WriteInt(string name, int value)
        {
            WriteFieldPrefix(name);
            _builder.Append(value.ToString(CultureInfo.InvariantCulture));
        }

        public void WriteLong(string name, long value)
        {
            WriteFieldPrefix(name);
            _builder.Append(value.ToString(CultureInfo.InvariantCulture));
        }

        public void WriteFloat(string name, float value)
        {
            WriteFieldPrefix(name);
            if (!float.IsFinite(value))
            {
                _builder.Append("0.0");
                return;
            }

            string text = value.ToString("R", CultureInfo.InvariantCulture);
            _builder.Append(text);
        }

        public void WriteDouble(string name, double value)
        {
            WriteFieldPrefix(name);
            if (!double.IsFinite(value))
            {
                _builder.Append("0.0");
                return;
            }

            string text = value.ToString("R", CultureInfo.InvariantCulture);
            _builder.Append(text);
        }

        public void WriteBool(string name, bool value)
        {
            WriteFieldPrefix(name);
            _builder.Append(value ? "true" : "false");
        }

        public void WriteVector2(string name, Vector2 value)
        {
            WriteFieldPrefix(name);
            JsonSerializeWriter sub = new JsonSerializeWriter(_builder, _prettyPrint, _indentDepth + 1);
            sub.BeginObject();
            sub.WriteFloat("x", value.x);
            sub.WriteFloat("y", value.y);
            sub.EndObject();
        }

        public void WriteVector3(string name, Vector3 value)
        {
            WriteFieldPrefix(name);
            JsonSerializeWriter sub = new JsonSerializeWriter(_builder, _prettyPrint, _indentDepth + 1);
            sub.BeginObject();
            sub.WriteFloat("x", value.x);
            sub.WriteFloat("y", value.y);
            sub.WriteFloat("z", value.z);
            sub.EndObject();
        }

        public void WriteVector4(string name, Vector4 value)
        {
            WriteFieldPrefix(name);
            JsonSerializeWriter sub = new JsonSerializeWriter(_builder, _prettyPrint, _indentDepth + 1);
            sub.BeginObject();
            sub.WriteFloat("x", value.x);
            sub.WriteFloat("y", value.y);
            sub.WriteFloat("z", value.z);
            sub.WriteFloat("w", value.w);
            sub.EndObject();
        }

        public void WriteQuaternion(string name, Quaternion value)
        {
            WriteFieldPrefix(name);
            JsonSerializeWriter sub = new JsonSerializeWriter(_builder, _prettyPrint, _indentDepth + 1);
            sub.BeginObject();
            sub.WriteFloat("x", value.x);
            sub.WriteFloat("y", value.y);
            sub.WriteFloat("z", value.z);
            sub.WriteFloat("w", value.w);
            sub.EndObject();
        }

        public void WriteRect(string name, Rect value)
        {
            WriteFieldPrefix(name);
            JsonSerializeWriter sub = new JsonSerializeWriter(_builder, _prettyPrint, _indentDepth + 1);
            sub.BeginObject();
            sub.WriteFloat("x", value.x);
            sub.WriteFloat("y", value.y);
            sub.WriteFloat("width", value.width);
            sub.WriteFloat("height", value.height);
            sub.EndObject();
        }

        public void WriteBounds(string name, Bounds value)
        {
            WriteFieldPrefix(name);
            JsonSerializeWriter sub = new JsonSerializeWriter(_builder, _prettyPrint, _indentDepth + 1);
            sub.BeginObject();
            sub.WriteVector3("m_Center", value.center);
            sub.WriteVector3("m_Extent", value.extents);
            sub.EndObject();
        }

        public void WriteNested<TNested>(string name, TNested value)
        {
            WriteFieldPrefix(name);
            JsonUtility.WriteInlineObject(value, _builder, _prettyPrint, _indentDepth + 1);
        }

        public void WriteStringList(string name, IReadOnlyList<string>? values)
        {
            WriteFieldPrefix(name);
            if (values == null || values.Count == 0)
            {
                _builder.Append("[]");
                return;
            }

            _builder.Append('[');
            for (int i = 0; i < values.Count; i++)
            {
                if (i > 0)
                {
                    _builder.Append(',');
                }

                if (_prettyPrint)
                {
                    _builder.Append('\n');
                    AppendIndent(_indentDepth + 2);
                }

                AppendEscapedString(values[i] ?? string.Empty);
            }

            if (_prettyPrint)
            {
                _builder.Append('\n');
                AppendIndent(_indentDepth + 1);
            }

            _builder.Append(']');
        }

        public void WriteIntList(string name, IReadOnlyList<int>? values)
        {
            WriteFieldPrefix(name);
            if (values == null || values.Count == 0)
            {
                _builder.Append("[]");
                return;
            }

            _builder.Append('[');
            for (int i = 0; i < values.Count; i++)
            {
                if (i > 0)
                {
                    _builder.Append(',');
                }

                if (_prettyPrint)
                {
                    _builder.Append('\n');
                    AppendIndent(_indentDepth + 2);
                }

                _builder.Append(values[i].ToString(CultureInfo.InvariantCulture));
            }

            if (_prettyPrint)
            {
                _builder.Append('\n');
                AppendIndent(_indentDepth + 1);
            }

            _builder.Append(']');
        }

        public void WriteFloatList(string name, IReadOnlyList<float>? values)
        {
            WriteFieldPrefix(name);
            if (values == null || values.Count == 0)
            {
                _builder.Append("[]");
                return;
            }

            _builder.Append('[');
            for (int i = 0; i < values.Count; i++)
            {
                if (i > 0)
                {
                    _builder.Append(',');
                }

                if (_prettyPrint)
                {
                    _builder.Append('\n');
                    AppendIndent(_indentDepth + 2);
                }

                float v = values[i];
                _builder.Append(float.IsFinite(v) ? v.ToString("R", CultureInfo.InvariantCulture) : "0.0");
            }

            if (_prettyPrint)
            {
                _builder.Append('\n');
                AppendIndent(_indentDepth + 1);
            }

            _builder.Append(']');
        }

        public void WriteBoolList(string name, IReadOnlyList<bool>? values)
        {
            WriteFieldPrefix(name);
            if (values == null || values.Count == 0)
            {
                _builder.Append("[]");
                return;
            }

            _builder.Append('[');
            for (int i = 0; i < values.Count; i++)
            {
                if (i > 0)
                {
                    _builder.Append(',');
                }

                if (_prettyPrint)
                {
                    _builder.Append('\n');
                    AppendIndent(_indentDepth + 2);
                }

                _builder.Append(values[i] ? "true" : "false");
            }

            if (_prettyPrint)
            {
                _builder.Append('\n');
                AppendIndent(_indentDepth + 1);
            }

            _builder.Append(']');
        }

        public void WriteNestedList<TElement>(string name, IReadOnlyList<TElement>? values)
        {
            WriteFieldPrefix(name);
            if (values == null || values.Count == 0)
            {
                _builder.Append("[]");
                return;
            }

            _builder.Append('[');
            for (int i = 0; i < values.Count; i++)
            {
                if (i > 0)
                {
                    _builder.Append(',');
                }

                if (_prettyPrint)
                {
                    _builder.Append('\n');
                    AppendIndent(_indentDepth + 2);
                }

                JsonUtility.WriteInlineObject(values[i], _builder, _prettyPrint, _indentDepth + 2);
            }

            if (_prettyPrint)
            {
                _builder.Append('\n');
                AppendIndent(_indentDepth + 1);
            }

            _builder.Append(']');
        }

        private void WriteFieldPrefix(string name)
        {
            if (_hasField)
            {
                _builder.Append(',');
            }

            _hasField = true;
            if (_prettyPrint)
            {
                _builder.Append('\n');
                AppendIndent(_indentDepth + 1);
            }

            AppendEscapedString(name);
            _builder.Append(_prettyPrint ? ": " : ":");
        }

        private void AppendIndent(int depth)
        {
            for (int i = 0; i < depth; i++)
            {
                _builder.Append("    ");
            }
        }

        private void AppendEscapedString(string text)
        {
            _builder.Append('"');
            for (int i = 0; i < text.Length; i++)
            {
                char ch = text[i];
                switch (ch)
                {
                    case '"':
                        _builder.Append("\\\"");
                        break;
                    case '\\':
                        _builder.Append("\\\\");
                        break;
                    case '\b':
                        _builder.Append("\\b");
                        break;
                    case '\f':
                        _builder.Append("\\f");
                        break;
                    case '\n':
                        _builder.Append("\\n");
                        break;
                    case '\r':
                        _builder.Append("\\r");
                        break;
                    case '\t':
                        _builder.Append("\\t");
                        break;
                    default:
                        if (ch < 0x20)
                        {
                            _builder.Append("\\u");
                            _builder.Append(((int)ch).ToString("x4", CultureInfo.InvariantCulture));
                        }
                        else
                        {
                            _builder.Append(ch);
                        }

                        break;
                }
            }

            _builder.Append('"');
        }
    }

    public enum JsonNodeKind
    {
        Null = 0,
        Bool = 1,
        Number = 2,
        String = 3,
        Array = 4,
        Object = 5
    }

    public sealed class JsonParseNode
    {
        private readonly Dictionary<string, JsonParseNode>? _fields;
        private readonly List<JsonParseNode>? _items;

        private JsonParseNode(
            JsonNodeKind kind,
            bool boolValue,
            double numberValue,
            string stringValue,
            List<JsonParseNode>? items,
            Dictionary<string, JsonParseNode>? fields)
        {
            Kind = kind;
            BoolValue = boolValue;
            NumberValue = numberValue;
            StringValue = stringValue;
            _items = items;
            _fields = fields;
        }

        public JsonNodeKind Kind { get; }

        public bool BoolValue { get; }

        public double NumberValue { get; }

        public string StringValue { get; }

        public static JsonParseNode CreateNull() =>
            new JsonParseNode(JsonNodeKind.Null, false, 0.0, string.Empty, null, null);

        public static JsonParseNode CreateBool(bool value) =>
            new JsonParseNode(JsonNodeKind.Bool, value, value ? 1.0 : 0.0, string.Empty, null, null);

        public static JsonParseNode CreateNumber(double value, string rawText) =>
            new JsonParseNode(JsonNodeKind.Number, value != 0.0, value, rawText, null, null);

        public static JsonParseNode CreateString(string value) =>
            new JsonParseNode(JsonNodeKind.String, false, 0.0, value, null, null);

        public static JsonParseNode CreateArray(List<JsonParseNode> items) =>
            new JsonParseNode(JsonNodeKind.Array, false, 0.0, string.Empty, items, null);

        public static JsonParseNode CreateObject(Dictionary<string, JsonParseNode> fields) =>
            new JsonParseNode(JsonNodeKind.Object, false, 0.0, string.Empty, null, fields);

        public string ReadString(string name, string currentValue)
        {
            if (_fields == null || !_fields.TryGetValue(name, out JsonParseNode? child))
            {
                return currentValue;
            }

            if (child.Kind == JsonNodeKind.String)
            {
                return child.StringValue;
            }

            if (child.Kind == JsonNodeKind.Null)
            {
                return string.Empty;
            }

            return currentValue;
        }

        public int ReadInt(string name, int currentValue)
        {
            if (_fields == null || !_fields.TryGetValue(name, out JsonParseNode? child))
            {
                return currentValue;
            }

            if (child.Kind == JsonNodeKind.Number)
            {
                if (!string.IsNullOrEmpty(child.StringValue)
                    && int.TryParse(child.StringValue, NumberStyles.Integer, CultureInfo.InvariantCulture, out int parsed))
                {
                    return parsed;
                }

                return (int)child.NumberValue;
            }

            return currentValue;
        }

        public long ReadLong(string name, long currentValue)
        {
            if (_fields == null || !_fields.TryGetValue(name, out JsonParseNode? child))
            {
                return currentValue;
            }

            if (child.Kind == JsonNodeKind.Number)
            {
                if (!string.IsNullOrEmpty(child.StringValue)
                    && long.TryParse(child.StringValue, NumberStyles.Integer, CultureInfo.InvariantCulture, out long parsed))
                {
                    return parsed;
                }

                return (long)child.NumberValue;
            }

            return currentValue;
        }

        public float ReadFloat(string name, float currentValue)
        {
            if (_fields == null || !_fields.TryGetValue(name, out JsonParseNode? child))
            {
                return currentValue;
            }

            if (child.Kind == JsonNodeKind.Number)
            {
                return (float)child.NumberValue;
            }

            return currentValue;
        }

        public double ReadDouble(string name, double currentValue)
        {
            if (_fields == null || !_fields.TryGetValue(name, out JsonParseNode? child))
            {
                return currentValue;
            }

            if (child.Kind == JsonNodeKind.Number)
            {
                return child.NumberValue;
            }

            return currentValue;
        }

        public bool ReadBool(string name, bool currentValue)
        {
            if (_fields == null || !_fields.TryGetValue(name, out JsonParseNode? child))
            {
                return currentValue;
            }

            if (child.Kind == JsonNodeKind.Bool)
            {
                return child.BoolValue;
            }

            return currentValue;
        }

        public Vector2 ReadVector2(string name, Vector2 currentValue)
        {
            if (_fields == null || !_fields.TryGetValue(name, out JsonParseNode? child) || child.Kind != JsonNodeKind.Object)
            {
                return currentValue;
            }

            return new Vector2(
                child.ReadFloat("x", currentValue.x),
                child.ReadFloat("y", currentValue.y));
        }

        public Vector3 ReadVector3(string name, Vector3 currentValue)
        {
            if (_fields == null || !_fields.TryGetValue(name, out JsonParseNode? child) || child.Kind != JsonNodeKind.Object)
            {
                return currentValue;
            }

            return new Vector3(
                child.ReadFloat("x", currentValue.x),
                child.ReadFloat("y", currentValue.y),
                child.ReadFloat("z", currentValue.z));
        }

        public Vector4 ReadVector4(string name, Vector4 currentValue)
        {
            if (_fields == null || !_fields.TryGetValue(name, out JsonParseNode? child) || child.Kind != JsonNodeKind.Object)
            {
                return currentValue;
            }

            return new Vector4(
                child.ReadFloat("x", currentValue.x),
                child.ReadFloat("y", currentValue.y),
                child.ReadFloat("z", currentValue.z),
                child.ReadFloat("w", currentValue.w));
        }

        public Quaternion ReadQuaternion(string name, Quaternion currentValue)
        {
            if (_fields == null || !_fields.TryGetValue(name, out JsonParseNode? child) || child.Kind != JsonNodeKind.Object)
            {
                return currentValue;
            }

            return new Quaternion(
                child.ReadFloat("x", currentValue.x),
                child.ReadFloat("y", currentValue.y),
                child.ReadFloat("z", currentValue.z),
                child.ReadFloat("w", currentValue.w));
        }

        public Rect ReadRect(string name, Rect currentValue)
        {
            if (_fields == null || !_fields.TryGetValue(name, out JsonParseNode? child) || child.Kind != JsonNodeKind.Object)
            {
                return currentValue;
            }

            return new Rect(
                child.ReadFloat("x", currentValue.x),
                child.ReadFloat("y", currentValue.y),
                child.ReadFloat("width", currentValue.width),
                child.ReadFloat("height", currentValue.height));
        }

        public Bounds ReadBounds(string name, Bounds currentValue)
        {
            if (_fields == null || !_fields.TryGetValue(name, out JsonParseNode? child) || child.Kind != JsonNodeKind.Object)
            {
                return currentValue;
            }

            Vector3 center = child.ReadVector3("m_Center", currentValue.center);
            Vector3 extents = child.ReadVector3("m_Extent", currentValue.extents);
            Bounds b = new Bounds(center, extents * 2f);
            return b;
        }

        public TNested ReadNested<TNested>(string name, TNested currentValue) where TNested : new()
        {
            if (_fields == null || !_fields.TryGetValue(name, out JsonParseNode? child))
            {
                return currentValue;
            }

            if (child.Kind == JsonNodeKind.Null)
            {
                return new TNested();
            }

            if (child.Kind != JsonNodeKind.Object)
            {
                return currentValue;
            }

            TNested target = currentValue is null ? new TNested() : currentValue;
            JsonUtility.ReadInlineObject(ref target, child);
            return target;
        }

        public string[] ReadStringArray(string name, string[]? currentValue)
        {
            List<string>? list = ReadStringListInternal(name);
            return list != null ? list.ToArray() : (currentValue ?? Array.Empty<string>());
        }

        public List<string> ReadStringList(string name, List<string>? currentValue)
        {
            List<string>? list = ReadStringListInternal(name);
            return list ?? (currentValue ?? new List<string>());
        }

        public int[] ReadIntArray(string name, int[]? currentValue)
        {
            List<int>? list = ReadIntListInternal(name);
            return list != null ? list.ToArray() : (currentValue ?? Array.Empty<int>());
        }

        public List<int> ReadIntList(string name, List<int>? currentValue)
        {
            List<int>? list = ReadIntListInternal(name);
            return list ?? (currentValue ?? new List<int>());
        }

        public float[] ReadFloatArray(string name, float[]? currentValue)
        {
            List<float>? list = ReadFloatListInternal(name);
            return list != null ? list.ToArray() : (currentValue ?? Array.Empty<float>());
        }

        public List<float> ReadFloatList(string name, List<float>? currentValue)
        {
            List<float>? list = ReadFloatListInternal(name);
            return list ?? (currentValue ?? new List<float>());
        }

        public bool[] ReadBoolArray(string name, bool[]? currentValue)
        {
            List<bool>? list = ReadBoolListInternal(name);
            return list != null ? list.ToArray() : (currentValue ?? Array.Empty<bool>());
        }

        public List<bool> ReadBoolList(string name, List<bool>? currentValue)
        {
            List<bool>? list = ReadBoolListInternal(name);
            return list ?? (currentValue ?? new List<bool>());
        }

        public TElement[] ReadNestedArray<TElement>(string name, TElement[]? currentValue) where TElement : new()
        {
            List<TElement>? list = ReadNestedListInternal<TElement>(name);
            return list != null ? list.ToArray() : (currentValue ?? Array.Empty<TElement>());
        }

        public List<TElement> ReadNestedList<TElement>(string name, List<TElement>? currentValue) where TElement : new()
        {
            List<TElement>? list = ReadNestedListInternal<TElement>(name);
            return list ?? (currentValue ?? new List<TElement>());
        }

        private List<string>? ReadStringListInternal(string name)
        {
            if (_fields == null || !_fields.TryGetValue(name, out JsonParseNode? child) || child.Kind != JsonNodeKind.Array || child._items == null)
            {
                return null;
            }

            List<string> result = new List<string>(child._items.Count);
            for (int i = 0; i < child._items.Count; i++)
            {
                JsonParseNode item = child._items[i];
                result.Add(item.Kind == JsonNodeKind.String ? item.StringValue : string.Empty);
            }

            return result;
        }

        private List<int>? ReadIntListInternal(string name)
        {
            if (_fields == null || !_fields.TryGetValue(name, out JsonParseNode? child) || child.Kind != JsonNodeKind.Array || child._items == null)
            {
                return null;
            }

            List<int> result = new List<int>(child._items.Count);
            for (int i = 0; i < child._items.Count; i++)
            {
                JsonParseNode item = child._items[i];
                if (item.Kind == JsonNodeKind.Number)
                {
                    if (!string.IsNullOrEmpty(item.StringValue)
                        && int.TryParse(item.StringValue, NumberStyles.Integer, CultureInfo.InvariantCulture, out int parsed))
                    {
                        result.Add(parsed);
                    }
                    else
                    {
                        result.Add((int)item.NumberValue);
                    }
                }
                else
                {
                    result.Add(0);
                }
            }

            return result;
        }

        private List<float>? ReadFloatListInternal(string name)
        {
            if (_fields == null || !_fields.TryGetValue(name, out JsonParseNode? child) || child.Kind != JsonNodeKind.Array || child._items == null)
            {
                return null;
            }

            List<float> result = new List<float>(child._items.Count);
            for (int i = 0; i < child._items.Count; i++)
            {
                JsonParseNode item = child._items[i];
                result.Add(item.Kind == JsonNodeKind.Number ? (float)item.NumberValue : 0f);
            }

            return result;
        }

        private List<bool>? ReadBoolListInternal(string name)
        {
            if (_fields == null || !_fields.TryGetValue(name, out JsonParseNode? child) || child.Kind != JsonNodeKind.Array || child._items == null)
            {
                return null;
            }

            List<bool> result = new List<bool>(child._items.Count);
            for (int i = 0; i < child._items.Count; i++)
            {
                JsonParseNode item = child._items[i];
                result.Add(item.Kind == JsonNodeKind.Bool && item.BoolValue);
            }

            return result;
        }

        private List<TElement>? ReadNestedListInternal<TElement>(string name) where TElement : new()
        {
            if (_fields == null || !_fields.TryGetValue(name, out JsonParseNode? child) || child.Kind != JsonNodeKind.Array || child._items == null)
            {
                return null;
            }

            List<TElement> result = new List<TElement>(child._items.Count);
            for (int i = 0; i < child._items.Count; i++)
            {
                JsonParseNode item = child._items[i];
                TElement element = new TElement();
                if (item.Kind == JsonNodeKind.Object)
                {
                    JsonUtility.ReadInlineObject(ref element, item);
                }

                result.Add(element);
            }

            return result;
        }
    }

    public static class JsonUtility
    {
        private static class TypeStorage<T>
        {
            public static Func<T>? Factory;
            public static JsonFieldWriter<T>? Writer;
            public static JsonFieldReader<T>? Reader;
            public static bool IsRegistered;
        }

        static JsonUtility()
        {
            RegisterBuiltInTypes();
        }

        public static void RegisterType<T>(
            Func<T> factory,
            JsonFieldWriter<T> writer,
            JsonFieldReader<T> reader)
        {
            if (factory == null)
            {
                throw new ArgumentNullException(nameof(factory));
            }

            if (writer == null)
            {
                throw new ArgumentNullException(nameof(writer));
            }

            if (reader == null)
            {
                throw new ArgumentNullException(nameof(reader));
            }

            TypeStorage<T>.Factory = factory;
            TypeStorage<T>.Writer = writer;
            TypeStorage<T>.Reader = reader;
            TypeStorage<T>.IsRegistered = true;
        }

        public static bool IsTypeRegistered<T>() => TypeStorage<T>.IsRegistered;

        public static string ToJson<T>(T obj) => ToJson(obj, false);

        public static string ToJson<T>(T obj, bool prettyPrint)
        {
            if (obj is null)
            {
                return string.Empty;
            }

            Type targetType = typeof(T);
            if (targetType.IsArray || targetType == typeof(string) || targetType.IsPrimitive)
            {
                throw new ArgumentException("ArgumentException: Cannot serialize type '" + targetType.FullName + "' as a top-level JSON object.");
            }

            if (!TypeStorage<T>.IsRegistered || TypeStorage<T>.Writer == null)
            {
                string message = "JsonUtility: Type '" + (targetType.FullName ?? targetType.Name) + "' is not registered for source-generated JSON serialization.";
                Debug.LogError(message);
                throw new InvalidOperationException(message);
            }

            if (obj is ISerializationCallbackReceiver callbackReceiver)
            {
                callbackReceiver.OnBeforeSerialize();
            }

            StringBuilder sb = new StringBuilder();
            JsonSerializeWriter writer = new JsonSerializeWriter(sb, prettyPrint, 0);
            writer.BeginObject();
            TypeStorage<T>.Writer(obj, writer);
            writer.EndObject();
            return sb.ToString();
        }

        public static T? FromJson<T>(string json)
        {
            if (string.IsNullOrEmpty(json))
            {
                return default;
            }

            Type targetType = typeof(T);
            if (targetType.IsArray || targetType == typeof(string) || targetType.IsPrimitive)
            {
                throw new ArgumentException("ArgumentException: Cannot deserialize type '" + targetType.FullName + "' as a top-level JSON object.");
            }

            if (!TypeStorage<T>.IsRegistered || TypeStorage<T>.Factory == null || TypeStorage<T>.Reader == null)
            {
                string message = "JsonUtility: Type '" + (targetType.FullName ?? targetType.Name) + "' is not registered for source-generated JSON serialization.";
                Debug.LogError(message);
                throw new InvalidOperationException(message);
            }

            JsonParseNode root = JsonDomParser.Parse(json);
            if (root.Kind == JsonNodeKind.Null)
            {
                return default;
            }

            if (root.Kind != JsonNodeKind.Object)
            {
                throw new ArgumentException("JSON must represent an object type at position 0.");
            }

            T instance = TypeStorage<T>.Factory();
            TypeStorage<T>.Reader(ref instance, root);

            if (instance is ISerializationCallbackReceiver callbackReceiver)
            {
                callbackReceiver.OnAfterDeserialize();
            }

            return instance;
        }

        public static void FromJsonOverwrite<T>(string json, ref T objectToOverwrite)
        {
            if (objectToOverwrite is null)
            {
                throw new ArgumentNullException(nameof(objectToOverwrite));
            }

            if (string.IsNullOrEmpty(json))
            {
                return;
            }

            Type targetType = typeof(T);
            if (!TypeStorage<T>.IsRegistered || TypeStorage<T>.Reader == null)
            {
                string message = "JsonUtility: Type '" + (targetType.FullName ?? targetType.Name) + "' is not registered for source-generated JSON serialization.";
                Debug.LogError(message);
                throw new InvalidOperationException(message);
            }

            JsonParseNode root = JsonDomParser.Parse(json);
            if (root.Kind == JsonNodeKind.Null)
            {
                return;
            }

            if (root.Kind != JsonNodeKind.Object)
            {
                throw new ArgumentException("JSON must represent an object type at position 0.");
            }

            TypeStorage<T>.Reader(ref objectToOverwrite, root);

            if (objectToOverwrite is ISerializationCallbackReceiver callbackReceiver)
            {
                callbackReceiver.OnAfterDeserialize();
            }
        }

        internal static void WriteInlineObject<TNested>(TNested value, StringBuilder builder, bool prettyPrint, int indentDepth)
        {
            if (!TypeStorage<TNested>.IsRegistered || TypeStorage<TNested>.Writer == null)
            {
                Type targetType = typeof(TNested);
                string message = "JsonUtility: Nested type '" + (targetType.FullName ?? targetType.Name) + "' is not registered for source-generated JSON serialization.";
                Debug.LogError(message);
                throw new InvalidOperationException(message);
            }

            TNested effective = value;
            if (effective is null)
            {
                if (TypeStorage<TNested>.Factory != null)
                {
                    effective = TypeStorage<TNested>.Factory();
                }
                else
                {
                    builder.Append("{}");
                    return;
                }
            }

            if (effective is ISerializationCallbackReceiver callbackReceiver)
            {
                callbackReceiver.OnBeforeSerialize();
            }

            JsonSerializeWriter sub = new JsonSerializeWriter(builder, prettyPrint, indentDepth);
            sub.BeginObject();
            TypeStorage<TNested>.Writer(effective, sub);
            sub.EndObject();
        }

        internal static void ReadInlineObject<TNested>(ref TNested target, JsonParseNode node)
        {
            if (!TypeStorage<TNested>.IsRegistered || TypeStorage<TNested>.Reader == null)
            {
                Type targetType = typeof(TNested);
                string message = "JsonUtility: Nested type '" + (targetType.FullName ?? targetType.Name) + "' is not registered for source-generated JSON serialization.";
                Debug.LogError(message);
                throw new InvalidOperationException(message);
            }

            TypeStorage<TNested>.Reader(ref target, node);
            if (target is ISerializationCallbackReceiver callbackReceiver)
            {
                callbackReceiver.OnAfterDeserialize();
            }
        }

        private static void RegisterBuiltInTypes()
        {
            RegisterType<Vector2>(
                () => Vector2.zero,
                (val, w) =>
                {
                    w.WriteFloat("x", val.x);
                    w.WriteFloat("y", val.y);
                },
                (ref Vector2 target, JsonParseNode n) =>
                {
                    target.x = n.ReadFloat("x", target.x);
                    target.y = n.ReadFloat("y", target.y);
                });

            RegisterType<Vector3>(
                () => Vector3.zero,
                (val, w) =>
                {
                    w.WriteFloat("x", val.x);
                    w.WriteFloat("y", val.y);
                    w.WriteFloat("z", val.z);
                },
                (ref Vector3 target, JsonParseNode n) =>
                {
                    target.x = n.ReadFloat("x", target.x);
                    target.y = n.ReadFloat("y", target.y);
                    target.z = n.ReadFloat("z", target.z);
                });

            RegisterType<Vector4>(
                () => Vector4.zero,
                (val, w) =>
                {
                    w.WriteFloat("x", val.x);
                    w.WriteFloat("y", val.y);
                    w.WriteFloat("z", val.z);
                    w.WriteFloat("w", val.w);
                },
                (ref Vector4 target, JsonParseNode n) =>
                {
                    target.x = n.ReadFloat("x", target.x);
                    target.y = n.ReadFloat("y", target.y);
                    target.z = n.ReadFloat("z", target.z);
                    target.w = n.ReadFloat("w", target.w);
                });

            RegisterType<Quaternion>(
                () => new Quaternion(0f, 0f, 0f, 0f),
                (val, w) =>
                {
                    w.WriteFloat("x", val.x);
                    w.WriteFloat("y", val.y);
                    w.WriteFloat("z", val.z);
                    w.WriteFloat("w", val.w);
                },
                (ref Quaternion target, JsonParseNode n) =>
                {
                    target.x = n.ReadFloat("x", target.x);
                    target.y = n.ReadFloat("y", target.y);
                    target.z = n.ReadFloat("z", target.z);
                    target.w = n.ReadFloat("w", target.w);
                });

            RegisterType<Rect>(
                () => Rect.zero,
                (val, w) =>
                {
                    w.WriteFloat("x", val.x);
                    w.WriteFloat("y", val.y);
                    w.WriteFloat("width", val.width);
                    w.WriteFloat("height", val.height);
                },
                (ref Rect target, JsonParseNode n) =>
                {
                    target.x = n.ReadFloat("x", target.x);
                    target.y = n.ReadFloat("y", target.y);
                    target.width = n.ReadFloat("width", target.width);
                    target.height = n.ReadFloat("height", target.height);
                });

            RegisterType<Bounds>(
                () => new Bounds(Vector3.zero, Vector3.zero),
                (val, w) =>
                {
                    w.WriteVector3("m_Center", val.center);
                    w.WriteVector3("m_Extent", val.extents);
                },
                (ref Bounds target, JsonParseNode n) =>
                {
                    target.center = n.ReadVector3("m_Center", target.center);
                    target.extents = n.ReadVector3("m_Extent", target.extents);
                });
        }

        private static class JsonDomParser
        {
            public static JsonParseNode Parse(string json)
            {
                int index = 0;
                SkipWhitespace(json, ref index);
                if (index >= json.Length)
                {
                    throw CreateSyntaxError("Unexpected end of JSON input", index);
                }

                JsonParseNode node = ParseValue(json, ref index);
                SkipWhitespace(json, ref index);
                if (index < json.Length)
                {
                    throw CreateSyntaxError("Unexpected trailing character '" + json[index] + "'", index);
                }

                return node;
            }

            private static JsonParseNode ParseValue(string json, ref int index)
            {
                SkipWhitespace(json, ref index);
                if (index >= json.Length)
                {
                    throw CreateSyntaxError("Unexpected end of JSON value", index);
                }

                char c = json[index];
                if (c == '{')
                {
                    return ParseObject(json, ref index);
                }

                if (c == '[')
                {
                    return ParseArray(json, ref index);
                }

                if (c == '"')
                {
                    return JsonParseNode.CreateString(ParseString(json, ref index));
                }

                if (c == 't' || c == 'f')
                {
                    return ParseBoolean(json, ref index);
                }

                if (c == 'n')
                {
                    return ParseNull(json, ref index);
                }

                if (c == '-' || (c >= '0' && c <= '9'))
                {
                    return ParseNumber(json, ref index);
                }

                throw CreateSyntaxError("Unexpected character '" + c + "'", index);
            }

            private static JsonParseNode ParseObject(string json, ref int index)
            {
                int startPos = index;
                index++;
                SkipWhitespace(json, ref index);
                Dictionary<string, JsonParseNode> fields = new Dictionary<string, JsonParseNode>(StringComparer.Ordinal);
                if (index < json.Length && json[index] == '}')
                {
                    index++;
                    return JsonParseNode.CreateObject(fields);
                }

                while (true)
                {
                    SkipWhitespace(json, ref index);
                    if (index >= json.Length || json[index] != '"')
                    {
                        throw CreateSyntaxError("Expected property name string in JSON object", index);
                    }

                    string key = ParseString(json, ref index);
                    SkipWhitespace(json, ref index);
                    if (index >= json.Length || json[index] != ':')
                    {
                        throw CreateSyntaxError("Expected ':' after property name '" + key + "'", index);
                    }

                    index++;
                    JsonParseNode val = ParseValue(json, ref index);
                    fields[key] = val;

                    SkipWhitespace(json, ref index);
                    if (index >= json.Length)
                    {
                        throw CreateSyntaxError("Unterminated JSON object starting at position " + startPos.ToString(CultureInfo.InvariantCulture), index);
                    }

                    char next = json[index];
                    if (next == '}')
                    {
                        index++;
                        return JsonParseNode.CreateObject(fields);
                    }

                    if (next == ',')
                    {
                        index++;
                        continue;
                    }

                    throw CreateSyntaxError("Expected ',' or '}' in JSON object", index);
                }
            }

            private static JsonParseNode ParseArray(string json, ref int index)
            {
                int startPos = index;
                index++;
                SkipWhitespace(json, ref index);
                List<JsonParseNode> items = new List<JsonParseNode>();
                if (index < json.Length && json[index] == ']')
                {
                    index++;
                    return JsonParseNode.CreateArray(items);
                }

                while (true)
                {
                    JsonParseNode item = ParseValue(json, ref index);
                    items.Add(item);

                    SkipWhitespace(json, ref index);
                    if (index >= json.Length)
                    {
                        throw CreateSyntaxError("Unterminated JSON array starting at position " + startPos.ToString(CultureInfo.InvariantCulture), index);
                    }

                    char next = json[index];
                    if (next == ']')
                    {
                        index++;
                        return JsonParseNode.CreateArray(items);
                    }

                    if (next == ',')
                    {
                        index++;
                        continue;
                    }

                    throw CreateSyntaxError("Expected ',' or ']' in JSON array", index);
                }
            }

            private static string ParseString(string json, ref int index)
            {
                int startPos = index;
                index++;
                StringBuilder sb = new StringBuilder();
                while (index < json.Length)
                {
                    char c = json[index++];
                    if (c == '"')
                    {
                        return sb.ToString();
                    }

                    if (c == '\\')
                    {
                        if (index >= json.Length)
                        {
                            throw CreateSyntaxError("Unterminated escape sequence in string", index);
                        }

                        char esc = json[index++];
                        switch (esc)
                        {
                            case '"':
                                sb.Append('"');
                                break;
                            case '\\':
                                sb.Append('\\');
                                break;
                            case '/':
                                sb.Append('/');
                                break;
                            case 'b':
                                sb.Append('\b');
                                break;
                            case 'f':
                                sb.Append('\f');
                                break;
                            case 'n':
                                sb.Append('\n');
                                break;
                            case 'r':
                                sb.Append('\r');
                                break;
                            case 't':
                                sb.Append('\t');
                                break;
                            case 'u':
                                if (index + 4 > json.Length)
                                {
                                    throw CreateSyntaxError("Incomplete unicode escape in string", index);
                                }

                                string hex = json.Substring(index, 4);
                                if (!int.TryParse(hex, NumberStyles.HexNumber, CultureInfo.InvariantCulture, out int codePoint))
                                {
                                    throw CreateSyntaxError("Invalid unicode escape '\\u" + hex + "'", index);
                                }

                                sb.Append((char)codePoint);
                                index += 4;
                                break;
                            default:
                                throw CreateSyntaxError("Invalid escape character '\\" + esc + "'", index - 1);
                        }
                    }
                    else
                    {
                        sb.Append(c);
                    }
                }

                throw CreateSyntaxError("Unterminated string literal", startPos);
            }

            private static JsonParseNode ParseBoolean(string json, ref int index)
            {
                if (string.CompareOrdinal(json, index, "true", 0, 4) == 0)
                {
                    index += 4;
                    return JsonParseNode.CreateBool(true);
                }

                if (string.CompareOrdinal(json, index, "false", 0, 5) == 0)
                {
                    index += 5;
                    return JsonParseNode.CreateBool(false);
                }

                throw CreateSyntaxError("Invalid boolean literal", index);
            }

            private static JsonParseNode ParseNull(string json, ref int index)
            {
                if (string.CompareOrdinal(json, index, "null", 0, 4) == 0)
                {
                    index += 4;
                    return JsonParseNode.CreateNull();
                }

                throw CreateSyntaxError("Invalid null literal", index);
            }

            private static JsonParseNode ParseNumber(string json, ref int index)
            {
                int start = index;
                if (json[index] == '-')
                {
                    index++;
                }

                bool hasDigits = false;
                while (index < json.Length && json[index] >= '0' && json[index] <= '9')
                {
                    hasDigits = true;
                    index++;
                }

                if (index < json.Length && json[index] == '.')
                {
                    index++;
                    bool hasFrac = false;
                    while (index < json.Length && json[index] >= '0' && json[index] <= '9')
                    {
                        hasFrac = true;
                        index++;
                    }

                    if (!hasFrac)
                    {
                        throw CreateSyntaxError("Malformed number literal", start);
                    }
                }

                if (index < json.Length && (json[index] == 'e' || json[index] == 'E'))
                {
                    index++;
                    if (index < json.Length && (json[index] == '+' || json[index] == '-'))
                    {
                        index++;
                    }

                    bool hasExp = false;
                    while (index < json.Length && json[index] >= '0' && json[index] <= '9')
                    {
                        hasExp = true;
                        index++;
                    }

                    if (!hasExp)
                    {
                        throw CreateSyntaxError("Malformed exponent in number literal", start);
                    }
                }

                if (!hasDigits)
                {
                    throw CreateSyntaxError("Invalid number literal", start);
                }

                string raw = json.Substring(start, index - start);
                if (!double.TryParse(raw, NumberStyles.Float, CultureInfo.InvariantCulture, out double val))
                {
                    throw CreateSyntaxError("Invalid number literal '" + raw + "'", start);
                }

                return JsonParseNode.CreateNumber(val, raw);
            }

            private static void SkipWhitespace(string json, ref int index)
            {
                while (index < json.Length)
                {
                    char c = json[index];
                    if (c == ' ' || c == '\t' || c == '\r' || c == '\n')
                    {
                        index++;
                    }
                    else
                    {
                        break;
                    }
                }
            }

            private static ArgumentException CreateSyntaxError(string detail, int position)
            {
                return new ArgumentException("JSON parse error at position " + position.ToString(CultureInfo.InvariantCulture) + ": " + detail);
            }
        }
    }
}
