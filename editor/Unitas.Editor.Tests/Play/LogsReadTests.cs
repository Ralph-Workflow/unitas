using System;
using System.Text.Json.Nodes;
using Unitas.Editor.Mcp;

namespace Unitas.Editor.Tests.Play
{
    public static class LogsReadTests
    {
        public static void Run()
        {
            NoActivePlaySessionReturnsEmptyEntries();
            LiveWorkerLogsReadReturnsScriptedLinesAndHonorsFilters();
        }

        private static void NoActivePlaySessionReturnsEmptyEntries()
        {
            using McpHost host = BuiltinDefaults.CreateHostWithDefaults();
            using PlaySessionManager manager = new PlaySessionManager();
            LogsTools.Register(host, manager);

            string sessionId = host.CreateSession(new SessionIdentity("agent-logs-none", "http", "person-1"));

            JsonObject envelope = (JsonObject)host.CallTool(sessionId, "logs.read", new JsonObject());
            Check.That(
                "logs_read_no_session: response is wrapped in untrusted envelope",
                UntrustedContent.IsEnvelope(envelope));
            JsonObject content = (JsonObject)envelope["content"]!;
            Check.Equal(
                "logs_read_no_session: status reports no active play session",
                LogsTools.StatusNoActivePlaySession,
                content["status"]!.GetValue<string>());
            Check.Equal(
                "logs_read_no_session: entries array is empty",
                0,
                ((JsonArray)content["entries"]!).Count);
        }

