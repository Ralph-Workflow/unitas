using System;
using System.Globalization;

namespace UnityEngine
{
    public static class Debug
    {
        private const string DefaultAssertionMessage = "Assertion failed";

        public static bool isDebugBuild => Application.isDevelopmentBuild;

        public static void Log(string message)
        {
            Record(LogSeverity.Info, message, LogRecorder.CurrentOrigin);
            Console.WriteLine(message);
        }

        public static void Log(string message, Object? context)
        {
            LogWithContext(LogSeverity.Info, message, context);
        }

        public static void LogWarning(string message)
        {
            Record(LogSeverity.Warning, message, LogRecorder.CurrentOrigin);
            Console.WriteLine(message);
        }

        public static void LogWarning(string message, Object? context)
        {
            LogWithContext(LogSeverity.Warning, message, context);
        }

        public static void LogError(string message)
        {
            Record(LogSeverity.Error, message, LogRecorder.CurrentOrigin);
            Console.Error.WriteLine(message);
        }

        public static void LogError(string message, Object? context)
        {
            LogWithContext(LogSeverity.Error, message, context);
        }

        public static void LogFormat(string format, params object?[] args)
        {
            Log(FormatMessage(format, args));
        }

        public static void LogFormat(Object? context, string format, params object?[] args)
        {
            Log(FormatMessage(format, args), context);
        }

        public static void LogWarningFormat(string format, params object?[] args)
        {
            LogWarning(FormatMessage(format, args));
        }

        public static void LogWarningFormat(Object? context, string format, params object?[] args)
        {
            LogWarning(FormatMessage(format, args), context);
        }

        public static void LogErrorFormat(string format, params object?[] args)
        {
            LogError(FormatMessage(format, args));
        }

        public static void LogErrorFormat(Object? context, string format, params object?[] args)
        {
            LogError(FormatMessage(format, args), context);
        }

        public static void Assert(bool condition)
        {
            if (!condition)
            {
                LogError(DefaultAssertionMessage);
            }
        }

        public static void Assert(bool condition, Object? context)
        {
            if (!condition)
            {
                LogError(DefaultAssertionMessage, context);
            }
        }

        public static void Assert(bool condition, string message)
        {
            if (!condition)
            {
                LogError(message);
            }
        }

        public static void Assert(bool condition, string message, Object? context)
        {
            if (!condition)
            {
                LogError(message, context);
            }
        }

        public static void AssertFormat(bool condition, string format, params object?[] args)
        {
            if (!condition)
            {
                LogError(FormatMessage(format, args));
            }
        }

        public static void AssertFormat(bool condition, Object? context, string format, params object?[] args)
        {
            if (!condition)
            {
                LogError(FormatMessage(format, args), context);
            }
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

        private static string FormatMessage(string format, object?[] args)
        {
            if (format == null)
            {
                throw new ArgumentNullException(nameof(format));
            }

            if (args == null)
            {
                throw new ArgumentNullException(nameof(args));
            }

            return string.Format(CultureInfo.InvariantCulture, format, args);
        }

        private static void LogWithContext(LogSeverity severity, string message, Object? context)
        {
            Object? previousOrigin = LogRecorder.CurrentOrigin;
            LogRecorder.CurrentOrigin = context;
            try
            {
                Record(severity, message, context);
                if (severity == LogSeverity.Error)
                {
                    Console.Error.WriteLine(message);
                }
                else
                {
                    Console.WriteLine(message);
                }
            }
            finally
            {
                LogRecorder.CurrentOrigin = previousOrigin;
            }
        }

        private static void Record(LogSeverity severity, string message, Object? origin)
        {
            LogEntry entry = new LogEntry(
                severity,
                Time.frameCount,
                Time.time,
                message,
                new System.Diagnostics.StackTrace(true).ToString(),
                origin);
            LogRecorder.Append(entry);
        }
    }
}
