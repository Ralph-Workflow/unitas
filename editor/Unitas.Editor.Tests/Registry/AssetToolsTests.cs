using System;
using System.IO;
using System.Text;
using System.Text.Json.Nodes;
using Unitas.Editor.Mcp;

namespace Unitas.Editor.Tests
{
    public static class AssetToolsTests
    {
        private static readonly byte[] MinimalWavBytes = CreateMinimalWav();

        public static void Run()
        {
            RegistersAssetsImportWithLeastPrivilegeScope();
            ImportsTextMaterialAndBinaryWavWithMetaAndDiagnostics();
            ReusesExistingGuidAndAcceptsCustomGuid();
            RecordsAuditAndChangeSetAndRejectRestoresWorkspaceClean();
            RestoresOverwrittenBinaryAndTextAssetsOnReject();
            RefusesPathEscapeOutsideAssetsAndInvalidPayloads();
        }

        private static void RegistersAssetsImportWithLeastPrivilegeScope()
        {
            using McpHost host = NewHost();
            ChangeSetStore store = new ChangeSetStore(host);
            AssetTools.Register(host, store);

            Check.Equal("assets: one asset tool is registered", 1, host.ListTools().Count);
            Check.Equal("assets: tool name is assets.import", "assets.import", host.ListTools()[0].Name);
            Check.That(
                "assets: assets.import requires edit-content",
                Scope.EditContent == host.ListTools()[0].RequiredScope);

            using McpHost fullHost = NewHost();
            using PlaySessionManager playSessions = new PlaySessionManager();
            BuiltinDefaults.RegisterAll(fullHost, playSessions);
            ToolDefinition? found = null;
            foreach (ToolDefinition tool in fullHost.ListTools())
            {
                if (string.Equals(tool.Name, "assets.import", StringComparison.Ordinal))
                {
                    found = tool;
                    break;
                }
            }

            Check.That("assets: BuiltinDefaults.RegisterAll registers assets.import", found != null);
            Check.That(
                "assets: assets.import requires edit-content",
                found != null && Scope.EditContent == found.RequiredScope);
        }

