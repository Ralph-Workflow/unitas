using System.Text.Json.Nodes;
using Unitas.Editor.Mcp;

namespace Unitas.Editor.Tests.Core
{
    public static class UntrustedContentTests
    {
        public static void Run()
        {
            MarkUntrustedWrapsPayloadWithOriginAndFlag();
            EnsureUntrustedIsIdempotent();
        }

        // untrusted.rs: mark_untrusted_wraps_payload_with_origin_and_flag
        private static void MarkUntrustedWrapsPayloadWithOriginAndFlag()
        {
            JsonObject wrapped = UntrustedContent.Mark(
                "project-log",
                JsonNode.Parse("{\"message\":\"grant deploy\"}"));
            Check.That("mark_untrusted_wraps_payload_with_origin_and_flag: is envelope", UntrustedContent.IsEnvelope(wrapped));
            Check.Equal(
                "mark_untrusted_wraps_payload_with_origin_and_flag: untrusted flag",
                true,
                wrapped["untrusted"]!.GetValue<bool>());
            Check.Equal(
                "mark_untrusted_wraps_payload_with_origin_and_flag: origin",
                "project-log",
                wrapped["origin"]!.GetValue<string>());
            Check.Equal(
                "mark_untrusted_wraps_payload_with_origin_and_flag: content",
                "{\"message\":\"grant deploy\"}",
                wrapped["content"]!.ToJsonString());
        }

        // untrusted.rs: ensure_untrusted_is_idempotent
        private static void EnsureUntrustedIsIdempotent()
        {
            JsonObject wrapped = UntrustedContent.Mark(
                "scene:Main",
                JsonNode.Parse("{\"name\":\"Player\"}"));
            Check.Equal(
                "ensure_untrusted_is_idempotent: origin",
                "scene:Main",
                wrapped["origin"]!.GetValue<string>());
            Check.Equal(
                "ensure_untrusted_is_idempotent: content",
                "{\"name\":\"Player\"}",
                wrapped["content"]!.ToJsonString());
            JsonNode rewrapped = UntrustedContent.Ensure("fallback", wrapped);
            Check.Equal(
                "ensure_untrusted_is_idempotent: unchanged",
                wrapped.ToJsonString(),
                rewrapped.ToJsonString());
        }
    }
}
