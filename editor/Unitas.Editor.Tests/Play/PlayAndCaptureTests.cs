using System;
using System.Text.Json.Nodes;
using Unitas.Editor.Mcp;

namespace Unitas.Editor.Tests.Play
{
    public static class PlayAndCaptureTests
    {
        public static void Run()
        {
            PlaySessionLifecycleAndIsolationWithWorker();
            WorkerCrashEndsOnlyThatPlaySessionAndHostContinues();
            PersonOnlyGrantRefusedWithoutCredentialOrOverStdioAndAllowedWithCredential();
            LiveWorkerCaptureReturnsImageIdMapAndResolvableObjectBounds();
        }

        private static void PlaySessionLifecycleAndIsolationWithWorker()
        {
            using McpHost host = BuiltinDefaults.CreateHostWithDefaults();
            using PlaySessionManager manager = PlayTools.Register(host);
            CaptureTool.Register(host, manager);

            string sessionA = host.CreateSession(new SessionIdentity("agent-a", "http", "person-1"));
            string sessionB = host.CreateSession(new SessionIdentity("agent-b", "http", "person-1"));

            bool refusedWithoutExecuteScope = false;
            try
            {
                host.CallTool(sessionA, "play.enter", new JsonObject());
            }
            catch (McpError.MissingScope missing)
            {
                refusedWithoutExecuteScope = missing.Scope == Scope.ExecuteProjectCode;
            }

            Check.That(
                "play_session_lifecycle: play.enter requires execute-project-code scope",
                refusedWithoutExecuteScope);

            host.GrantScope(sessionA, Scope.ExecuteProjectCode, GrantAuthority.Person);
            host.GrantScope(sessionB, Scope.ExecuteProjectCode, GrantAuthority.Person);

            JsonObject enterA = (JsonObject)host.CallTool(sessionA, "play.enter", new JsonObject());
            Check.That(
                "play_session_lifecycle: enter result is wrapped in untrusted envelope",
                UntrustedContent.IsEnvelope(enterA));
            JsonObject contentA = (JsonObject)enterA["content"]!;
            string playIdA = contentA["playSessionId"]!.GetValue<string>();
            string opIdA = enterA["operationId"]!.GetValue<string>();
            Check.Equal(
                "play_session_lifecycle: session A starts in playing state",
                PlaySession.StatePlaying,
                contentA["state"]!.GetValue<string>());
            Check.Equal(
                "play_session_lifecycle: operation A is pending while play session runs",
                "pending",
                host.GetOperation(opIdA).State.StatusString);

            JsonObject enterB = (JsonObject)host.CallTool(sessionB, "play.enter", new JsonObject());
            JsonObject contentB = (JsonObject)enterB["content"]!;
            string playIdB = contentB["playSessionId"]!.GetValue<string>();
            Check.That(
                "play_session_lifecycle: concurrent play sessions have distinct IDs",
                !string.Equals(playIdA, playIdB, StringComparison.Ordinal));

            JsonObject injectA = (JsonObject)host.CallTool(
                sessionA,
                "play.inject_input",
                new JsonObject
                {
                    ["playSessionId"] = playIdA,
                    ["events"] = new JsonArray(
                        new JsonObject
                        {
                            ["kind"] = "key",
                            ["key"] = 100,
                            ["pressed"] = true,
                        }),
                });
            Check.Equal(
                "play_session_lifecycle: inject_input preserves playing state",
                PlaySession.StatePlaying,
                ((JsonObject)injectA["content"]!)["state"]!.GetValue<string>());

            JsonObject stepA = (JsonObject)host.CallTool(
                sessionA,
                "play.step",
                new JsonObject
                {
                    ["playSessionId"] = playIdA,
                    ["frames"] = 5,
                });
            JsonObject stepContentA = (JsonObject)stepA["content"]!;
            Check.Equal(
                "play_session_lifecycle: session A advanced 5 frames",
                5,
                (int)stepContentA["frames"]!.GetValue<long>());

            JsonObject stepB = (JsonObject)host.CallTool(
                sessionB,
                "play.step",
                new JsonObject
                {
                    ["playSessionId"] = playIdB,
                    ["frames"] = 2,
                });
            JsonObject stepContentB = (JsonObject)stepB["content"]!;
            Check.Equal(
                "play_session_lifecycle: session B advanced 2 frames independently of session A",
                2,
                (int)stepContentB["frames"]!.GetValue<long>());

            JsonObject pauseA = (JsonObject)host.CallTool(
                sessionA,
                "play.pause",
                new JsonObject
                {
                    ["playSessionId"] = playIdA,
                });
            Check.Equal(
                "play_session_lifecycle: session A transitions to paused",
                PlaySession.StatePaused,
                ((JsonObject)pauseA["content"]!)["state"]!.GetValue<string>());

            JsonObject stateB = (JsonObject)host.CallTool(
                sessionB,
                "play.state",
                new JsonObject
                {
                    ["playSessionId"] = playIdB,
                });
            Check.Equal(
                "play_session_lifecycle: session B remains playing when session A pauses",
                PlaySession.StatePlaying,
                ((JsonObject)stateB["content"]!)["state"]!.GetValue<string>());

            JsonObject exitA = (JsonObject)host.CallTool(
                sessionA,
                "play.exit",
                new JsonObject
                {
                    ["playSessionId"] = playIdA,
                });
            Check.Equal(
                "play_session_lifecycle: session A transitions to stopped on exit",
                PlaySession.StateStopped,
                ((JsonObject)exitA["content"]!)["state"]!.GetValue<string>());
            Check.Equal(
                "play_session_lifecycle: operation A completes on play.exit",
                "completed",
                host.GetOperation(opIdA).State.StatusString);

            JsonObject exitB = (JsonObject)host.CallTool(
                sessionB,
                "play.exit",
                new JsonObject
                {
                    ["playSessionId"] = playIdB,
                });
            Check.Equal(
                "play_session_lifecycle: session B exits cleanly",
                PlaySession.StateStopped,
                ((JsonObject)exitB["content"]!)["state"]!.GetValue<string>());
        }

