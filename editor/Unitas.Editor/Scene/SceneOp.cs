namespace Unitas.Editor.Scene
{
    public enum SceneOpKind
    {
        SetProperty,
        CreateGameObject,
        DestroyGameObject,
        Reparent,
        SetActive,
        DuplicateGameObject,
    }

    /// <summary>
    /// A reversible scene edit. The inverse op returned by SceneDocument.Apply carries the
    /// document text from before the edit, so applying it restores that text verbatim.
    /// </summary>
    public sealed class SceneOp
    {
        private SceneOp(
            SceneOpKind kind,
            long fileId,
            long parentFileId,
            string propertyPath,
            string value,
            string objectName,
            string documentSnapshot)
        {
            Kind = kind;
            FileId = fileId;
            ParentFileId = parentFileId;
            PropertyPath = propertyPath;
            Value = value;
            ObjectName = objectName;
            DocumentSnapshot = documentSnapshot;
        }

        public SceneOpKind Kind { get; }

        public long FileId { get; }

        public long ParentFileId { get; }

        public string PropertyPath { get; }

        public string Value { get; }

        public string ObjectName { get; }

        public string DocumentSnapshot { get; }

        public static SceneOp SetProperty(long fileId, string path, string value)
        {
            return new SceneOp(
                SceneOpKind.SetProperty,
                fileId,
                0,
                path ?? string.Empty,
                value ?? string.Empty,
                string.Empty,
                string.Empty);
        }

        public static SceneOp CreateGameObject(string name, long parentFileId)
        {
            return new SceneOp(
                SceneOpKind.CreateGameObject,
                0,
                parentFileId,
                string.Empty,
                string.Empty,
                name ?? string.Empty,
                string.Empty);
        }

        public static SceneOp DestroyGameObject(long fileId)
        {
            return new SceneOp(
                SceneOpKind.DestroyGameObject,
                fileId,
                0,
                string.Empty,
                string.Empty,
                string.Empty,
                string.Empty);
        }

        /// <summary>Moves one object under a new parent; a parent file id of 0 is the scene root.</summary>
        public static SceneOp Reparent(long fileId, long parentFileId)
        {
            return new SceneOp(
                SceneOpKind.Reparent,
                fileId,
                parentFileId,
                string.Empty,
                string.Empty,
                string.Empty,
                string.Empty);
        }

        public static SceneOp SetActive(long fileId, bool active)
        {
            return new SceneOp(
                SceneOpKind.SetActive,
                fileId,
                0,
                string.Empty,
                active ? "1" : "0",
                string.Empty,
                string.Empty);
        }

        /// <summary>
        /// Copies one object. A parent file id of -1 keeps the source object's parent,
        /// 0 moves the copy to the scene root and a positive id names the new parent.
        /// </summary>
        public static SceneOp DuplicateGameObject(long fileId, string name, long parentFileId)
        {
            return new SceneOp(
                SceneOpKind.DuplicateGameObject,
                fileId,
                parentFileId,
                string.Empty,
                string.Empty,
                name ?? string.Empty,
                string.Empty);
        }

        public SceneOp WithSnapshot(string documentSnapshot)
        {
            return new SceneOp(
                Kind,
                FileId,
                ParentFileId,
                PropertyPath,
                Value,
                ObjectName,
                documentSnapshot ?? string.Empty);
        }
    }
}
