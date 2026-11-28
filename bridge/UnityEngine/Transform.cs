using System;
using System.Collections.Generic;

namespace UnityEngine
{
    public enum Space
    {
        World = 0,
        Self = 1,
    }

    public class Transform : Component
    {
        private static readonly List<Transform> s_rootTransforms = new List<Transform>();
        private readonly List<Transform> _children = new List<Transform>();
        private Transform? _parent;
        private Vector3 _localPosition;
        private Quaternion _localRotation = Quaternion.identity;
        private Vector3 _localScale = Vector3.one;
        private bool _hasChanged = true;
        private Rigidbody? _cachedPositionBody;

        public Transform()
        {
            s_rootTransforms.Add(this);
        }

        public new Transform transform => this;

        public bool hasChanged
        {
            get => _hasChanged;
            set => _hasChanged = value;
        }

        public Vector3 localPosition
        {
            get => _localPosition;
            set
            {
                _localPosition = value;
                MarkChanged();
                PushPositionToNative();
                NotifyCharacterControllerTeleported();
            }
        }

        public Vector3 position
        {
            get
            {
                if (_parent != null)
                {
                    return _parent.TransformPoint(_localPosition);
                }
                return _localPosition;
            }
            set
            {
                if (_parent != null)
                {
                    _localPosition = _parent.InverseTransformPoint(value);
                }
                else
                {
                    _localPosition = value;
                }
                MarkChanged();
                PushPositionToNative();
                NotifyCharacterControllerTeleported();
            }
        }

        public Quaternion localRotation
        {
            get => _localRotation;
            set
            {
                _localRotation = Quaternion.Normalize(value);
                MarkChanged();
                PushRotationToNative();
            }
        }

        public Quaternion rotation
        {
            get
            {
                if (_parent != null)
                {
                    if (_parent.lossyScale.x * _parent.lossyScale.y * _parent.lossyScale.z >= 0f
                        && _localScale.x * _localScale.y * _localScale.z >= 0f)
                    {
                        return Quaternion.Normalize(_parent.rotation * _localRotation);
                    }
                    return localToWorldMatrix.rotation;
                }
                return _localRotation;
            }
            set
            {
                Quaternion normalized = Quaternion.Normalize(value);
                if (_parent != null)
                {
                    _localRotation = Quaternion.Normalize(Quaternion.Inverse(_parent.rotation) * normalized);
                }
                else
                {
                    _localRotation = normalized;
                }
                MarkChanged();
                PushRotationToNative();
            }
        }

        public Vector3 localEulerAngles
        {
            get => _localRotation.eulerAngles;
            set => localRotation = Quaternion.Euler(value);
        }

        public Vector3 eulerAngles
        {
            get => rotation.eulerAngles;
            set => rotation = Quaternion.Euler(value);
        }

        public Vector3 localScale
        {
            get => _localScale;
            set
            {
                _localScale = value;
                MarkChanged();
                PushRuntimeTransformToNative();
            }
        }

        public Vector3 lossyScale => localToWorldMatrix.lossyScale;

        public Matrix4x4 localToWorldMatrix
        {
            get
            {
                Matrix4x4 local = Matrix4x4.TRS(_localPosition, _localRotation, _localScale);
                if (_parent != null)
                {
                    return _parent.localToWorldMatrix * local;
                }
                return local;
            }
        }

        public Matrix4x4 worldToLocalMatrix => localToWorldMatrix.inverse;

        public Vector3 right
        {
            get => rotation * Vector3.right;
            set => rotation = Quaternion.FromToRotation(Vector3.right, value);
        }

        public Vector3 up
        {
            get => rotation * Vector3.up;
            set => rotation = Quaternion.FromToRotation(Vector3.up, value);
        }

        public Vector3 forward
        {
            get => rotation * Vector3.forward;
            set => rotation = Quaternion.LookRotation(value);
        }

        public Transform? parent
        {
            get => _parent;
            set => SetParent(value);
        }

        public Transform root
        {
            get
            {
                Transform current = this;
                while (current._parent != null)
                {
                    current = current._parent;
                }
                return current;
            }
        }

        public int childCount => _children.Count;

        public Transform GetChild(int index) => _children[index];

        public void SetParent(Transform? parent)
        {
            SetParent(parent, true);
        }

