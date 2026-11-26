namespace UnityEngine.UI
{
    /// <summary>Managed uGUI canvas scaler: a thin wrapper over the native UI object.</summary>
    public class CanvasScaler : MonoBehaviour
    {
        /// <summary>How the canvas scales its content with the screen.</summary>
        public enum ScaleMode
        {
            ConstantPixelSize = 0,
            ScaleWithScreenSize = 1,
            ConstantPhysicalSize = 2,
        }

        /// <summary>How the canvas matches the screen width against its height.</summary>
        public enum ScreenMatchMode
        {
            MatchWidthOrHeight = 0,
            Expand = 1,
            Shrink = 2,
        }

        private ScaleMode _uiScaleMode = ScaleMode.ScaleWithScreenSize;
        private ScreenMatchMode _screenMatchMode = ScreenMatchMode.MatchWidthOrHeight;
        private Vector2 _referenceResolution = new Vector2(800f, 600f);
        private float _matchWidthOrHeight = 0.5f;

        /// <summary>How the canvas scales its content with the screen.</summary>
        public ScaleMode uiScaleMode
        {
            get => _uiScaleMode;
            set => _uiScaleMode = value;
        }

        /// <summary>How the canvas matches the screen width against its height.</summary>
        public ScreenMatchMode screenMatchMode
        {
            get => _screenMatchMode;
            set => _screenMatchMode = value;
        }

        /// <summary>The resolution the canvas is laid out against.</summary>
        public Vector2 referenceResolution
        {
            get => _referenceResolution;
            set => _referenceResolution = value;
        }

        /// <summary>Where the match between the screen width and height sits, from zero to one.</summary>
        public float matchWidthOrHeight
        {
            get => _matchWidthOrHeight;
            set => _matchWidthOrHeight = value;
        }

        /// <summary>Native scene object id of the game object this scaler belongs to.</summary>
        public ulong NativeObjectId
        {
            get
            {
                GameObject? owner = Owner;
                return owner != null && !owner.IsDestroyed ? owner.NativeEntityId : 0UL;
            }
        }

        internal GameObject? Owner => OwnerOrNull;
    }
}
