using System.Collections.Generic;

namespace Unitas.Editor.Scene
{
    /// <summary>One property of a scene object or of one of its components.</summary>
    public sealed class SceneProperty
    {
        public SceneProperty(string name, string value)
        {
            Name = name;
            Value = value;
        }

        public string Name { get; }

        /// <summary>The property value as the scene file spells it.</summary>
        public string Value { get; }
    }

    /// <summary>One component of a scene object with the properties the scene file holds for it.</summary>
    public sealed class SceneComponentInspection
    {
        public SceneComponentInspection(long fileId, string type, IReadOnlyList<SceneProperty> properties)
        {
            FileId = fileId;
            Type = type;
            Properties = properties;
        }

        public long FileId { get; }

        /// <summary>The component type as the scene file names it, such as Transform or Rigidbody.</summary>
        public string Type { get; }

        public IReadOnlyList<SceneProperty> Properties { get; }
    }

    /// <summary>Everything the editor knows about one GameObject of the open scene.</summary>
    public sealed class SceneInspection
    {
        public SceneInspection(
            long fileId,
            string name,
            long parentFileId,
            bool active,
            string position,
            string rotation,
            string scale,
            IReadOnlyList<long> childFileIds,
            IReadOnlyList<SceneComponentInspection> components)
        {
            FileId = fileId;
            Name = name;
            ParentFileId = parentFileId;
            Active = active;
            Position = position;
            Rotation = rotation;
            Scale = scale;
            ChildFileIds = childFileIds;
            Components = components;
        }

        public long FileId { get; }

        public string Name { get; }

        public long ParentFileId { get; }

        public bool Active { get; }

        /// <summary>The local position of the object as the scene file spells it.</summary>
        public string Position { get; }

        public string Rotation { get; }

        public string Scale { get; }

        public IReadOnlyList<long> ChildFileIds { get; }

        public IReadOnlyList<SceneComponentInspection> Components { get; }
    }
}
