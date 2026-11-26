using System;
using System.Collections.Generic;
using System.Globalization;
using UnityEngine;

namespace Unitas.PlayerHost
{
    internal static class PlayerHostHud
    {
        private const string CollectedMarker = "Pickup collected score ";
        private const string PickupsMarker = " pickups ";
        private const string WonMarker = "pickups collected, press R to play again";

        public static void Read(out int score, out int total, out bool won, out string text)
        {
            score = 0;
            total = 0;
            won = false;
            text = string.Empty;
            IReadOnlyList<LogEntry> entries = LogRecorder.Entries;
            for (int index = entries.Count - 1; index >= 0; index--)
            {
                LogEntry entry = entries[index];
                if (entry.Origin == null)
                {
                    continue;
                }

                string message = entry.Message;
                if (text.Length == 0)
                {
                    text = message;
                }

                if (score == 0 && TryReadNumberAfter(message, CollectedMarker, out int collected))
                {
                    score = collected;
                }

                if (total == 0)
                {
                    total = ReadTotal(message);
                }

                if (message.Contains(WonMarker, StringComparison.Ordinal))
                {
                    won = true;
                }
            }
        }

        private static int ReadTotal(string message)
        {
            if (message.Contains(WonMarker, StringComparison.Ordinal)
                && TryReadNumberBefore(message, " pickups collected", out int wonTotal))
            {
                return wonTotal;
            }

            if (TryReadNumberAfter(message, PickupsMarker, out int pickups))
            {
                return pickups;
            }

            return 0;
        }

        private static bool TryReadNumberAfter(string message, string marker, out int value)
        {
            value = 0;
            int markerIndex = message.IndexOf(marker, StringComparison.Ordinal);
            if (markerIndex < 0)
            {
                return false;
            }

            return TryReadLeadingNumber(message, markerIndex + marker.Length, out value);
        }

        private static bool TryReadNumberBefore(string message, string marker, out int value)
        {
            value = 0;
            int markerIndex = message.IndexOf(marker, StringComparison.Ordinal);
            if (markerIndex <= 0)
            {
                return false;
            }

            int start = markerIndex;
            while (start > 0 && char.IsDigit(message[start - 1]))
            {
                start--;
            }

            return int.TryParse(
                message.AsSpan(start, markerIndex - start),
                NumberStyles.None,
                CultureInfo.InvariantCulture,
                out value);
        }

        private static bool TryReadLeadingNumber(string message, int start, out int value)
        {
            value = 0;
            int end = start;
            while (end < message.Length && char.IsDigit(message[end]))
            {
                end++;
            }

            if (end == start)
            {
                return false;
            }

            return int.TryParse(
                message.AsSpan(start, end - start),
                NumberStyles.None,
                CultureInfo.InvariantCulture,
                out value);
        }
    }
}