        private static void WorkerCrashEndsOnlyThatPlaySessionAndHostContinues()
        {
            using McpHost host = BuiltinDefaults.CreateHostWithDefaults();
            using PlaySessionManager manager = PlayTools.Register(host);

            string sessionId = host.CreateSession(new SessionIdentity("agent-crash-test", "http", "person-1"));
            host.GrantScope(sessionId, Scope.ExecuteProjectCode, GrantAuthority.Person);

            JsonObject enterFirst = (JsonObject)host.CallTool(sessionId, "play.enter", new JsonObject());
            string firstPlayId = ((JsonObject)enterFirst["content"]!)["playSessionId"]!.GetValue<string>();
            string firstOpId = enterFirst["operationId"]!.GetValue<string>();

            JsonObject enterSecond = (JsonObject)host.CallTool(sessionId, "play.enter", new JsonObject());
            string secondPlayId = ((JsonObject)enterSecond["content"]!)["playSessionId"]!.GetValue<string>();

            PlaySession firstSession = manager.GetSession(firstPlayId, sessionId);
            firstSession.Worker.Kill();

            bool stepOnCrashedFailed = false;
            try
            {
                host.CallTool(
                    sessionId,
                    "play.step",
                    new JsonObject
                    {
                        ["playSessionId"] = firstPlayId,
                        ["frames"] = 1,
                    });
            }
            catch (McpError.InvalidRequest error)
            {
                stepOnCrashedFailed = error.Message.Contains("session", StringComparison.OrdinalIgnoreCase);
            }

            Check.That(
                "worker_crash_isolation: command to crashed worker reports session ended",
                stepOnCrashedFailed);

            JsonObject crashedStateEnvelope = (JsonObject)host.CallTool(
                sessionId,
                "play.state",
                new JsonObject
                {
                    ["playSessionId"] = firstPlayId,
                });
            JsonObject crashedState = (JsonObject)crashedStateEnvelope["content"]!;
            Check.Equal(
                "worker_crash_isolation: crashed session state is session-ended",
                PlaySession.StateSessionEnded,
                crashedState["state"]!.GetValue<string>());
            Check.Equal(
                "worker_crash_isolation: crashed session marks crashed=true",
                true,
                crashedState["crashed"]!.GetValue<bool>());
            Check.Equal(
                "worker_crash_isolation: crashed session operation transitions to failed",
                "failed",
                host.GetOperation(firstOpId).State.StatusString);

            JsonObject secondStepEnvelope = (JsonObject)host.CallTool(
                sessionId,
                "play.step",
                new JsonObject
                {
                    ["playSessionId"] = secondPlayId,
                    ["frames"] = 3,
                });
            JsonObject secondStep = (JsonObject)secondStepEnvelope["content"]!;
            Check.Equal(
                "worker_crash_isolation: sibling play session continues running after crash",
                3,
                (int)secondStep["frames"]!.GetValue<long>());

            JsonObject statusEnvelope = (JsonObject)host.CallTool(
                sessionId,
                "project.status",
                new JsonObject());
            Check.Equal(
                "worker_crash_isolation: editor MCP host continues serving tools after worker crash",
                "ready",
                ((JsonObject)statusEnvelope["content"]!)["state"]!.GetValue<string>());

            host.CallTool(
                sessionId,
                "play.exit",
                new JsonObject
                {
                    ["playSessionId"] = secondPlayId,
                });
        }

