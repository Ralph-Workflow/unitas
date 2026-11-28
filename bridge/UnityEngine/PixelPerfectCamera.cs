using System;

namespace UnityEngine
{
    public sealed class PixelPerfectCamera : MonoBehaviour
    {
        public int assetsPPU = 16;

        public Vector2 referenceResolution = new Vector2(320f, 180f);

        public bool upscaleRT;

        public bool pixelSnapping = true;

        public bool cropFrameX;

        public bool cropFrameY;

        public bool stretchFill;

        public int ComputeIntegerScale(int verticalPixels)
        {
            int reference = (int)referenceResolution.y;
            if (reference <= 0)
            {
                return 1;
            }

            return Math.Max(1, verticalPixels / reference);
        }

        public float OrthographicSizeForReference()
        {
            int ppu = assetsPPU > 0 ? assetsPPU : 16;
            return referenceResolution.y / (2f * ppu);
        }

        public override void OnEnable()
        {
            ApplyToCamera();
        }

        public override void LateUpdate()
        {
            ApplyToCamera();
        }

        private void ApplyToCamera()
        {
            GameObject? owner = gameObject;
            if (owner == null || owner.IsDestroyed)
            {
                return;
            }

            Camera? camera = owner.GetComponent<Camera>();
            if (camera == null)
            {
                return;
            }

            camera.orthographic = true;
            camera.orthographicSize = OrthographicSizeForReference();
        }
    }
}
