using Unitas.Editor.Mcp;

namespace Unitas.Editor.Tests.Core
{
    public static class UndoHistoryTests
    {
        public static void Run()
        {
            UndoRestoresThePreviousValue();
            RedoReappliesTheUndoneValue();
            NewEditClearsTheRedoStack();
            EmptyStackReturnsAStructuredError();
        }

        private static void UndoRestoresThePreviousValue()
        {
            FakeDoc doc = new FakeDoc();
            UndoHistory<FakeOp> history = new UndoHistory<FakeOp>(op => Apply(doc, op));
            Edit(history, doc, "first");

            Check.That("undo_history_has_something_to_undo", history.CanUndo);
            UndoResult<FakeOp> undone = history.Undo();

            Check.That("undo_history_undo_reports_success", undone.Ok);
            Check.Equal("undo_history_undo_restores_the_previous_value", "start", doc.Value);
            Check.That("undo_history_redo_becomes_available", history.CanRedo);
        }

        private static void RedoReappliesTheUndoneValue()
        {
            FakeDoc doc = new FakeDoc();
            UndoHistory<FakeOp> history = new UndoHistory<FakeOp>(op => Apply(doc, op));
            Edit(history, doc, "first");
            history.Undo();

            UndoResult<FakeOp> redone = history.Redo();

            Check.That("undo_history_redo_reports_success", redone.Ok);
            Check.Equal("undo_history_redo_reapplies_the_value", "first", doc.Value);
            Check.That("undo_history_undo_is_available_again", history.CanUndo);
        }

        private static void NewEditClearsTheRedoStack()
        {
            FakeDoc doc = new FakeDoc();
            UndoHistory<FakeOp> history = new UndoHistory<FakeOp>(op => Apply(doc, op));
            Edit(history, doc, "first");
            history.Undo();

            Edit(history, doc, "second");

            Check.That("undo_history_new_edit_clears_redo", !history.CanRedo);
            Check.Equal("undo_history_new_edit_keeps_the_new_value", "second", doc.Value);
        }

        private static void EmptyStackReturnsAStructuredError()
        {
            FakeDoc doc = new FakeDoc();
            UndoHistory<FakeOp> history = new UndoHistory<FakeOp>(op => Apply(doc, op));

            UndoResult<FakeOp> undone = history.Undo();
            UndoResult<FakeOp> redone = history.Redo();

            Check.That("undo_history_empty_undo_is_not_ok", !undone.Ok);
            Check.That("undo_history_empty_undo_reports_an_error", undone.Error.Length > 0);
            Check.That("undo_history_empty_redo_is_not_ok", !redone.Ok);
            Check.That("undo_history_empty_redo_reports_an_error", redone.Error.Length > 0);
            Check.Equal("undo_history_empty_stack_leaves_the_document_alone", "start", doc.Value);
        }

        private static void Edit(UndoHistory<FakeOp> history, FakeDoc doc, string value)
        {
            FakeOp op = new FakeOp(value);
            history.Push(op, Apply(doc, op));
        }

        private static FakeOp Apply(FakeDoc doc, FakeOp op)
        {
            string previous = doc.Value;
            doc.Value = op.Value;
            return new FakeOp(previous);
        }

        private sealed class FakeDoc
        {
            public string Value = "start";
        }

        private sealed class FakeOp
        {
            public FakeOp(string value)
            {
                Value = value;
            }

            public string Value { get; }
        }
    }
}
