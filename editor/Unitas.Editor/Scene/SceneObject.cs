using System.Collections.Generic;

namespace Unitas.Editor.Scene
{
    /// <summary>A GameObject in the scene: its name, its parent GameObject and its components.</summary>
    public sealed class SceneObject
    {
        private readonly List<SceneObject> children = new List<SceneObject>();
        private readonly List<long> componentFileIds = new List<long>();

        internal SceneObject(long fileId, string name, long parentFileId)
        {
            FileId = fileId;
            Name = name;
            ParentFileId = parentFileId;
        }

        public long FileId { get; }

        public string Name { get; }

        /// <summary>The parent GameObject file id, or 0 when the object is a scene root.</summary>
        public long ParentFileId { get; }

        public IReadOnlyList<SceneObject> Children => children;

        /// <summary>The file ids listed in the GameObject m_Component list.</summary>
        public IReadOnlyList<long> ComponentFileIds => componentFileIds;

        internal void AddChild(SceneObject child)
        {
            children.Add(child);
        }

        internal void AddComponent(long fileId)
        {
            componentFileIds.Add(fileId);
        }
    }
}
