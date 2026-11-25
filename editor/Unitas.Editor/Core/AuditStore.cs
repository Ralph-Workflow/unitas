using System;
using System.Collections.Generic;
using System.IO;
using System.Security.Cryptography;
using System.Text;
using System.Text.Json.Nodes;

namespace Unitas.Editor.Mcp
{
    public sealed class AuditStore
    {
        public const string StateDirectoryEnvironmentVariable = "UNITAS_STATE_DIR";

        public AuditStore(string auditFilePath, string? protectedProjectPath = null)
        {
            if (string.IsNullOrWhiteSpace(auditFilePath))
            {
                throw new McpError.InvalidParams("audit file path must not be empty");
            }

            AuditFilePath = Path.GetFullPath(auditFilePath);
            if (!string.IsNullOrWhiteSpace(protectedProjectPath))
            {
                ProtectedProjectPath = Path.GetFullPath(protectedProjectPath);
                EnsurePathOutsideProject(
                    AuditFilePath,
                    ProtectedProjectPath,
                    "audit log state directory must reside outside project content (AUT-008)");
            }
        }

        public string AuditFilePath { get; }

        public string? ProtectedProjectPath { get; }

        public static AuditStore ForProject(string projectPath, string? stateRootDir = null)
        {
            if (string.IsNullOrWhiteSpace(projectPath))
            {
                throw new McpError.InvalidParams("projectPath must not be empty");
            }

            string normalizedProjectPath = NormalizeDirectoryPath(projectPath);
            string resolvedStateRoot = ResolveStateRoot(stateRootDir);
            EnsurePathOutsideProject(
                resolvedStateRoot,
                normalizedProjectPath,
                "audit log state directory must reside outside project content (AUT-008)");

            string projectHash = ComputeProjectHash(normalizedProjectPath);
            string auditFilePath = Path.Combine(resolvedStateRoot, "projects", projectHash, "audit.jsonl");
            return new AuditStore(auditFilePath, normalizedProjectPath);
        }

        public static bool IsPathInsideDirectory(string candidatePath, string directoryPath)
        {
            string normalizedDirectory = NormalizeDirectoryPath(directoryPath);
            string normalizedCandidate = NormalizeDirectoryPath(candidatePath);
            if (string.Equals(normalizedCandidate, normalizedDirectory, StringComparison.Ordinal))
            {
                return true;
            }

            string directoryPrefix = normalizedDirectory + Path.DirectorySeparatorChar;
            return normalizedCandidate.StartsWith(directoryPrefix, StringComparison.Ordinal);
        }

        public static void EnsurePathOutsideProject(
            string candidatePath,
            string? protectedProjectPath,
            string errorMessage)
        {
            if (string.IsNullOrWhiteSpace(protectedProjectPath))
            {
                return;
            }

            if (IsPathInsideDirectory(candidatePath, protectedProjectPath))
            {
                throw new McpError.InvalidParams(errorMessage);
            }
        }

        public void Append(AuditRecord record)
        {
            ArgumentNullException.ThrowIfNull(record);
            try
            {
                string? parentDirectory = Path.GetDirectoryName(AuditFilePath);
                if (!string.IsNullOrEmpty(parentDirectory))
                {
                    Directory.CreateDirectory(parentDirectory);
                }

                string line = record.ToValue().ToJsonString() + "\n";
                File.AppendAllText(AuditFilePath, line, Encoding.UTF8);
            }
            catch (Exception error) when (error is IOException || error is UnauthorizedAccessException)
            {
                throw new McpError.Io(error);
            }
        }

        public IReadOnlyList<AuditRecord> LoadAll()
        {
            if (!File.Exists(AuditFilePath))
            {
                return Array.Empty<AuditRecord>();
            }

            string[] lines;
            try
            {
                lines = File.ReadAllLines(AuditFilePath, Encoding.UTF8);
            }
            catch (Exception error) when (error is IOException || error is UnauthorizedAccessException)
            {
                throw new McpError.Io(error);
            }

            List<AuditRecord> records = new List<AuditRecord>(lines.Length);
            foreach (string rawLine in lines)
            {
                if (string.IsNullOrWhiteSpace(rawLine))
                {
                    continue;
                }

                JsonNode? parsed;
                try
                {
                    parsed = JsonNode.Parse(rawLine);
                }
                catch (Exception error)
                {
                    throw new McpError.ParseError(error.Message);
                }

                if (!(parsed is JsonObject obj))
                {
                    throw new McpError.ParseError("audit record line must be a JSON object");
                }

                records.Add(AuditRecord.FromJson(obj));
            }

            return records;
        }

        private static string ResolveStateRoot(string? stateRootDir)
        {
            if (!string.IsNullOrWhiteSpace(stateRootDir))
            {
                return Path.GetFullPath(stateRootDir);
            }

            string? envDir = Environment.GetEnvironmentVariable(StateDirectoryEnvironmentVariable);
            if (!string.IsNullOrWhiteSpace(envDir))
            {
                return Path.GetFullPath(envDir);
            }

            string homeDir = Environment.GetFolderPath(Environment.SpecialFolder.UserProfile);
            if (string.IsNullOrWhiteSpace(homeDir))
            {
                homeDir = Path.GetTempPath();
            }

            return Path.GetFullPath(Path.Combine(homeDir, ".unitas"));
        }

        private static string NormalizeDirectoryPath(string path)
        {
            string full = Path.GetFullPath(path);
            return Path.TrimEndingDirectorySeparator(full);
        }

        private static string ComputeProjectHash(string normalizedProjectPath)
        {
            byte[] inputBytes = Encoding.UTF8.GetBytes(normalizedProjectPath);
            byte[] digest = SHA256.HashData(inputBytes);
            StringBuilder builder = new StringBuilder(16);
            for (int i = 0; i < 8; i++)
            {
                builder.Append(digest[i].ToString("x2", System.Globalization.CultureInfo.InvariantCulture));
            }

            return builder.ToString();
        }
    }
}