        private static void ImportsTextMaterialAndBinaryWavWithMetaAndDiagnostics()
        {
            string project = CreateProject("import-mat-wav");
            try
            {
                using McpHost host = NewHost();
                ChangeSetStore store = new ChangeSetStore(host);
                AssetTools.Register(host, store);
                string session = NewEditingSession(host);

                string materialYaml =
                    "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!21 &2100000\nMaterial:\n  serializedVersion: 6\n  m_Name: BonusPickup\n";
                JsonNode matEnvelope = host.CallTool(
                    session,
                    "assets.import",
                    new JsonObject
                    {
                        ["projectPath"] = project,
                        ["path"] = "Assets/Materials/BonusPickup.mat",
                        ["text"] = materialYaml,
                    });
                Check.That("assets: mat import returns untrusted envelope", UntrustedContent.IsEnvelope(matEnvelope));
                Check.Equal(
                    "assets: mat import origin",
                    "unitas://assets/import",
                    matEnvelope["origin"]?.GetValue<string>() ?? string.Empty);

                JsonObject matContent = (JsonObject)matEnvelope["content"]!;
                Check.Equal("assets: mat status is imported", "imported", matContent["status"]?.GetValue<string>() ?? string.Empty);
                Check.Equal("assets: mat path", "Assets/Materials/BonusPickup.mat", matContent["path"]?.GetValue<string>() ?? string.Empty);
                Check.Equal("assets: mat metaPath", "Assets/Materials/BonusPickup.mat.meta", matContent["metaPath"]?.GetValue<string>() ?? string.Empty);
                Check.Equal("assets: mat importer", "NativeFormatImporter", matContent["importer"]?.GetValue<string>() ?? string.Empty);
                Check.Equal("assets: mat bytes", Encoding.UTF8.GetByteCount(materialYaml), matContent["bytes"]?.GetValue<int>() ?? 0);
                string matGuid = matContent["guid"]?.GetValue<string>() ?? string.Empty;
                Check.That("assets: mat guid is 32 lowercase hex chars", IsValidLowerHexGuid(matGuid));
                JsonArray? matDiagnostics = matContent["diagnostics"] as JsonArray;
                Check.Equal("assets: mat import produces zero error diagnostics", 0, matDiagnostics?.Count ?? -1);

                string matFile = Path.Combine(project, "Assets", "Materials", "BonusPickup.mat");
                string matMetaFile = matFile + ".meta";
                Check.That("assets: mat file exists on disk", File.Exists(matFile));
                Check.That("assets: mat meta file exists on disk", File.Exists(matMetaFile));
                Check.Equal("assets: mat file content matches", materialYaml, File.ReadAllText(matFile));
                string matMetaText = File.ReadAllText(matMetaFile);
                Check.That("assets: mat meta has fileFormatVersion 2", matMetaText.Contains("fileFormatVersion: 2\n", StringComparison.Ordinal));
                Check.That("assets: mat meta has guid", matMetaText.Contains("guid: " + matGuid + "\n", StringComparison.Ordinal));
                Check.That("assets: mat meta has NativeFormatImporter", matMetaText.Contains("NativeFormatImporter:\n", StringComparison.Ordinal));
                Check.That("assets: mat meta has mainObjectFileID 2100000", matMetaText.Contains("mainObjectFileID: 2100000\n", StringComparison.Ordinal));

                string wavBase64 = Convert.ToBase64String(MinimalWavBytes);
                JsonNode wavEnvelope = host.CallTool(
                    session,
                    "assets.import",
                    new JsonObject
                    {
                        ["projectPath"] = project,
                        ["path"] = "Assets/Audio/BonusChime.wav",
                        ["base64"] = wavBase64,
                    });
                JsonObject wavContent = (JsonObject)wavEnvelope["content"]!;
                Check.Equal("assets: wav status is imported", "imported", wavContent["status"]?.GetValue<string>() ?? string.Empty);
                Check.Equal("assets: wav path", "Assets/Audio/BonusChime.wav", wavContent["path"]?.GetValue<string>() ?? string.Empty);
                Check.Equal("assets: wav metaPath", "Assets/Audio/BonusChime.wav.meta", wavContent["metaPath"]?.GetValue<string>() ?? string.Empty);
                Check.Equal("assets: wav importer", "AudioImporter", wavContent["importer"]?.GetValue<string>() ?? string.Empty);
                Check.Equal("assets: wav bytes", MinimalWavBytes.Length, wavContent["bytes"]?.GetValue<int>() ?? 0);
                string wavGuid = wavContent["guid"]?.GetValue<string>() ?? string.Empty;
                Check.That("assets: wav guid is 32 lowercase hex chars", IsValidLowerHexGuid(wavGuid));

                string wavFile = Path.Combine(project, "Assets", "Audio", "BonusChime.wav");
                string wavMetaFile = wavFile + ".meta";
                Check.That("assets: wav file exists on disk", File.Exists(wavFile));
                Check.That("assets: wav meta file exists on disk", File.Exists(wavMetaFile));
                Check.That("assets: wav bytes match", ByteArraysEqual(MinimalWavBytes, File.ReadAllBytes(wavFile)));
                string wavMetaText = File.ReadAllText(wavMetaFile);
                Check.That("assets: wav meta has AudioImporter", wavMetaText.Contains("AudioImporter:\n", StringComparison.Ordinal));
                Check.That("assets: wav meta has guid", wavMetaText.Contains("guid: " + wavGuid + "\n", StringComparison.Ordinal));
            }
            finally
            {
                DeleteDirectory(project);
            }
        }

        private static void ReusesExistingGuidAndAcceptsCustomGuid()
        {
            string project = CreateProject("guid-stability");
            try
            {
                using McpHost host = NewHost();
                ChangeSetStore store = new ChangeSetStore(host);
                AssetTools.Register(host, store);
                string session = NewEditingSession(host);

                const string customGuid = "11111111111111112222222222222222";
                JsonNode first = host.CallTool(
                    session,
                    "assets.import",
                    new JsonObject
                    {
                        ["projectPath"] = project,
                        ["path"] = "Assets/Materials/Custom.mat",
                        ["text"] = "%YAML 1.1\n--- !u!21 &2100000\nMaterial:\n  m_Name: Custom\n",
                        ["guid"] = customGuid,
                    });
                Check.Equal(
                    "assets: custom guid is used",
                    customGuid,
                    first["content"]?["guid"]?.GetValue<string>() ?? string.Empty);

                JsonNode second = host.CallTool(
                    session,
                    "assets.import",
                    new JsonObject
                    {
                        ["projectPath"] = project,
                        ["path"] = "Assets/Materials/Custom.mat",
                        ["text"] = "%YAML 1.1\n--- !u!21 &2100000\nMaterial:\n  m_Name: CustomUpdated\n",
                    });
                Check.Equal(
                    "assets: overwrite without guid reuses existing meta guid",
                    customGuid,
                    second["content"]?["guid"]?.GetValue<string>() ?? string.Empty);
            }
            finally
            {
                DeleteDirectory(project);
            }
        }

