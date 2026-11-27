namespace UnityEngine
{
    /// <summary>The way a SpriteRenderer draws its sprite.</summary>
    public enum SpriteDrawMode
    {
        Simple = 0,
        Sliced = 1,
        Tiled = 2,
    }

    /// <summary>The state of the sprite a SpriteRenderer draws.</summary>
    public sealed class SpriteState
    {
        public SpriteState(
            Color color,
            int sortingLayerId,
            int sortingOrder,
            bool flipX,
            bool flipY,
            bool enabled,
            bool hasTexture)
        {
            Color = color;
            SortingLayerId = sortingLayerId;
            SortingOrder = sortingOrder;
            FlipX = flipX;
            FlipY = flipY;
            Enabled = enabled;
            HasTexture = hasTexture;
        }

        public Color Color { get; }

        public int SortingLayerId { get; }

        public int SortingOrder { get; }

        public bool FlipX { get; }

        public bool FlipY { get; }

        public bool Enabled { get; }

        public bool HasTexture { get; }
    }

    /// <summary>The native sprite a managed SpriteRenderer reads and writes.</summary>
    public interface ISpriteProvider
    {
        bool TrySetSprite(ulong entity, string texturePath);

        bool TrySetSpriteColor(ulong entity, Color color);

        bool TrySetSpriteSorting(ulong entity, int sortingLayerId, int sortingOrder);

        bool TrySetSpriteFlip(ulong entity, bool flipX, bool flipY);

        bool TrySetSpriteEnabled(ulong entity, bool enabled);

        bool TrySetSpriteDrawMode(ulong entity, SpriteDrawMode drawMode);

        bool TryGetSpriteState(ulong entity, out SpriteState state);
    }

    /// <summary>
    /// Draws a texture as a sprite. The sprite joins the sprites the scene
    /// declares, so a script sprite and a scene sprite sort into the same
    /// draw pass by their sorting layer and their order in the layer.
    /// </summary>
    public sealed class SpriteRenderer : Behaviour
    {
        /// <summary>The native sprite the managed component reads and writes.</summary>
        public static ISpriteProvider? Provider { get; set; }

        private string _sprite = string.Empty;
        private Color _color = Color.white;
        private int _sortingLayerId;
        private int _sortingOrder;
        private bool _flipX;
        private bool _flipY;
        private SpriteDrawMode _drawMode = SpriteDrawMode.Simple;

        /// <summary>
        /// The texture the renderer draws, named relative to the project asset
        /// folder or as an absolute path. An empty path draws no texture.
        /// </summary>
        public string sprite
        {
            get => _sprite;
            set
            {
                _sprite = value ?? string.Empty;
                if (!TryNativeEntity(out ulong entity))
                {
                    return;
                }

                ISpriteProvider? provider = Provider;
                if (provider == null)
                {
                    return;
                }

                if (!provider.TrySetSprite(entity, _sprite))
                {
                    if (!string.IsNullOrEmpty(_sprite))
                    {
                        Debug.LogError("SpriteRenderer could not draw the sprite " + _sprite);
                    }

                    return;
                }

                provider.TrySetSpriteColor(entity, _color);
                provider.TrySetSpriteSorting(entity, _sortingLayerId, _sortingOrder);
                provider.TrySetSpriteFlip(entity, _flipX, _flipY);
                provider.TrySetSpriteEnabled(entity, enabled);
            }
        }

        /// <summary>The tint the renderer multiplies the texture with.</summary>
        public Color color
        {
            get => ReadState()?.Color ?? _color;
            set
            {
                _color = value;
                if (TryNativeEntity(out ulong entity))
                {
                    Provider?.TrySetSpriteColor(entity, value);
                }
            }
        }

        /// <summary>Whether the renderer draws the sprite mirrored sideways.</summary>
        public bool flipX
        {
            get => ReadState()?.FlipX ?? _flipX;
            set
            {
                _flipX = value;
                if (TryNativeEntity(out ulong entity))
                {
                    Provider?.TrySetSpriteFlip(entity, value, _flipY);
                }
            }
        }

        /// <summary>Whether the renderer draws the sprite mirrored upside down.</summary>
        public bool flipY
        {
            get => ReadState()?.FlipY ?? _flipY;
            set
            {
                _flipY = value;
                if (TryNativeEntity(out ulong entity))
                {
                    Provider?.TrySetSpriteFlip(entity, _flipX, value);
                }
            }
        }

        /// <summary>The sorting layer the renderer draws the sprite in.</summary>
        public int sortingLayerID
        {
            get => ReadState()?.SortingLayerId ?? _sortingLayerId;
            set
            {
                _sortingLayerId = value;
                if (TryNativeEntity(out ulong entity))
                {
                    Provider?.TrySetSpriteSorting(entity, value, _sortingOrder);
                }
            }
        }

        /// <summary>The order the renderer draws the sprite in its sorting layer.</summary>
        public int sortingOrder
        {
            get => ReadState()?.SortingOrder ?? _sortingOrder;
            set
            {
                _sortingOrder = value;
                if (TryNativeEntity(out ulong entity))
                {
                    Provider?.TrySetSpriteSorting(entity, _sortingLayerId, value);
                }
            }
        }

        /// <summary>
        /// The way the renderer draws the sprite. The sliced and tiled modes are
        /// not drawn yet and are reported as an error instead of being drawn
        /// as a simple sprite without a word.
        /// </summary>
        public SpriteDrawMode drawMode
        {
            get => _drawMode;
            set
            {
                _drawMode = value;
                if (TryNativeEntity(out ulong entity))
                {
                    Provider?.TrySetSpriteDrawMode(entity, value);
                }

                if (value != SpriteDrawMode.Simple)
                {
                    Debug.LogError(
                        "SpriteRenderer draws in the " + DrawModeName(value)
                        + " mode, which the renderer does not draw yet");
                }
            }
        }

        internal override void OnEnabledChanged(bool isEnabled)
        {
            if (TryNativeEntity(out ulong entity))
            {
                Provider?.TrySetSpriteEnabled(entity, isEnabled);
            }
        }

        private static string DrawModeName(SpriteDrawMode mode)
        {
            switch (mode)
            {
                case SpriteDrawMode.Sliced:
                    return "Sliced";
                case SpriteDrawMode.Tiled:
                    return "Tiled";
                default:
                    return "Simple";
            }
        }

        private SpriteState? ReadState()
        {
            ISpriteProvider? provider = Provider;
            if (provider == null || !TryNativeEntity(out ulong entity))
            {
                return null;
            }

            return provider.TryGetSpriteState(entity, out SpriteState state) ? state : null;
        }

        private bool TryNativeEntity(out ulong entity)
        {
            entity = 0;
            if (Provider == null)
            {
                return false;
            }

            GameObject? owner = gameObject;
            if (owner == null || owner.IsDestroyed)
            {
                return false;
            }

            entity = owner.NativeEntityId;
            return entity != 0;
        }
    }
}