        public void SetParent(Transform? newParent, bool worldPositionStays)
        {
            if (_parent == newParent)
            {
                return;
            }

            if (newParent != null && IsChildOf(this, newParent))
            {
                return;
            }

            Matrix4x4 oldWorldMatrix = localToWorldMatrix;
            Quaternion oldWorldRotation = rotation;

            if (_parent != null)
            {
                _parent._children.Remove(this);
            }
            else
            {
                s_rootTransforms.Remove(this);
            }

            _parent = newParent;

            if (_parent != null)
            {
                _parent._children.Add(this);
                if (worldPositionStays)
                {
                    ApplyWorldMatrixUnderParent(oldWorldMatrix, oldWorldRotation, _parent);
                }
            }
            else
            {
                s_rootTransforms.Add(this);
                if (worldPositionStays)
                {
                    _localPosition = oldWorldMatrix.GetPosition();
                    _localRotation = oldWorldMatrix.ValidTRS() && oldWorldMatrix.determinant != 0f
                        ? oldWorldMatrix.rotation
                        : oldWorldRotation;
                    _localScale = oldWorldMatrix.lossyScale;
                }
            }

            MarkChanged();
        }

        private void ApplyWorldMatrixUnderParent(Matrix4x4 oldWorldMatrix, Quaternion oldWorldRotation, Transform parentTransform)
        {
            Matrix4x4 parentWorld = parentTransform.localToWorldMatrix;
            Matrix4x4 parentInv = parentWorld.inverse;
            Matrix4x4 newLocal = parentInv * oldWorldMatrix;

            _localPosition = newLocal.GetPosition();

            Vector3 col0 = new Vector3(newLocal.m00, newLocal.m10, newLocal.m20);
            Vector3 col1 = new Vector3(newLocal.m01, newLocal.m11, newLocal.m21);
            Vector3 col2 = new Vector3(newLocal.m02, newLocal.m12, newLocal.m22);

            Quaternion targetLocalRot = Quaternion.Normalize(Quaternion.Inverse(parentTransform.rotation) * oldWorldRotation);

            Vector3 basis0 = targetLocalRot * Vector3.right;
            Vector3 basis1 = targetLocalRot * Vector3.up;
            Vector3 basis2 = targetLocalRot * Vector3.forward;

            float dot0 = Vector3.Dot(col0, basis0);
            float dot1 = Vector3.Dot(col1, basis1);
            float dot2 = Vector3.Dot(col2, basis2);

            Vector3 aligned0 = basis0 * dot0;
            Vector3 aligned1 = basis1 * dot1;
            Vector3 aligned2 = basis2 * dot2;

            bool isAlignedWithTargetRotation =
                (col0 - aligned0).magnitude <= 1e-3f &&
                (col1 - aligned1).magnitude <= 1e-3f &&
                (col2 - aligned2).magnitude <= 1e-3f;

            if (isAlignedWithTargetRotation)
            {
                _localRotation = targetLocalRot;
                _localScale = new Vector3(dot0, dot1, dot2);
            }
            else
            {
                _localRotation = newLocal.rotation;
                _localScale = newLocal.lossyScale;
            }
        }

        private static bool IsChildOf(Transform potentialAncestor, Transform candidate)
        {
            Transform? curr = candidate;
            while (curr != null)
            {
                if (ReferenceEquals(curr, potentialAncestor))
                {
                    return true;
                }
                curr = curr._parent;
            }
            return false;
        }

        public bool IsChildOf(Transform parent)
        {
            if (parent == null)
            {
                return false;
            }
            return IsChildOf(parent, this);
        }

        public void DetachChildren()
        {
            for (int i = _children.Count - 1; i >= 0; i--)
            {
                _children[i].SetParent(null, true);
            }
        }

        public int GetSiblingIndex()
        {
            if (_parent != null)
            {
                return _parent._children.IndexOf(this);
            }

            List<Transform> roots = GetSceneRootTransforms();
            int idx = roots.IndexOf(this);
            return idx >= 0 ? idx : 0;
        }

        public void SetSiblingIndex(int index)
        {
            if (_parent != null)
            {
                int currentIdx = _parent._children.IndexOf(this);
                if (currentIdx >= 0)
                {
                    _parent._children.RemoveAt(currentIdx);
                }
                int clamped = index < 0 ? 0 : (index > _parent._children.Count ? _parent._children.Count : index);
                _parent._children.Insert(clamped, this);
                MarkChanged();
                return;
            }

            ReorderRootSibling(index);
            MarkChanged();
        }

