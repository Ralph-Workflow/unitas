using System;
using System.Text.Json.Nodes;

namespace Unitas.Editor.Mcp
{
    public static class CaptureTool
    {
        public const string CaptureFrameOrigin = "unitas://capture/frame";
        public const int DefaultCaptureWidth = 64;
        public const int DefaultCaptureHeight = 36;

        public static void Register(McpHost host, PlaySessionManager manager)
        {
            ArgumentNullException.ThrowIfNull(host);
            ArgumentNullException.ThrowIfNull(manager);

            host.RegisterTool(
                new ToolDefinition(
                    "capture",
                    "Capture an offscreen rendered frame, per-pixel object ID map, and object screen bounds",
                    new JsonObject
                    {
                        ["type"] = "object",
                        ["properties"] = new JsonObject
                        {
                            ["playSessionId"] = new JsonObject
                            {
                                ["type"] = "string",
                            },
                            ["width"] = new JsonObject
                            {
                                ["type"] = "integer",
                            },
                            ["height"] = new JsonObject
                            {
                                ["type"] = "integer",
                            },
                            ["id_map"] = new JsonObject
                            {
                                ["type"] = "boolean",
                            },
                            ["pixelX"] = new JsonObject
                            {
                                ["type"] = "integer",
                            },
                            ["pixelY"] = new JsonObject
                            {
                                ["type"] = "integer",
                            },
                        },
                    },
                    Scope.ReadProject),
                arguments => HandleCapture(host, manager, arguments));
        }

        public static uint ResolvePixelObjectId(JsonObject capturePayload, int pixelX, int pixelY)
        {
            ArgumentNullException.ThrowIfNull(capturePayload);
            int width = ExtractRequiredInt(capturePayload, "width");
            int height = ExtractRequiredInt(capturePayload, "height");
            if (pixelX < 0 || pixelX >= width || pixelY < 0 || pixelY >= height)
            {
                throw new McpError.InvalidParams(
                    "pixel coordinates (" + pixelX + ", " + pixelY + ") are outside capture bounds " + width + "x" + height);
            }

            JsonArray? idMap = capturePayload["id_map"] as JsonArray;
            if (idMap == null)
            {
                throw new McpError.ParseError("capture payload is missing 'id_map' array");
            }

            int index = (pixelY * width) + pixelX;
            if (index < 0 || index >= idMap.Count)
            {
                throw new McpError.ParseError("capture id_map length does not match width * height");
            }

            JsonNode? entry = idMap[index];
            if (entry is JsonValue value)
            {
                if (value.TryGetValue(out uint uintId))
                {
                    return uintId;
                }

                if (value.TryGetValue(out int intId) && intId >= 0)
                {
                    return (uint)intId;
                }

                if (value.TryGetValue(out long longId) && longId >= 0)
                {
                    return (uint)longId;
                }
            }

            throw new McpError.ParseError("capture id_map entry is not a valid object ID");
        }

        public static string? ResolveObjectName(JsonObject capturePayload, uint objectId)
        {
            ArgumentNullException.ThrowIfNull(capturePayload);
            if (objectId == 0)
            {
                return null;
            }

            JsonArray? objects = capturePayload["objects"] as JsonArray;
            if (objects == null)
            {
                return null;
            }

            foreach (JsonNode? node in objects)
            {
                if (!(node is JsonObject obj))
                {
                    continue;
                }

                if (obj["id"] is JsonValue idValue)
                {
                    uint candidateId = 0;
                    if (idValue.TryGetValue(out uint parsedUint))
                    {
                        candidateId = parsedUint;
                    }
                    else if (idValue.TryGetValue(out int parsedInt) && parsedInt >= 0)
                    {
                        candidateId = (uint)parsedInt;
                    }
                    else if (idValue.TryGetValue(out long parsedLong) && parsedLong >= 0)
                    {
                        candidateId = (uint)parsedLong;
                    }

                    if (candidateId == objectId
                        && obj["name"] is JsonValue nameValue
                        && nameValue.TryGetValue(out string? name))
                    {
                        return name;
                    }
                }
            }

            return null;
        }

