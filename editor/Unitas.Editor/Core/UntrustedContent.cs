using System.Text.Json.Nodes;

namespace Unitas.Editor.Mcp
{
    public static class UntrustedContent
    {
        public static JsonObject Mark(string origin, JsonNode? payload)
        {
            return new JsonObject
            {
                ["untrusted"] = true,
                ["origin"] = origin,
                // Deep clone mirrors Rust value semantics: a JsonNode already
                // parented elsewhere cannot be attached directly.
                ["content"] = payload?.DeepClone(),
            };
        }

        public static bool IsEnvelope(JsonNode? node)
        {
            if (!(node is JsonObject map))
            {
                return false;
            }

            if (!(map["untrusted"] is JsonValue flag)
                || !flag.TryGetValue(out bool untrusted)
                || !untrusted)
            {
                return false;
            }

            if (!(map["origin"] is JsonValue originValue)
                || !originValue.TryGetValue(out string? origin)
                || string.IsNullOrEmpty(origin))
            {
                return false;
            }

            return map.ContainsKey("content");
        }

        public static JsonNode Ensure(string defaultOrigin, JsonNode? node)
        {
            if (IsEnvelope(node))
            {
                return node!;
            }

            return Mark(defaultOrigin, node);
        }
    }
}