        public void SetAsFirstSibling()
        {
            SetSiblingIndex(0);
        }

        public void SetAsLastSibling()
        {
            if (_parent != null)
            {
                SetSiblingIndex(_parent._children.Count - 1);
                return;
            }

            List<Transform> roots = GetSceneRootTransforms();
            SetSiblingIndex(roots.Count - 1);
        }

        private List<Transform> GetSceneRootTransforms()
        {
            PruneRootTransforms();
            List<Transform> roots = new List<Transform>();
            GameObject? myOwner = gameObject;
            for (int i = 0; i < s_rootTransforms.Count; i++)
            {
                Transform candidate = s_rootTransforms[i];
                GameObject? owner = candidate.gameObject;
                if (owner != null && !owner.IsDestroyed)
                {
                    if (myOwner == null || owner.scene == myOwner.scene)
                    {
                        roots.Add(candidate);
                    }
                }
            }
            return roots;
        }

        private void ReorderRootSibling(int index)
        {
            List<Transform> sceneRoots = GetSceneRootTransforms();
            int currentSceneIdx = sceneRoots.IndexOf(this);
            if (currentSceneIdx < 0)
            {
                return;
            }
            sceneRoots.RemoveAt(currentSceneIdx);
            int clamped = index < 0 ? 0 : (index > sceneRoots.Count ? sceneRoots.Count : index);
            sceneRoots.Insert(clamped, this);

            GameObject? myOwner = gameObject;
            List<Transform> rebuilt = new List<Transform>(s_rootTransforms.Count);
            int sceneCursor = 0;
            for (int i = 0; i < s_rootTransforms.Count; i++)
            {
                Transform candidate = s_rootTransforms[i];
                GameObject? owner = candidate.gameObject;
                if (owner != null && !owner.IsDestroyed && (myOwner == null || owner.scene == myOwner.scene))
                {
                    if (sceneCursor < sceneRoots.Count)
                    {
                        rebuilt.Add(sceneRoots[sceneCursor]);
                        sceneCursor++;
                    }
                }
                else
                {
                    rebuilt.Add(candidate);
                }
            }
            s_rootTransforms.Clear();
            s_rootTransforms.AddRange(rebuilt);
        }

        private static void PruneRootTransforms()
        {
            for (int i = s_rootTransforms.Count - 1; i >= 0; i--)
            {
                Transform candidate = s_rootTransforms[i];
                GameObject? owner = candidate.gameObject;
                if (candidate.IsDestroyed || candidate._parent != null || (owner != null && owner.IsDestroyed))
                {
                    s_rootTransforms.RemoveAt(i);
                }
            }
        }

        public Transform? Find(string n)
        {
            if (n == null)
            {
                throw new ArgumentNullException(nameof(n));
            }
            if (n.Length == 0)
            {
                return null;
            }

            string[] segments = n.Split('/');
            Transform current = this;
            for (int s = 0; s < segments.Length; s++)
            {
                string seg = segments[s];
                if (seg.Length == 0)
                {
                    return null;
                }
                Transform? matchedChild = null;
                for (int i = 0; i < current._children.Count; i++)
                {
                    Transform child = current._children[i];
                    GameObject? childGo = child.gameObject;
                    if (childGo != null && !childGo.IsDestroyed && string.Equals(childGo.name, seg, StringComparison.Ordinal))
                    {
                        matchedChild = child;
                        break;
                    }
                }
                if (matchedChild == null)
                {
                    return null;
                }
                current = matchedChild;
            }
            return current;
        }

        public void Translate(Vector3 translation)
        {
            Translate(translation, Space.Self);
        }

        public void Translate(Vector3 translation, Space relativeTo)
        {
            if (relativeTo == Space.World)
            {
                position += translation;
            }
            else
            {
                position += TransformDirection(translation);
            }
        }

        public void Translate(float x, float y, float z)
        {
            Translate(new Vector3(x, y, z), Space.Self);
        }

        public void Translate(float x, float y, float z, Space relativeTo)
        {
            Translate(new Vector3(x, y, z), relativeTo);
        }

        public void Translate(Vector3 translation, Transform? relativeTo)
        {
            if (relativeTo != null)
            {
                position += relativeTo.TransformDirection(translation);
            }
            else
            {
                position += translation;
            }
        }

        public void Translate(float x, float y, float z, Transform? relativeTo)
        {
            Translate(new Vector3(x, y, z), relativeTo);
        }

