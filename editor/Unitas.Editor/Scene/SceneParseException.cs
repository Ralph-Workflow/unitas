using System;
using System.Globalization;

namespace Unitas.Editor.Scene
{
    /// <summary>A scene file could not be read; the file and the 1-based line are named.</summary>
    public sealed class SceneParseException : Exception
    {
        public SceneParseException(string filePath, int line, string reason)
            : base(BuildMessage(filePath, line, reason))
        {
            FilePath = filePath ?? string.Empty;
            Line = line;
            Reason = reason ?? string.Empty;
        }

        public string FilePath { get; }

        /// <summary>The 1-based line of the problem, or 0 when the problem is not on one line.</summary>
        public int Line { get; }

        public string Reason { get; }

        private static string BuildMessage(string filePath, int line, string reason)
        {
            return (filePath ?? string.Empty)
                + ":"
                + line.ToString(CultureInfo.InvariantCulture)
                + ": "
                + (reason ?? string.Empty);
        }
    }
}