        private static void RecordsAuditAndChangeSetAndRejectRestoresWorkspaceClean()
        {
            string project = CreateProject("changeset-reject");
            try
            {
                using McpHost host = new McpHost(new FixedClock(5_000), PersonCredential.FromToken("person-token"));
                ChangeSetStore store = new ChangeSetStore(host);
                AssetTools.Register(host, store);
                ChangeReviewTools.Register(host, store);
                host.OpenProject(project);

                string agentSession = host.CreateSession(new SessionIdentity("agent-1", "stdio", "person-1"));
                host.GrantScope(agentSession, Scope.EditContent, GrantAuthority.Person);
                string personSession = host.CreateSession(new SessionIdentity("person-ui", "http", "person-1"));

                JsonNode imported = host.CallTool(
                    agentSession,
                    "assets.import",
                    new JsonObject
                    {
                        ["projectPath"] = project,
                        ["path"] = "Assets/Audio/BonusChime.wav",
                        ["base64"] = Convert.ToBase64String(MinimalWavBytes),
                    });
                string changeId = imported["content"]?["changeId"]?.GetValue<string>() ?? string.Empty;
                Check.Equal("assets: changeId is cs-1", "cs-1", changeId);
                Check.Equal("assets: one change set held", 1, store.All.Count);

                bool hasAssetPath = false;
                bool hasMetaPath = false;
                foreach (JsonObject op in store.All[0].Operations)
                {
                    string opPath = op["path"]?.GetValue<string>() ?? string.Empty;
                    if (string.Equals(opPath, "Assets/Audio/BonusChime.wav", StringComparison.Ordinal))
                    {
                        hasAssetPath = true;
                    }
                    else if (string.Equals(opPath, "Assets/Audio/BonusChime.wav.meta", StringComparison.Ordinal))
                    {
                        hasMetaPath = true;
                    }
                }

                Check.That("assets: change set tracks asset path", hasAssetPath);
                Check.That("assets: change set tracks meta path", hasMetaPath);

                bool auditedToolCall = false;
                foreach (AuditRecord record in host.Audit.Records)
                {
                    if (record.TargetKind == AuditTargetKind.Tool
                        && string.Equals(record.Target, "assets.import", StringComparison.Ordinal)
                        && record.Outcome is AuditOutcome.Allowed
                        && record.AffectedIdentities.Count == 1
                        && string.Equals(record.AffectedIdentities[0], changeId, StringComparison.Ordinal))
                    {
                        auditedToolCall = true;
                    }
                }

                Check.That("assets: audit log records allowed assets.import tool call with changeId", auditedToolCall);

                string wavFile = Path.Combine(project, "Assets", "Audio", "BonusChime.wav");
                string wavMetaFile = wavFile + ".meta";
                Check.That("assets: wav exists before reject", File.Exists(wavFile));
                Check.That("assets: wav meta exists before reject", File.Exists(wavMetaFile));

                JsonNode rejected = host.CallTool(
                    personSession,
                    "changes.reject",
                    new JsonObject
                    {
                        ["id"] = changeId,
                        ["reason"] = "revert imported asset",
                        ["personCredential"] = "person-token",
                    });
                JsonObject restore = (JsonObject)rejected["content"]!["restore"]!;
                Check.Equal("assets: reject restores both asset and meta", 2, restore["restoredCount"]?.GetValue<int>() ?? 0);
                Check.That("assets: wav deleted after reject", !File.Exists(wavFile));
                Check.That("assets: wav meta deleted after reject", !File.Exists(wavMetaFile));
                Check.That(
                    "assets: empty Assets/Audio directory pruned after reject",
                    !Directory.Exists(Path.Combine(project, "Assets", "Audio")));
            }
            finally
            {
                DeleteDirectory(project);
            }
        }

