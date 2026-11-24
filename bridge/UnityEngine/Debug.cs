using System;

namespace UnityEngine
{
    public static class Debug
    {
        public static bool isDebugBuild => Application.isDevelopmentBuild;

        public static void Log(string message)
        {
            Record(LogSeverity.Info, message);
            Console.WriteLine(message);
        }

        public static void LogWarning(string message)
        {
            Record(LogSeverity.Warning, message);
            Console.WriteLine(message);
        }

        public static void LogError(string message)
        {
            Record(LogSeverity.Error, message);
            Console.Error.WriteLine(message);
        }

        public static void LogException(Exception exception, Object? context = null)
        {
            if (exception == null)
            {
                throw new ArgumentNullException(nameof(exception));
            }

            string stack = exception.StackTrace ?? new System.Diagnostics.StackTrace(true).ToString();
            LogEntry entry = new LogEntry(
                LogSeverity.Error,
                Time.frameCount,
                Time.time,
                exception.Message,
                stack,
                context ?? LogRecorder.CurrentOrigin);
            LogRecorder.Append(entry);
            Console.Error.WriteLine(exception);
        }

        private static void Record(LogSeverity severity, string message)
        {
            LogEntry entry = new LogEntry(
                severity,
                Time.frameCount,
                Time.time,
                message,
                new System.Diagnostics.StackTrace(true).ToString(),
                LogRecorder.CurrentOrigin);
            LogRecorder.Append(entry);
        }
    }
}
