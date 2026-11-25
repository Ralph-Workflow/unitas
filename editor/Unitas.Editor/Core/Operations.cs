using System.Collections.Generic;
using System.Text.Json.Nodes;

namespace Unitas.Editor.Mcp
{
    public abstract record OperationState
    {
        private OperationState()
        {
        }

        public abstract string StatusString { get; }

        public bool IsPending => this is Pending;

        public sealed record Pending(string? Stage) : OperationState
        {
            public override string StatusString => "pending";
        }

        public sealed record Completed(JsonNode? Result) : OperationState
        {
            public override string StatusString => "completed";
        }

        public sealed record Failed(string Error) : OperationState
        {
            public override string StatusString => "failed";
        }
    }

    public sealed record OperationHandle
    {
        internal OperationHandle(string id, string name, OperationState state)
        {
            Id = id;
            Name = name;
            State = state;
        }

        public string Id { get; }

        public string Name { get; }

        public OperationState State { get; internal set; }

        public JsonObject ToValue()
        {
            JsonObject value = new JsonObject
            {
                ["id"] = Id,
                ["operation"] = Name,
                ["status"] = State.StatusString,
            };
            switch (State)
            {
                case OperationState.Pending pending:
                    value["stage"] = pending.Stage;
                    break;
                case OperationState.Completed completed:
                    value["result"] = completed.Result?.DeepClone();
                    break;
                case OperationState.Failed failed:
                    value["error"] = failed.Error;
                    break;
            }

            return value;
        }
    }

    public sealed class OperationStore
    {
        private readonly Dictionary<string, OperationHandle> _operations = new Dictionary<string, OperationHandle>();
        private readonly List<OperationHandle> _order = new List<OperationHandle>();
        private long _nextId;

        public string Start(string name)
        {
            return StartWithStage(name, null);
        }

        public string StartWithStage(string name, string? stage)
        {
            _nextId++;
            string id = "op-" + _nextId;
            OperationHandle handle = new OperationHandle(id, name, new OperationState.Pending(stage));
            _operations.Add(id, handle);
            _order.Add(handle);
            return id;
        }

        public void UpdateStage(string id, string stage)
        {
            OperationHandle handle = PendingHandle(id);
            handle.State = new OperationState.Pending(stage);
        }

        public void Complete(string id, JsonNode? result)
        {
            OperationHandle handle = PendingHandle(id);
            handle.State = new OperationState.Completed(result);
        }

        public void Fail(string id, string error)
        {
            OperationHandle handle = PendingHandle(id);
            handle.State = new OperationState.Failed(error);
        }

        public OperationHandle Get(string id)
        {
            if (!_operations.TryGetValue(id, out OperationHandle? handle))
            {
                throw new McpError.UnknownOperation(id);
            }

            return handle;
        }

        public IReadOnlyList<OperationHandle> All => _order;

        private OperationHandle PendingHandle(string id)
        {
            OperationHandle handle = Get(id);
            if (!handle.State.IsPending)
            {
                throw new McpError.OperationNotPending(id);
            }

            return handle;
        }
    }
}