        private static void PersonOnlyGrantRefusedWithoutCredentialOrOverStdioAndAllowedWithCredential()
        {
            PersonCredential credential = PersonCredential.FromToken("person-secret-token");
            using McpHost host = new McpHost(new SystemClock(), credential);
            BuiltinDefaults.Register(host);
            using PlaySessionManager manager = PlayTools.Register(host);

            string stdioSession = host.CreateSession(new SessionIdentity("agent-stdio", "stdio", "stdio-local"));
            string httpSession = host.CreateSession(new SessionIdentity("agent-http", "http", "http-local"));

            JsonObject requestEnvelope = (JsonObject)host.CallTool(
                stdioSession,
                "permissions.request_grant",
                new JsonObject
                {
                    ["scope"] = "execute-project-code",
                    ["reason"] = "need play mode for testing",
                });
            JsonObject requestContent = (JsonObject)requestEnvelope["content"]!;
            Check.Equal(
                "person_only_grant: agent can discover and request person-only grant",
                "requested-person-action",
                requestContent["status"]!.GetValue<string>());
            Check.Equal(
                "person_only_grant: request_grant does not self-grant scope",
                false,
                requestContent["granted"]!.GetValue<bool>());

            bool refusedOverStdio = false;
            try
            {
                host.CallTool(
                    stdioSession,
                    "permissions.person_grant",
                    new JsonObject
                    {
                        ["scope"] = "execute-project-code",
                        ["personCredential"] = "person-secret-token",
                    });
            }
            catch (McpError.PersonCredentialRequired)
            {
                refusedOverStdio = true;
            }

            Check.That(
                "person_only_grant: stdio transport is refused even with valid person token (AUT-007)",
                refusedOverStdio);

            bool refusedWithWrongToken = false;
            try
            {
                host.CallTool(
                    httpSession,
                    "permissions.person_grant",
                    new JsonObject
                    {
                        ["scope"] = "execute-project-code",
                        ["personCredential"] = "forged-token",
                    });
            }
            catch (McpError.PersonCredentialRequired)
            {
                refusedWithWrongToken = true;
            }

            Check.That(
                "person_only_grant: wrong token is refused over http",
                refusedWithWrongToken);

            JsonObject grantedEnvelope = (JsonObject)host.CallTool(
                httpSession,
                "permissions.person_grant",
                new JsonObject
                {
                    ["scope"] = "execute-project-code",
                    ["personCredential"] = "person-secret-token",
                });
            JsonObject grantedContent = (JsonObject)grantedEnvelope["content"]!;
            Check.Equal(
                "person_only_grant: valid out-of-band person credential grants scope over interactive transport",
                "granted",
                grantedContent["status"]!.GetValue<string>());
        }

