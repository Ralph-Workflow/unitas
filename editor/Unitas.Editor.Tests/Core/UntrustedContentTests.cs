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
            CompleteOperationWrapsResultInUntrustedEnvelope();
        }

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

        private static void CompleteOperationWrapsResultInUntrustedEnvelope()
        {
            using McpHost host = new McpHost(new FixedClock(1_700_000_000_000L));
            string session = host.CreateSession(new SessionIdentity("agent-1", "stdio", "person-1"));
            host.GrantScope(session, Scope.Build, GrantAuthority.Person);
            string operationId = host.StartOperation(session, "build.macos", Scope.Build);
            host.CompleteOperation(operationId, "build-pipeline", new JsonObject { ["bundle"] = "Game.app" });

            OperationHandle handle = host.GetOperation(operationId);
            Check.That(
                "resource_read_marks_untrusted_and_operations_track_completion: completed state is Completed",
                handle.State is OperationState.Completed);
            JsonNode? result = (handle.State as OperationState.Completed)?.Result;
            Check.That(
                "resource_read_marks_untrusted_and_operations_track_completion: completed operation result is untrusted envelope",
                UntrustedContent.IsEnvelope(result));
            Check.Equal(
                "resource_read_marks_untrusted_and_operations_track_completion: completed operation origin",
                "build-pipeline",
                result?["origin"]?.GetValue<string>() ?? string.Empty);
        }
    }
}
