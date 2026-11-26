namespace UnityEngine.Collections
{
    public interface INativeArray
    {
        int Length { get; }

        string Name { get; }

        bool IsCreated { get; }
    }

    internal interface INativeArrayDependency
    {
        bool IsCompleted { get; }

        void Complete();

        INativeArrayDependency? Parent { get; }
    }
}