        private static void RestoresOverwrittenBinaryAndTextAssetsOnReject()
        {
            string project = CreateProject("changeset-overwrite");
            try
            {
                using McpHost host = new McpHost(new FixedClock(6_000), PersonCredential.FromToken("person-token"));
                ChangeSetStore store = new ChangeSetStore(host);
                AssetTools.Register(host, store);
                ChangeReviewTools.Register(host, store);
                host.OpenProject(project);

                string agentSession = host.CreateSession(new SessionIdentity("agent-1", "stdio", "person-1"));
                host.GrantScope(agentSession, Scope.EditContent, GrantAuthority.Person);
                string personSession = host.CreateSession(new SessionIdentity("person-ui", "http", "person-1"));

                string audioDir = Path.Combine(project, "Assets", "Audio");
                Directory.CreateDirectory(audioDir);
                string wavPath = Path.Combine(audioDir, "Existing.wav");
                string wavMetaPath = wavPath + ".meta";
                byte[] initialBytes = new byte[] { 1, 2, 3, 4 };
                string initialMeta = "fileFormatVersion: 2\nguid: 0123456789abcdef0123456789abcdef\nAudioImporter:\n";
                File.WriteAllBytes(wavPath, initialBytes);
                File.WriteAllText(wavMetaPath, initialMeta);

                JsonNode overwritten = host.CallTool(
                    agentSession,
                    "assets.import",
                    new JsonObject
                    {
                        ["projectPath"] = project,
                        ["path"] = "Assets/Audio/Existing.wav",
                        ["base64"] = Convert.ToBase64String(MinimalWavBytes),
                    });
                string changeId = overwritten["content"]?["changeId"]?.GetValue<string>() ?? string.Empty;
                Check.That("assets: overwritten wav holds new bytes", ByteArraysEqual(MinimalWavBytes, File.ReadAllBytes(wavPath)));

                host.CallTool(
                    personSession,
                    "changes.reject",
                    new JsonObject
                    {
                        ["id"] = changeId,
                        ["reason"] = "restore original wav",
                        ["personCredential"] = "person-token",
                    });
                Check.That("assets: reject restores original binary bytes", ByteArraysEqual(initialBytes, File.ReadAllBytes(wavPath)));
                Check.Equal("assets: reject restores original meta text", initialMeta, File.ReadAllText(wavMetaPath));
            }
            finally
            {
                DeleteDirectory(project);
            }
        }

        private static void RefusesPathEscapeOutsideAssetsAndInvalidPayloads()
        {
            string project = CreateProject("validation");
            try
            {
                using McpHost host = NewHost();
                ChangeSetStore store = new ChangeSetStore(host);
                AssetTools.Register(host, store);
                string session = NewEditingSession(host);

                ExpectFailure(
                    host,
                    session,
                    new JsonObject
                    {
                        ["projectPath"] = project,
                        ["path"] = "../escape.mat",
                        ["text"] = "content",
                    },
                    "../escape.mat",
                    "assets: refuses parent traversal path");

                ExpectFailure(
                    host,
                    session,
                    new JsonObject
                    {
                        ["projectPath"] = project,
                        ["path"] = "ProjectSettings/TimeManager.asset",
                        ["text"] = "content",
                    },
                    "ProjectSettings/TimeManager.asset",
                    "assets: refuses path outside Assets/");

                ExpectFailure(
                    host,
                    session,
                    new JsonObject
                    {
                        ["projectPath"] = project,
                        ["path"] = Path.Combine(project, "Assets", "Absolute.mat"),
                        ["text"] = "content",
                    },
                    "Assets",
                    "assets: refuses absolute path");

                ExpectFailure(
                    host,
                    session,
                    new JsonObject
                    {
                        ["projectPath"] = project,
                        ["path"] = "Assets/Materials/Broken.mat.meta",
                        ["text"] = "content",
                    },
                    "Assets/Materials/Broken.mat.meta",
                    "assets: refuses direct .meta target path");

                ExpectFailure(
                    host,
                    session,
                    new JsonObject
                    {
                        ["projectPath"] = project,
                        ["path"] = "Assets/Materials/Neither.mat",
                    },
                    "Assets/Materials/Neither.mat",
                    "assets: refuses missing text and base64");

                ExpectFailure(
                    host,
                    session,
                    new JsonObject
                    {
                        ["projectPath"] = project,
                        ["path"] = "Assets/Materials/Both.mat",
                        ["text"] = "hello",
                        ["base64"] = Convert.ToBase64String(Encoding.UTF8.GetBytes("hello")),
                    },
                    "Assets/Materials/Both.mat",
                    "assets: refuses both text and base64");

                ExpectFailure(
                    host,
                    session,
                    new JsonObject
                    {
                        ["projectPath"] = project,
                        ["path"] = "Assets/Audio/BadBase64.wav",
                        ["base64"] = "%%%not-valid-base64%%%",
                    },
                    "Assets/Audio/BadBase64.wav",
                    "assets: refuses invalid base64 payload");

                ExpectFailure(
                    host,
                    session,
                    new JsonObject
                    {
                        ["projectPath"] = project,
                        ["path"] = "Assets/Materials/BadGuid.mat",
                        ["text"] = "hello",
                        ["guid"] = "NOT_A_32_HEX_GUID",
                    },
                    "Assets/Materials/BadGuid.mat",
                    "assets: refuses malformed guid");

                Check.Equal("assets: failed imports record no change sets", 0, store.All.Count);
            }
            finally
            {
                DeleteDirectory(project);
            }
        }