        public void Rotate(Vector3 eulers)
        {
            Rotate(eulers, Space.Self);
        }

        public void Rotate(Vector3 eulers, Space relativeTo)
        {
            Quaternion eulerRot = Quaternion.Euler(eulers.x, eulers.y, eulers.z);
            if (relativeTo == Space.Self)
            {
                localRotation = _localRotation * eulerRot;
            }
            else
            {
                Quaternion currentRot = rotation;
                rotation = currentRot * (Quaternion.Inverse(currentRot) * eulerRot * currentRot);
            }
        }

        public void Rotate(float xAngle, float yAngle, float zAngle)
        {
            Rotate(new Vector3(xAngle, yAngle, zAngle), Space.Self);
        }

        public void Rotate(float xAngle, float yAngle, float zAngle, Space relativeTo)
        {
            Rotate(new Vector3(xAngle, yAngle, zAngle), relativeTo);
        }

        public void Rotate(Vector3 axis, float angle)
        {
            Rotate(axis, angle, Space.Self);
        }

        public void Rotate(Vector3 axis, float angle, Space relativeTo)
        {
            if (relativeTo == Space.Self)
            {
                RotateAroundInternal(TransformDirection(axis), angle);
            }
            else
            {
                RotateAroundInternal(axis, angle);
            }
        }

        public void RotateAround(Vector3 point, Vector3 axis, float angle)
        {
            Vector3 worldPos = position;
            Quaternion q = Quaternion.AngleAxis(angle, axis);
            Vector3 dif = worldPos - point;
            dif = q * dif;
            position = point + dif;
            RotateAroundInternal(axis, angle);
        }

        private void RotateAroundInternal(Vector3 worldAxis, float angleDegrees)
        {
            Quaternion delta = Quaternion.AngleAxis(angleDegrees, worldAxis);
            rotation = delta * rotation;
        }

        public void LookAt(Transform target)
        {
            if (target != null)
            {
                LookAt(target.position, Vector3.up);
            }
        }

        public void LookAt(Transform target, Vector3 worldUp)
        {
            if (target != null)
            {
                LookAt(target.position, worldUp);
            }
        }

        public void LookAt(Vector3 worldPosition)
        {
            LookAt(worldPosition, Vector3.up);
        }

        public void LookAt(Vector3 worldPosition, Vector3 worldUp)
        {
            Vector3 forwardDir = worldPosition - position;
            if (forwardDir.magnitude > 1e-6f)
            {
                rotation = Quaternion.LookRotation(forwardDir, worldUp);
            }
        }

        public Vector3 TransformPoint(Vector3 position)
        {
            return localToWorldMatrix.MultiplyPoint3x4(position);
        }

        public Vector3 TransformPoint(float x, float y, float z)
        {
            return TransformPoint(new Vector3(x, y, z));
        }

        public Vector3 InverseTransformPoint(Vector3 position)
        {
            return worldToLocalMatrix.MultiplyPoint3x4(position);
        }

        public Vector3 InverseTransformPoint(float x, float y, float z)
        {
            return InverseTransformPoint(new Vector3(x, y, z));
        }

        public Vector3 TransformVector(Vector3 vector)
        {
            return localToWorldMatrix.MultiplyVector(vector);
        }

        public Vector3 TransformVector(float x, float y, float z)
        {
            return TransformVector(new Vector3(x, y, z));
        }

        public Vector3 InverseTransformVector(Vector3 vector)
        {
            return worldToLocalMatrix.MultiplyVector(vector);
        }

        public Vector3 InverseTransformVector(float x, float y, float z)
        {
            return InverseTransformVector(new Vector3(x, y, z));
        }

        public Vector3 TransformDirection(Vector3 direction)
        {
            return rotation * direction;
        }

        public Vector3 TransformDirection(float x, float y, float z)
        {
            return TransformDirection(new Vector3(x, y, z));
        }

        public Vector3 InverseTransformDirection(Vector3 direction)
        {
            return Quaternion.Inverse(rotation) * direction;
        }

        public Vector3 InverseTransformDirection(float x, float y, float z)
        {
            return InverseTransformDirection(new Vector3(x, y, z));
        }

        public void SetPositionAndRotation(Vector3 position, Quaternion rotation)
        {
            this.position = position;
            this.rotation = rotation;
        }

