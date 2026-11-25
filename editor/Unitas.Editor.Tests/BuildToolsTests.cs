using System;
using System.Collections.Generic;
using System.IO;
using System.Text.Json.Nodes;
using Unitas.Editor.Mcp;

namespace Unitas.Editor.Tests
{
    public static class BuildToolsTests
    {
        public static void Run()
        {
            RegistersBuildLicensingWithTheReadProjectScope();
            LicensingQueryReturnsTheRecordTheBundleCarries();
            MissingBundleMissingRecordAndMalformedRecordAreNamedFailures();
        }

        private static void RegistersBuildLicensingWithTheReadProjectScope()
        {
            using McpHost host = NewHost();
            BuildTools.Register(host);

            IReadOnlyList<ToolDefinition> tools = host.ListTools();
            ToolDefinition? tool = null;
            foreach (ToolDefinition candidate in tools)
            {
                if (candidate.Name == "build.licensing")
                {
                    tool = candidate;
                }
            }

            Check.That("build: build.licensing is registered", tool != null);
            if (tool == null)
            {
                return;
            }

            Check.That(
                "build: build.licensing requires ReadProject",
                tool.RequiredScope == Scope.ReadProject);
            Check.That(
                "build: build.licensing schema declares bundlePath",
                tool.InputSchema["properties"]?["bundlePath"] != null);
        }

        private static void LicensingQueryReturnsTheRecordTheBundleCarries()
        {
            string bundle = NewTempDir("licensing-ok");
            try
            {
                JsonObject record = SampleLicensingRecord();
                File.WriteAllText(
                    Path.Combine(bundle, "licensing-record.json"),
                    record.ToJsonString());

                using McpHost host = NewHost();
                BuildTools.Register(host);
                string session = host.CreateSession(new SessionIdentity("agent-1", "stdio", "stdio-local"));

                JsonNode envelope = host.CallTool(
                    session,
                    "build.licensing",
                    new JsonObject { ["bundlePath"] = bundle });

                Check.That(
                    "build: licensing query returns an untrusted envelope",
                    UntrustedContent.IsEnvelope(envelope));
                Check.Equal(
                    "build: licensing query origin",
                    "unitas://build/licensing",
                    envelope["origin"]?.GetValue<string>() ?? string.Empty);
                Check.Equal(
                    "build: governing license is GPLv3",
                    "GPL-3.0-only",
                    envelope["content"]?["governing_license"]?.GetValue<string>() ?? string.Empty);
                Check.That(
                    "build: commercial license statement is present",
                    (envelope["content"]?["commercial_license_statement"]?.GetValue<string>() ?? string.Empty)
                        .Contains("commercial license", StringComparison.Ordinal));
                JsonArray? notices = envelope["content"]?["third_party_notices"] as JsonArray;
                Check.Equal("build: third-party notices are present", 1, notices?.Count ?? 0);
            }
            finally
            {
                Directory.Delete(bundle, recursive: true);
            }
        }

        private static void MissingBundleMissingRecordAndMalformedRecordAreNamedFailures()
        {
            string absent = Path.Combine(Path.GetTempPath(), "unitas-build-test-absent-" + Guid.NewGuid().ToString("N"));
            ExpectInvalidRequest(
                "build: missing bundle is a named failure",
                absent,
                "bundle directory does not exist");

            string empty = NewTempDir("licensing-missing-record");
            try
            {
                ExpectInvalidRequest(
                    "build: bundle without a record is a named failure",
                    empty,
                    "carries no licensing record");
            }
            finally
            {
                Directory.Delete(empty, recursive: true);
            }

            string malformed = NewTempDir("licensing-malformed");
            try
            {
                File.WriteAllText(Path.Combine(malformed, "licensing-record.json"), "{ not json");
                ExpectFailure<McpError.ParseError>(
                    "build: malformed record is a parse failure",
                    malformed);
            }
            finally
            {
                Directory.Delete(malformed, recursive: true);
            }
        }

        private static void ExpectInvalidRequest(string name, string bundlePath, string expectedMessage)
        {
            McpError? error = InvokeFailure(bundlePath);
            Check.That(name + " (error type)", error is McpError.InvalidRequest);
            Check.That(
                name + " (message)",
                error != null && error.Message.Contains(expectedMessage, StringComparison.Ordinal));
        }

        private static void ExpectFailure<TError>(string name, string bundlePath)
            where TError : McpError
        {
            McpError? error = InvokeFailure(bundlePath);
            Check.That(name + " (error type)", error is TError);
        }

        private static McpError? InvokeFailure(string bundlePath)
        {
            using McpHost host = NewHost();
            BuildTools.Register(host);
            string session = host.CreateSession(new SessionIdentity("agent-1", "stdio", "stdio-local"));
            try
            {
                host.CallTool(
                    session,
                    "build.licensing",
                    new JsonObject { ["bundlePath"] = bundlePath });
            }
            catch (McpError error)
            {
                return error;
            }

            Check.That("build: failing licensing query raised an error", false);
            return null;
        }

        private static McpHost NewHost()
        {
            return new McpHost(new FixedClock(1_700_000_000_000L));
        }

        private static string NewTempDir(string label)
        {
            string path = Path.Combine(
                Path.GetTempPath(),
                "unitas-build-test-" + label + "-" + Guid.NewGuid().ToString("N"));
            Directory.CreateDirectory(path);
            return path;
        }

        // Shape matches the licensing record JSON the native build.player tool
        // writes (same keys as unitas-build's report licensing serialization).
        private static JsonObject SampleLicensingRecord()
        {
            return new JsonObject
            {
                ["application_identifier"] = "com.default.fixture",
                ["governing_license"] = "GPL-3.0-only",
                ["governing_license_name"] = "GNU General Public License version 3 (GPLv3)",
                ["conveyance_statement"] = "conveyed under the GPLv3",
                ["corresponding_source_obligation"] = "provide Corresponding Source",
                ["corresponding_source_availability"] = "offered by the conveying party",
                ["linking_exception_statement"] = "no linking exception",
                ["commercial_license_statement"] = "The commercial license is the route for distributing a player without these GPLv3 obligations.",
                ["not_legal_advice_statement"] = "not legal advice",
                ["license_texts"] = new JsonArray(
                    "GNU General Public License version 3 (Resources/LICENSE)"),
                ["third_party_notices"] = new JsonArray
                {
                    new JsonObject
                    {
                        ["ecosystem"] = "cargo",
                        ["component"] = "unitas-core",
                        ["version"] = "0.1.0",
                        ["license"] = "GPL-3.0-only",
                    },
                },
            };
        }
    }
}
