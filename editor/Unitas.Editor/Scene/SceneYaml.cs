using System;
using System.Collections.Generic;
using System.Globalization;
using System.Text.RegularExpressions;

namespace Unitas.Editor.Scene
{
    /// <summary>Line and header helpers for Unity tagged YAML; every line keeps its own bytes.</summary>
    internal static class SceneYaml
    {
        private static readonly Regex HeaderPattern = new Regex(
            @"^--- !u!(?<class>-?\d+) &(?<file>-?\d+)( stripped)?$",
            RegexOptions.Compiled | RegexOptions.CultureInvariant);

        /// <summary>Splits text into lines that keep their line terminator, so the text rebuilds byte for byte.</summary>
        public static List<string> SplitLines(string text)
        {
            List<string> lines = new List<string>();
            int start = 0;
            for (int index = 0; index < text.Length; index++)
            {
                if (text[index] != '\n')
                {
                    continue;
                }

                lines.Add(text[start..(index + 1)]);
                start = index + 1;
            }

            if (start < text.Length)
            {
                lines.Add(text.Substring(start));
            }

            return lines;
        }

        public static string Content(string line)
        {
            if (line.EndsWith("\r\n", StringComparison.Ordinal))
            {
                return line[..^2];
            }

            if (line.EndsWith('\n'))
            {
                return line[..^1];
            }

            return line;
        }

        public static string Terminator(string line)
        {
            if (line.EndsWith("\r\n", StringComparison.Ordinal))
            {
                return "\r\n";
            }

            if (line.EndsWith('\n'))
            {
                return "\n";
            }

            return string.Empty;
        }

        public static int Indent(string content)
        {
            int indent = 0;
            while (indent < content.Length && content[indent] == ' ')
            {
                indent++;
            }

            return indent;
        }

        public static int CountLeadingSpaces(string value)
        {
            int indent = 0;
            while (indent < value.Length && (value[indent] == ' ' || value[indent] == '\t'))
            {
                indent++;
            }

            return indent;
        }

        /// <summary>True for any line that starts a YAML document, valid or not.</summary>
        public static bool IsHeaderStart(string line)
        {
            return Content(line).StartsWith("---", StringComparison.Ordinal);
        }

        public static bool TryParseHeader(string content, out int classId, out long fileId, out bool stripped)
        {
            classId = 0;
            fileId = 0;
            stripped = false;
            Match match = HeaderPattern.Match(content.TrimEnd());
            if (!match.Success)
            {
                return false;
            }

            classId = int.Parse(match.Groups["class"].Value, CultureInfo.InvariantCulture);
            fileId = long.Parse(match.Groups["file"].Value, CultureInfo.InvariantCulture);
            stripped = match.Groups["stripped"].Success;
            return true;
        }
    }
}
