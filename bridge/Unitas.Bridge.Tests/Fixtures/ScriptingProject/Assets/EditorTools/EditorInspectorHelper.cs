using MyGame.Core;

namespace MyGame.EditorTools
{
    public static class EditorInspectorHelper
    {
        public static string Inspect()
        {
            return "Editor:" + CoreCounter.ActiveProfileSummary();
        }
    }
}