        private static ToolOutput HandleCapture(
            McpHost host,
            PlaySessionManager manager,
            JsonNode arguments)
        {
            string callerSessionId = host.CurrentSessionId
                ?? throw new McpError.InvalidRequest("capture requires an active MCP session");

            int width = ExtractOptionalInt(arguments, "width", DefaultCaptureWidth);
            int height = ExtractOptionalInt(arguments, "height", DefaultCaptureHeight);
            bool idMap = ExtractOptionalBool(arguments, "id_map", true);
            string? requestedPlaySessionId = ExtractOptionalString(arguments, "playSessionId");

            string operationId = host.StartOperation(callerSessionId, "capture", Scope.ReadProject);
            JsonObject workerResponse;
            string affectedIdentity;

            PlaySession? existingSession = !string.IsNullOrWhiteSpace(requestedPlaySessionId)
                ? manager.GetSession(requestedPlaySessionId, callerSessionId)
                : manager.TryGetSession(null, callerSessionId);

            try
            {
                if (existingSession != null && existingSession.State != PlaySession.StateStopped && !existingSession.Crashed)
                {
                    workerResponse = existingSession.Capture(width, height, idMap);
                    affectedIdentity = existingSession.PlaySessionId;
                }
                else if (!string.IsNullOrWhiteSpace(requestedPlaySessionId))
                {
                    workerResponse = existingSession!.Capture(width, height, idMap);
                    affectedIdentity = existingSession.PlaySessionId;
                }
                else
                {
                    workerResponse = CaptureOneShot(manager.WorkerFactory, width, height, idMap);
                    affectedIdentity = CaptureFrameOrigin;
                }
            }
            catch (McpError error)
            {
                host.FailOperation(operationId, error.Message);
                throw;
            }

            JsonObject payload = BuildCapturePayload(workerResponse, existingSession?.PlaySessionId, arguments);
            host.CompleteOperation(operationId, CaptureFrameOrigin, payload);
            return ToolOutput.Untrusted(CaptureFrameOrigin, payload)
                .WithAffectedIdentities(affectedIdentity)
                .WithOperation(operationId);
        }

        private static JsonObject CaptureOneShot(
            PlayWorkerFactory workerFactory,
            int width,
            int height,
            bool idMap)
        {
            if (width <= 0 || height <= 0)
            {
                throw new McpError.InvalidParams("capture width and height must be positive integers");
            }

            using IPlayWorker worker = workerFactory(null);
            worker.SendCommand(new JsonObject
            {
                ["cmd"] = "play",
            });
            return worker.SendCommand(new JsonObject
            {
                ["cmd"] = "capture",
                ["width"] = width,
                ["height"] = height,
                ["id_map"] = idMap,
            });
        }

        private static JsonObject BuildCapturePayload(
            JsonObject workerResponse,
            string? playSessionId,
            JsonNode arguments)
        {
            JsonObject payload = new JsonObject
            {
                ["width"] = workerResponse["width"]?.DeepClone(),
                ["height"] = workerResponse["height"]?.DeepClone(),
                ["image"] = workerResponse["image"]?.DeepClone(),
                ["id_map"] = workerResponse["id_map"]?.DeepClone(),
                ["objects"] = workerResponse["objects"]?.DeepClone(),
            };

            if (!string.IsNullOrEmpty(playSessionId))
            {
                payload["playSessionId"] = playSessionId;
            }

            int? pixelX = TryExtractInt(arguments, "pixelX");
            int? pixelY = TryExtractInt(arguments, "pixelY");
            if (pixelX.HasValue && pixelY.HasValue)
            {
                uint objectId = ResolvePixelObjectId(payload, pixelX.Value, pixelY.Value);
                payload["resolvedObjectId"] = objectId;
                payload["resolvedObjectName"] = ResolveObjectName(payload, objectId);
            }

            return payload;
        }

        private static string? ExtractOptionalString(JsonNode arguments, string propertyName)
        {
            if (arguments is JsonObject obj
                && obj[propertyName] is JsonValue value
                && value.TryGetValue(out string? text)
                && !string.IsNullOrWhiteSpace(text))
            {
                return text;
            }

            return null;
        }

        private static int ExtractOptionalInt(JsonNode arguments, string propertyName, int defaultValue)
        {
            int? extracted = TryExtractInt(arguments, propertyName);
            return extracted ?? defaultValue;
        }

        private static int? TryExtractInt(JsonNode arguments, string propertyName)
        {
            if (arguments is JsonObject obj && obj[propertyName] is JsonValue value)
            {
                if (value.TryGetValue(out int intValue))
                {
                    return intValue;
                }

                if (value.TryGetValue(out long longValue))
                {
                    return (int)longValue;
                }
            }

            return null;
        }

        private static int ExtractRequiredInt(JsonObject obj, string propertyName)
        {
            if (obj[propertyName] is JsonValue value)
            {
                if (value.TryGetValue(out int intValue))
                {
                    return intValue;
                }

                if (value.TryGetValue(out long longValue))
                {
                    return (int)longValue;
                }
            }

            throw new McpError.ParseError("capture payload is missing integer '" + propertyName + "'");
        }

        private static bool ExtractOptionalBool(JsonNode arguments, string propertyName, bool defaultValue)
        {
            if (arguments is JsonObject obj
                && obj[propertyName] is JsonValue value
                && value.TryGetValue(out bool boolValue))
            {
                return boolValue;
            }

            return defaultValue;
        }
    }
}