        private static void ExpectFailure(
            McpHost host,
            string session,
            JsonObject arguments,
            string expectedPathSubstring,
            string label)
        {
            McpError? caught = null;
            try
            {
                host.CallTool(session, "assets.import", arguments);
            }
            catch (McpError error)
            {
                caught = error;
            }

            Check.That(
                label,
                caught != null && caught.Message.Contains(expectedPathSubstring, StringComparison.Ordinal));
        }

        private static bool IsValidLowerHexGuid(string guid)
        {
            if (guid.Length != 32)
            {
                return false;
            }

            foreach (char ch in guid)
            {
                bool isDigit = ch >= '0' && ch <= '9';
                bool isLowerHex = ch >= 'a' && ch <= 'f';
                if (!isDigit && !isLowerHex)
                {
                    return false;
                }
            }

            return true;
        }

        private static bool ByteArraysEqual(byte[] left, byte[] right)
        {
            if (left.Length != right.Length)
            {
                return false;
            }

            for (int index = 0; index < left.Length; index++)
            {
                if (left[index] != right[index])
                {
                    return false;
                }
            }

            return true;
        }

        private static byte[] CreateMinimalWav()
        {
            const int sampleRate = 8000;
            const short channels = 1;
            const short bitsPerSample = 16;
            const int sampleCount = 16;
            const int dataBytes = sampleCount * channels * (bitsPerSample / 8);
            const int riffChunkSize = 36 + dataBytes;
            const int byteRate = sampleRate * channels * (bitsPerSample / 8);
            const short blockAlign = channels * (bitsPerSample / 8);

            using MemoryStream stream = new MemoryStream();
            using BinaryWriter writer = new BinaryWriter(stream, Encoding.ASCII, leaveOpen: true);
            writer.Write(Encoding.ASCII.GetBytes("RIFF"));
            writer.Write(riffChunkSize);
            writer.Write(Encoding.ASCII.GetBytes("WAVE"));
            writer.Write(Encoding.ASCII.GetBytes("fmt "));
            writer.Write(16);
            writer.Write((short)1);
            writer.Write(channels);
            writer.Write(sampleRate);
            writer.Write(byteRate);
            writer.Write(blockAlign);
            writer.Write(bitsPerSample);
            writer.Write(Encoding.ASCII.GetBytes("data"));
            writer.Write(dataBytes);
            for (int index = 0; index < sampleCount; index++)
            {
                short sample = (short)(index % 2 == 0 ? 1200 : -1200);
                writer.Write(sample);
            }

            writer.Flush();
            return stream.ToArray();
        }

        private static McpHost NewHost()
        {
            return new McpHost(new FixedClock(1_700_000_000_000L));
        }

        private static string NewEditingSession(McpHost host)
        {
            string session = host.CreateSession(new SessionIdentity("agent-1", "stdio", "stdio-local"));
            host.GrantScope(session, Scope.ReadProject, GrantAuthority.Person);
            host.GrantScope(session, Scope.EditContent, GrantAuthority.Person);
            return session;
        }

        private static string CreateProject(string label)
        {
            string root = Path.Combine(
                Path.GetTempPath(),
                "unitas-u3-assets-" + label + "-" + Guid.NewGuid().ToString("N"));
            Directory.CreateDirectory(Path.Combine(root, "Assets"));
            return root;
        }

        private static void DeleteDirectory(string directory)
        {
            if (Directory.Exists(directory))
            {
                Directory.Delete(directory, true);
            }
        }
    }
}