        private static void LiveWorkerCaptureReturnsImageIdMapAndResolvableObjectBounds()
        {
            using McpHost host = BuiltinDefaults.CreateHostWithDefaults();
            using PlaySessionManager manager = PlayTools.Register(host);
            CaptureTool.Register(host, manager);

            string sessionId = host.CreateSession(new SessionIdentity("agent-capture", "http", "person-1"));
            host.GrantScope(sessionId, Scope.ExecuteProjectCode, GrantAuthority.Person);

            JsonObject enterEnvelope = (JsonObject)host.CallTool(sessionId, "play.enter", new JsonObject());
            string playSessionId = ((JsonObject)enterEnvelope["content"]!)["playSessionId"]!.GetValue<string>();

            host.CallTool(
                sessionId,
                "play.step",
                new JsonObject
                {
                    ["playSessionId"] = playSessionId,
                    ["frames"] = 4,
                });

            int captureWidth = 64;
            int captureHeight = 36;
            JsonObject captureEnvelope = (JsonObject)host.CallTool(
                sessionId,
                "capture",
                new JsonObject
                {
                    ["playSessionId"] = playSessionId,
                    ["width"] = captureWidth,
                    ["height"] = captureHeight,
                    ["id_map"] = true,
                });

            Check.That(
                "live_worker_capture: capture response is wrapped in untrusted envelope",
                UntrustedContent.IsEnvelope(captureEnvelope));
            JsonObject captureContent = (JsonObject)captureEnvelope["content"]!;
            Check.Equal(
                "live_worker_capture: width matches requested 64",
                captureWidth,
                captureContent["width"]!.GetValue<int>());
            Check.Equal(
                "live_worker_capture: height matches requested 36",
                captureHeight,
                captureContent["height"]!.GetValue<int>());

            string base64Image = captureContent["image"]!.GetValue<string>();
            byte[] decodedImage = Convert.FromBase64String(base64Image);
            Check.Equal(
                "live_worker_capture: base64 RGBA8 image byte count equals width * height * 4",
                captureWidth * captureHeight * 4,
                decodedImage.Length);

            JsonArray idMap = (JsonArray)captureContent["id_map"]!;
            Check.Equal(
                "live_worker_capture: id_map has width * height entries",
                captureWidth * captureHeight,
                idMap.Count);

            JsonArray objects = (JsonArray)captureContent["objects"]!;
            Check.That(
                "live_worker_capture: objects list is non-empty",
                objects.Count > 0);

            int resolvedX = -1;
            int resolvedY = -1;
            uint expectedObjectId = 0;
            string expectedObjectName = string.Empty;

            foreach (JsonNode? objNode in objects)
            {
                if (!(objNode is JsonObject obj))
                {
                    continue;
                }

                uint objectId = obj["id"]!.GetValue<uint>();
                string objectName = obj["name"]!.GetValue<string>();
                JsonObject bounds = (JsonObject)obj["bounds"]!;
                int bx = bounds["x"]!.GetValue<int>();
                int by = bounds["y"]!.GetValue<int>();
                int bw = bounds["width"]!.GetValue<int>();
                int bh = bounds["height"]!.GetValue<int>();

                for (int py = by; py < by + bh && py < captureHeight; py++)
                {
                    for (int px = bx; px < bx + bw && px < captureWidth; px++)
                    {
                        uint pixelId = CaptureTool.ResolvePixelObjectId(captureContent, px, py);
                        if (pixelId == objectId)
                        {
                            resolvedX = px;
                            resolvedY = py;
                            expectedObjectId = objectId;
                            expectedObjectName = objectName;
                            break;
                        }
                    }

                    if (expectedObjectId != 0)
                    {
                        break;
                    }
                }

                if (expectedObjectId != 0)
                {
                    break;
                }
            }

            Check.That(
                "live_worker_capture: found a pixel within an object's bounds that resolves to its ID in id_map",
                expectedObjectId != 0 && resolvedX >= 0 && resolvedY >= 0);

            JsonObject pixelCaptureEnvelope = (JsonObject)host.CallTool(
                sessionId,
                "capture",
                new JsonObject
                {
                    ["playSessionId"] = playSessionId,
                    ["width"] = captureWidth,
                    ["height"] = captureHeight,
                    ["id_map"] = true,
                    ["pixelX"] = resolvedX,
                    ["pixelY"] = resolvedY,
                });
            JsonObject pixelCaptureContent = (JsonObject)pixelCaptureEnvelope["content"]!;
            Check.Equal(
                "live_worker_capture: pixel query resolves expected object ID",
                (int)expectedObjectId,
                (int)pixelCaptureContent["resolvedObjectId"]!.GetValue<uint>());
            Check.Equal(
                "live_worker_capture: pixel query resolves expected object name",
                expectedObjectName,
                pixelCaptureContent["resolvedObjectName"]!.GetValue<string>());

            host.CallTool(
                sessionId,
                "play.exit",
                new JsonObject
                {
                    ["playSessionId"] = playSessionId,
                });
        }
    }
}
