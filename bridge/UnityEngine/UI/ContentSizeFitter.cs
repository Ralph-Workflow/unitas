namespace UnityEngine.UI
{
    public class ContentSizeFitter : MonoBehaviour
    {
        public enum FitMode
        {
            Unconstrained = 0,
            MinSize = 1,
            PreferredSize = 2,
        }

        private FitMode _horizontalFit = FitMode.Unconstrained;
        private FitMode _verticalFit = FitMode.Unconstrained;

        public FitMode horizontalFit
        {
            get => _horizontalFit;
            set
            {
                _horizontalFit = value;
                ApplyFitter();
            }
        }

        public FitMode verticalFit
        {
            get => _verticalFit;
            set
            {
                _verticalFit = value;
                ApplyFitter();
            }
        }

        public ulong NativeObjectId
        {
            get
            {
                GameObject? owner = Owner;
                return owner != null && !owner.IsDestroyed ? owner.NativeEntityId : 0UL;
            }
        }

        internal GameObject? Owner => OwnerOrNull;

        public virtual void SetLayoutHorizontal()
        {
            ApplyFitter();
        }

        public virtual void SetLayoutVertical()
        {
            ApplyFitter();
        }

        public override void OnEnable()
        {
            ApplyFitter();
        }

        public override void OnDisable()
        {
            ApplyFitter();
        }

        private void ApplyFitter()
        {
            IUiProvider? provider = Ui.Provider;
            ulong objectId = NativeObjectId;
            if (provider == null || objectId == 0UL)
            {
                return;
            }

            provider.SetContentSizeFitter(
                objectId,
                _horizontalFit,
                _verticalFit,
                isActiveAndEnabled);
        }
    }
}
