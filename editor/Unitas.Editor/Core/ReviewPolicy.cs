using System.Text.Json.Nodes;

namespace Unitas.Editor.Mcp
{
    public sealed class ReviewPolicy
    {
        public ReviewPolicy(bool requireApproval, bool holdAgentWrites)
        {
            RequireApproval = requireApproval;
            HoldAgentWrites = holdAgentWrites;
        }

        public static ReviewPolicy Default => new ReviewPolicy(requireApproval: true, holdAgentWrites: false);

        public bool RequireApproval { get; }

        public bool HoldAgentWrites { get; }

        public JsonObject ToValue()
        {
            return new JsonObject
            {
                ["requireApproval"] = RequireApproval,
                ["holdAgentWrites"] = HoldAgentWrites,
            };
        }

        public static ReviewPolicy FromValue(JsonObject? value)
        {
            if (value == null)
            {
                return Default;
            }

            if (!TryReadBool(value, "requireApproval", true, out bool requireApproval)
                || !TryReadBool(value, "holdAgentWrites", false, out bool holdAgentWrites))
            {
                return Default;
            }

            return new ReviewPolicy(requireApproval, holdAgentWrites);
        }

        private static bool TryReadBool(JsonObject value, string propertyName, bool fallback, out bool result)
        {
            result = fallback;
            JsonNode? node = value[propertyName];
            if (node == null)
            {
                return true;
            }

            if (node is JsonValue jsonValue && jsonValue.TryGetValue(out bool parsed))
            {
                result = parsed;
                return true;
            }

            return false;
        }
    }
}
