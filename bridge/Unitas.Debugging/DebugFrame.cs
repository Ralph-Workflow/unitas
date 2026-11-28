using System;
using System.Collections.Generic;

namespace Unitas.Debugging
{
    public sealed class DebugFrame
    {
        public DebugFrame(int sequencePointId, string method, string file, int line, IReadOnlyList<string> localNames, object?[] locals)
            : this(sequencePointId, method, file, line, localNames, locals, 0)
        {
        }

        public DebugFrame(int sequencePointId, string method, string file, int line, IReadOnlyList<string> localNames, object?[] locals, int depth)
            : this(sequencePointId, method, file, line, localNames, locals, depth, null, false)
        {
        }

        public DebugFrame(
            int sequencePointId,
            string method,
            string file,
            int line,
            IReadOnlyList<string> localNames,
            object?[] locals,
            int depth,
            Exception? exception,
            bool isExceptionUnhandled)
        {
            SequencePointId = sequencePointId;
            Method = method ?? string.Empty;
            File = file ?? string.Empty;
            Line = line;
            LocalNames = localNames ?? Array.Empty<string>();
            Locals = locals ?? Array.Empty<object?>();
            Depth = depth < 0 ? 0 : depth;
            Exception = exception;
            IsExceptionUnhandled = isExceptionUnhandled;
        }

        public int SequencePointId { get; }
        public string Method { get; }
        public string File { get; }
        public int Line { get; }
        public IReadOnlyList<string> LocalNames { get; }
        public object?[] Locals { get; }
        public int Depth { get; }
        public Exception? Exception { get; }
        public bool IsExceptionUnhandled { get; }

        public string ExceptionTypeName
        {
            get
            {
                if (Exception == null)
                {
                    return string.Empty;
                }
                string rendered = Exception.ToString();
                int colon = rendered.IndexOf(':', StringComparison.Ordinal);
                int newline = rendered.IndexOf('\n', StringComparison.Ordinal);
                int end = colon >= 0 && (newline < 0 || colon < newline) ? colon : newline;
                string head = end >= 0 ? rendered.Substring(0, end).Trim() : rendered.Trim();
                int arrow = head.IndexOf("--->", StringComparison.Ordinal);
                if (arrow > 0)
                {
                    head = head.Substring(0, arrow).Trim();
                }
                return string.IsNullOrEmpty(head) ? nameof(System.Exception) : head;
            }
        }
    }
}
