using System;
using System.IO;
using System.Security.Cryptography;

namespace Unitas.Editor.Mcp
{
    public interface IPersonCredential
    {
        bool IsPresent { get; }

        bool Verify(string? candidateToken, string? transportName);

        void RequirePerson(string? candidateToken, string? transportName, string actionName);
    }

    public sealed class PersonCredential : IPersonCredential
    {
        public const string EnvironmentVariableName = "UNITAS_PERSON_CREDENTIAL";
        public const string CliFlagName = "--person-credential";
        public const string StdioTransportName = "stdio";
        public const string CliTransportName = "cli";

        private readonly byte[]? expectedTokenBytes;

        private PersonCredential(string? secretToken)
        {
            if (!string.IsNullOrWhiteSpace(secretToken))
            {
                expectedTokenBytes = System.Text.Encoding.UTF8.GetBytes(secretToken.Trim());
            }
        }

        public static PersonCredential Absent { get; } = new PersonCredential(null);

        public bool IsPresent => expectedTokenBytes != null && expectedTokenBytes.Length > 0;

        public static PersonCredential FromToken(string secretToken)
        {
            if (string.IsNullOrWhiteSpace(secretToken))
            {
                throw new ArgumentException("Person credential token must not be empty.", nameof(secretToken));
            }

            return new PersonCredential(secretToken);
        }

        public static PersonCredential FromStartup(string[]? args, TextReader? interactivePrompt = null)
        {
            if (args != null)
            {
                for (int i = 0; i < args.Length - 1; i++)
                {
                    if (string.Equals(args[i], CliFlagName, StringComparison.Ordinal)
                        && !string.IsNullOrWhiteSpace(args[i + 1]))
                    {
                        return new PersonCredential(args[i + 1]);
                    }
                }
            }

            string? envToken = Environment.GetEnvironmentVariable(EnvironmentVariableName);
            if (!string.IsNullOrWhiteSpace(envToken))
            {
                return new PersonCredential(envToken);
            }

            if (interactivePrompt != null)
            {
                string? line = interactivePrompt.ReadLine();
                if (!string.IsNullOrWhiteSpace(line))
                {
                    return new PersonCredential(line);
                }
            }

            return Absent;
        }

        public static bool IsInteractiveTransport(string? transportName)
        {
            if (string.IsNullOrWhiteSpace(transportName))
            {
                return true;
            }

            if (string.Equals(transportName, StdioTransportName, StringComparison.OrdinalIgnoreCase)
                || string.Equals(transportName, CliTransportName, StringComparison.OrdinalIgnoreCase))
            {
                return false;
            }

            return true;
        }

        public bool Verify(string? candidateToken, string? transportName)
        {
            if (!IsPresent || string.IsNullOrWhiteSpace(candidateToken))
            {
                return false;
            }

            if (!IsInteractiveTransport(transportName))
            {
                return false;
            }

            byte[] candidateBytes = System.Text.Encoding.UTF8.GetBytes(candidateToken.Trim());
            return CryptographicOperations.FixedTimeEquals(expectedTokenBytes!, candidateBytes);
        }

        public void RequirePerson(string? candidateToken, string? transportName, string actionName)
        {
            ArgumentNullException.ThrowIfNull(actionName);
            if (!IsInteractiveTransport(transportName))
            {
                throw new McpError.PersonCredentialRequired(
                    actionName,
                    "requests over " + transportName + " are never treated as a person's (AUT-007)");
            }

            if (!IsPresent)
            {
                throw new McpError.NoPersonPresent(actionName);
            }

            if (!Verify(candidateToken, transportName))
            {
                throw new McpError.PersonCredentialRequired(
                    actionName,
                    "a valid person credential is required for " + actionName + " (AUT-007)");
            }
        }
    }
}
