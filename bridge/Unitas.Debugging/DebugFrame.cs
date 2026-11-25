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
        {
            SequencePointId = sequencePointId;
            Method = method ?? string.Empty;
            File = file ?? string.Empty;
            Line = line;
            LocalNames = localNames ?? Array.Empty<string>();
            Locals = locals ?? Array.Empty<object?>();
            Depth = depth < 0 ? 0 : depth;
        }

        public int SequencePointId { get; }
        public string Method { get; }
        public string File { get; }
        public int Line { get; }
        public IReadOnlyList<string> LocalNames { get; }
        public object?[] Locals { get; }
        public int Depth { get; }
    }
}
