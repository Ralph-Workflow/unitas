using System;
using System.Collections.Generic;

namespace Unitas.Editor.Mcp
{
    /// <summary>One reversible editor operation: the op that was applied and the op that undoes it.</summary>
    public sealed class UndoEntry<TOp>
    {
        public UndoEntry(TOp forward, TOp inverse)
        {
            Forward = forward;
            Inverse = inverse;
        }

        public TOp Forward { get; }

        public TOp Inverse { get; }
    }

    /// <summary>Outcome of an undo or redo request. An empty stack is a structured error, never a throw.</summary>
    public sealed class UndoResult<TOp>
    {
        public UndoResult(bool ok, TOp applied, string error)
        {
            Ok = ok;
            Applied = applied;
            Error = error;
        }

        public bool Ok { get; }

        public TOp Applied { get; }

        public string Error { get; }
    }

    /// <summary>
    /// Per-document undo stack. The apply delegate is the document's own Apply, which returns the
    /// inverse op, so undo and redo restore the previous state exactly.
    /// </summary>
    public sealed class UndoHistory<TOp>
    {
        private readonly Func<TOp, TOp> _apply;
        private readonly List<UndoEntry<TOp>> _undo = new List<UndoEntry<TOp>>();
        private readonly List<UndoEntry<TOp>> _redo = new List<UndoEntry<TOp>>();

        public UndoHistory(Func<TOp, TOp> apply)
        {
            _apply = apply ?? throw new ArgumentNullException(nameof(apply));
        }

        public bool CanUndo
        {
            get { return _undo.Count > 0; }
        }

        public bool CanRedo
        {
            get { return _redo.Count > 0; }
        }

        public int UndoCount
        {
            get { return _undo.Count; }
        }

        public int RedoCount
        {
            get { return _redo.Count; }
        }

        public void Push(TOp op, TOp inverse)
        {
            _undo.Add(new UndoEntry<TOp>(op, inverse));
            _redo.Clear();
        }

        public UndoResult<TOp> Undo()
        {
            if (_undo.Count == 0)
            {
                return new UndoResult<TOp>(false, default!, "nothing to undo");
            }

            UndoEntry<TOp> entry = _undo[_undo.Count - 1];
            _undo.RemoveAt(_undo.Count - 1);
            TOp applied = _apply(entry.Inverse);
            _redo.Add(new UndoEntry<TOp>(applied, default!));
            return new UndoResult<TOp>(true, applied, string.Empty);
        }

        public UndoResult<TOp> Redo()
        {
            if (_redo.Count == 0)
            {
                return new UndoResult<TOp>(false, default!, "nothing to redo");
            }

            UndoEntry<TOp> entry = _redo[_redo.Count - 1];
            _redo.RemoveAt(_redo.Count - 1);
            TOp applied = _apply(entry.Forward);
            _undo.Add(new UndoEntry<TOp>(entry.Forward, applied));
            return new UndoResult<TOp>(true, applied, string.Empty);
        }
    }
}
