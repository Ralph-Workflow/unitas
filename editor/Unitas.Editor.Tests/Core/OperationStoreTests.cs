using System.Text.Json.Nodes;
using Unitas.Editor.Mcp;

namespace Unitas.Editor.Tests.Core
{
    public static class OperationStoreTests
    {
        public static void Run()
        {
            OperationLifecycleTransitionsFromPendingToCompleted();
            OperationCanTransitionToFailedAndUnknownIsRejected();
        }

        private static void OperationLifecycleTransitionsFromPendingToCompleted()
        {
            OperationStore store = new OperationStore();
            string id = store.StartWithStage("build-player", "compiling");
            OperationHandle initial = store.Get(id);
            Check.Equal(
                "operation_lifecycle_transitions_from_pending_to_completed: initial status",
                "pending",
                initial.State.StatusString);
            Check.That(
                "operation_lifecycle_transitions_from_pending_to_completed: initial stage",
                initial.State is OperationState.Pending pendingInitial && pendingInitial.Stage == "compiling");
            store.UpdateStage(id, "packaging");
            Check.That(
                "operation_lifecycle_transitions_from_pending_to_completed: updated stage",
                store.Get(id).State is OperationState.Pending pendingUpdated && pendingUpdated.Stage == "packaging");
            store.Complete(id, JsonNode.Parse("{\"artifact\":\"App.app\"}"));
            OperationHandle completed = store.Get(id);
            Check.That(
                "operation_lifecycle_transitions_from_pending_to_completed: completed state",
                completed.State is OperationState.Completed);
            Check.Equal(
                "operation_lifecycle_transitions_from_pending_to_completed: completed result",
                "{\"artifact\":\"App.app\"}",
                ((OperationState.Completed)completed.State).Result!.ToJsonString());

            bool refused;
            try
            {
                store.Fail(id, "too late");
                refused = false;
            }
            catch (McpError.OperationNotPending)
            {
                refused = true;
            }

            Check.That(
                "operation_lifecycle_transitions_from_pending_to_completed: completed operation rejects fail",
                refused);
        }

        private static void OperationCanTransitionToFailedAndUnknownIsRejected()
        {
            OperationStore store = new OperationStore();
            string id = store.Start("import-assets");
            store.Fail(id, "corrupt asset");
            OperationHandle failed = store.Get(id);
            Check.That(
                "operation_can_transition_to_failed_and_unknown_is_rejected: failed state",
                failed.State is OperationState.Failed);
            Check.Equal(
                "operation_can_transition_to_failed_and_unknown_is_rejected: failure reason",
                "corrupt asset",
                ((OperationState.Failed)failed.State).Error);

            bool rejected;
            try
            {
                store.Get("op-999");
                rejected = false;
            }
            catch (McpError.UnknownOperation)
            {
                rejected = true;
            }

            Check.That(
                "operation_can_transition_to_failed_and_unknown_is_rejected: unknown id rejected",
                rejected);
        }
    }
}