        public void SetLocalPositionAndRotation(Vector3 localPosition, Quaternion localRotation)
        {
            this.localPosition = localPosition;
            this.localRotation = localRotation;
        }

        public void GetPositionAndRotation(out Vector3 position, out Quaternion rotation)
        {
            position = this.position;
            rotation = this.rotation;
        }

        public void GetLocalPositionAndRotation(out Vector3 localPosition, out Quaternion localRotation)
        {
            localPosition = _localPosition;
            localRotation = _localRotation;
        }

        public List<Transform>.Enumerator GetEnumerator() => _children.GetEnumerator();

        private void MarkChanged()
        {
            _hasChanged = true;
            for (int i = 0; i < _children.Count; i++)
            {
                _children[i].MarkChanged();
            }
        }

        internal void SeedPositionFromNative(Vector3 simulatedPosition)
        {
            if (_parent != null)
            {
                _localPosition = _parent.InverseTransformPoint(simulatedPosition);
            }
            else
            {
                _localPosition = simulatedPosition;
            }
            MarkChanged();
        }

        internal void SeedLocalTransformFromScene(Vector3 localPosition, Quaternion localRotation, Vector3 localScale)
        {
            _localPosition = localPosition;
            _localRotation = Quaternion.Normalize(localRotation);
            _localScale = localScale;
            MarkChanged();
        }

        private void PushPositionToNative()
        {
            GameObject? owner = gameObject;
            if (owner == null || owner.IsDestroyed || owner.NativeEntityId == 0)
            {
                return;
            }
            if (owner.IsRuntimeObject)
            {
                IRuntimeObjectProvider? runtimeProvider = RuntimeObjectProvider.Current;
                if (runtimeProvider != null)
                {
                    runtimeProvider.SetRuntimeTransform(owner.NativeEntityId, position, rotation, lossyScale);
                }
            }
            Rigidbody? body = ResolvePositionBody(owner);
            if (body == null)
            {
                return;
            }
            IPhysicsProvider? provider = Physics.Provider;
            if (provider != null)
            {
                provider.TrySetBodyPosition(owner.NativeEntityId, position);
            }
        }

        // Positive-result cache: only a found Rigidbody is cached, so a body added later is
        // re-resolved on the next set. The fake-null operator and owner check catch destroyed or
        // re-owned bodies, keeping add/remove correct without a GetComponent scan on every set.
        private Rigidbody? ResolvePositionBody(GameObject owner)
        {
            Rigidbody? cached = _cachedPositionBody;
            if (cached != null && ReferenceEquals(cached.OwnerOrNull, owner))
            {
                return cached;
            }

            _cachedPositionBody = owner.GetComponent<Rigidbody>();
            return _cachedPositionBody;
        }

        private void PushRotationToNative()
        {
            GameObject? owner = gameObject;
            if (owner == null || owner.IsDestroyed || owner.NativeEntityId == 0)
            {
                return;
            }
            if (owner.IsRuntimeObject)
            {
                IRuntimeObjectProvider? runtimeProvider = RuntimeObjectProvider.Current;
                if (runtimeProvider != null)
                {
                    runtimeProvider.SetRuntimeTransform(owner.NativeEntityId, position, rotation, lossyScale);
                }
            }
            Rigidbody? body = owner.GetComponent<Rigidbody>();
            if (body == null)
            {
                return;
            }
            IPhysicsProvider? provider = Physics.Provider;
            if (provider != null)
            {
                provider.TrySetBodyRotation(owner.NativeEntityId, rotation);
            }
        }

        private void PushRuntimeTransformToNative()
        {
            GameObject? owner = gameObject;
            if (owner == null || owner.IsDestroyed || owner.NativeEntityId == 0 || !owner.IsRuntimeObject)
            {
                return;
            }
            IRuntimeObjectProvider? runtimeProvider = RuntimeObjectProvider.Current;
            if (runtimeProvider != null)
            {
                runtimeProvider.SetRuntimeTransform(owner.NativeEntityId, position, rotation, lossyScale);
            }
        }

        private void NotifyCharacterControllerTeleported()
        {
            GameObject? owner = gameObject;
            if (owner == null || owner.IsDestroyed)
            {
                return;
            }
            CharacterController? controller = owner.GetComponent<CharacterController>();
            if (controller != null && !controller.IsUpdatingTransformFromMove)
            {
                controller.OnTransformTeleported(position);
            }
        }
    }
}