        // The staged roll-a-ball bundle scripts Debug.Log calls at startup:
        // GoldPickupMarker.Start logs the gold-variant line on the first
        // frame. The filters run against the same live session.
        private static void LiveWorkerLogsReadReturnsScriptedLinesAndHonorsFilters()
        {
            using McpHost host = BuiltinDefaults.CreateHostWithDefaults();
            using PlaySessionManager manager = new PlaySessionManager();
            PlayTools.Register(host, manager);
            LogsTools.Register(host, manager);

            string sessionId = host.CreateSession(new SessionIdentity("agent-logs", "http", "person-1"));
            host.GrantScope(sessionId, Scope.ExecuteProjectCode, GrantAuthority.Person);

            JsonObject enterEnvelope = (JsonObject)host.CallTool(sessionId, "play.enter", new JsonObject());
            string playSessionId = ((JsonObject)enterEnvelope["content"]!)["playSessionId"]!.GetValue<string>();

            try
            {
                host.CallTool(
                    sessionId,
                    "play.step",
                    new JsonObject
                    {
                        ["playSessionId"] = playSessionId,
                        ["frames"] = 4,
                    });

                JsonObject allEnvelope = (JsonObject)host.CallTool(
                    sessionId,
                    "logs.read",
                    new JsonObject
                    {
                        ["sessionId"] = playSessionId,
                    });
                Check.That(
                    "logs_read_live: response is wrapped in untrusted envelope",
                    UntrustedContent.IsEnvelope(allEnvelope));
                JsonObject allContent = (JsonObject)allEnvelope["content"]!;
                Check.Equal(
                    "logs_read_live: status is ok while the play session runs",
                    LogsTools.StatusOk,
                    allContent["status"]!.GetValue<string>());
                Check.Equal(
                    "logs_read_live: playSessionId echoes the requesting session",
                    playSessionId,
                    allContent["playSessionId"]!.GetValue<string>());

                JsonArray allEntries = (JsonArray)allContent["entries"]!;
                Check.That(
                    "logs_read_live: entries are recorded after stepping frames",
                    allEntries.Count > 0);

                JsonObject? scripted = null;
                int infoCount = 0;
                int framesAboveZero = 0;
                bool everyEntryWellFormed = true;
                foreach (JsonNode? node in allEntries)
                {
                    if (!(node is JsonObject entry))
                    {
                        everyEntryWellFormed = false;
                        continue;
                    }

                    string severity = entry["severity"]!.GetValue<string>();
                    string message = entry["message"]!.GetValue<string>();
                    if ((severity != "info" && severity != "warning" && severity != "error")
                        || entry["frame"] == null
                        || entry["time"] == null
                        || entry["callStack"] == null)
                    {
                        everyEntryWellFormed = false;
                    }

                    if (severity == "info")
                    {
                        infoCount++;
                    }

                    if (entry["frame"]!.GetValue<long>() > 0)
                    {
                        framesAboveZero++;
                    }

                    if (message.Contains("gold variant applied", StringComparison.Ordinal))
                    {
                        scripted = entry;
                    }
                }

                Check.That(
                    "logs_read_live: every entry carries severity, frame, time, message and callStack",
                    everyEntryWellFormed);
                Check.That(
                    "logs_read_live: the scripted Debug.Log line from GoldPickupMarker is readable",
                    scripted != null);
                Check.Equal(
                    "logs_read_live: the scripted line is an info entry",
                    "info",
                    scripted?["severity"]?.GetValue<string>() ?? string.Empty);
                Check.That(
                    "logs_read_live: the scripted line carries a call stack",
                    (scripted?["callStack"]?.GetValue<string>() ?? string.Empty).Length > 0);

                JsonArray infoEntries = ReadEntries(host, sessionId, playSessionId, "severity", "info");
                Check.Equal(
                    "logs_read_filters: severity=info selects exactly the info entries",
                    infoCount,
                    infoEntries.Count);
                bool infoOnly = true;
                foreach (JsonNode? node in infoEntries)
                {
                    if (node is not JsonObject entry || entry["severity"]!.GetValue<string>() != "info")
                    {
                        infoOnly = false;
                    }
                }

                Check.That(
                    "logs_read_filters: severity=info returns only info entries",
                    infoOnly);

                JsonArray warningEntries = ReadEntries(host, sessionId, playSessionId, "severity", "warning");
                JsonArray errorEntries = ReadEntries(host, sessionId, playSessionId, "severity", "error");
                bool nonInfoOnly = true;
                foreach (JsonArray filtered in new[] { warningEntries, errorEntries })
                {
                    foreach (JsonNode? node in filtered)
                    {
                        if (node is not JsonObject entry || entry["severity"]!.GetValue<string>() == "info")
                        {
                            nonInfoOnly = false;
                        }
                    }
                }

                Check.That(
                    "logs_read_filters: severity=warning and severity=error return no info entries",
                    nonInfoOnly);
                Check.Equal(
                    "logs_read_filters: the three severity filters partition the unfiltered entries",
                    allEntries.Count,
                    infoEntries.Count + warningEntries.Count + errorEntries.Count);

                JsonArray afterFrameZero = ReadEntries(host, sessionId, playSessionId, "afterFrame", 0);
                Check.Equal(
                    "logs_read_filters: afterFrame=0 excludes entries at frame 0",
                    framesAboveZero,
                    afterFrameZero.Count);
                bool aboveZeroOnly = true;
                foreach (JsonNode? node in afterFrameZero)
                {
                    if (node is not JsonObject entry || entry["frame"]!.GetValue<long>() <= 0)
                    {
                        aboveZeroOnly = false;
                    }
                }

                Check.That(
                    "logs_read_filters: afterFrame=0 returns only entries above frame 0",
                    aboveZeroOnly);

                JsonArray afterFarFuture = ReadEntries(host, sessionId, playSessionId, "afterFrame", 1000000);
                Check.Equal(
                    "logs_read_filters: afterFrame beyond the session returns no entries",
                    0,
                    afterFarFuture.Count);

                JsonArray limited = ReadEntries(host, sessionId, playSessionId, "limit", 1);
                Check.Equal(
                    "logs_read_filters: limit=1 caps the entries at one",
                    1,
                    limited.Count);
                Check.Equal(
                    "logs_read_filters: limit=1 returns the first recorded entry",
                    allEntries[0]!["message"]!.GetValue<string>(),
                    limited[0]!["message"]!.GetValue<string>());
            }
            finally
            {
                host.CallTool(
                    sessionId,
                    "play.exit",
                    new JsonObject
                    {
                        ["playSessionId"] = playSessionId,
                    });
            }
        }

        private static JsonArray ReadEntries(
            McpHost host,
            string sessionId,
            string playSessionId,
            string filterName,
            JsonNode filterValue)
        {
            JsonObject envelope = (JsonObject)host.CallTool(
                sessionId,
                "logs.read",
                new JsonObject
                {
                    ["sessionId"] = playSessionId,
                    [filterName] = filterValue.DeepClone(),
                });
            JsonObject content = (JsonObject)envelope["content"]!;
            Check.Equal(
                "logs_read_filters: filtered read reports ok",
                LogsTools.StatusOk,
                content["status"]!.GetValue<string>());
            return (JsonArray)content["entries"]!;
        }
    }
}
