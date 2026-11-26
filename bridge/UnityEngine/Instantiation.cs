using System;
using System.Collections.Generic;

namespace UnityEngine
{
    public partial class Object
    {
        public static GameObject Instantiate(GameObject original)
        {
            if (original is null)
            {
                throw new ArgumentNullException(nameof(original));
            }

            GameObject clone = new GameObject(original.name + " (Clone)");
            clone.activeSelf = original.activeSelf;
            clone.transform.localPosition = original.transform.localPosition;
            clone.transform.localRotation = original.transform.localRotation;
            clone.transform.localScale = original.transform.localScale;
            CopyComponents(original, clone);
            CopyChildren(original.transform, clone.transform);
            return clone;
        }

        private static void CopyComponents(GameObject source, GameObject target)
        {
            IReadOnlyList<Component> components = source.Components;
            for (int i = 0; i < components.Count; i++)
            {
                Component component = components[i];
                if (component is Transform)
                {
                    continue;
                }

                if (component is MonoBehaviour behaviour)
                {
                    Type scriptType = behaviour.AttachedRuntimeType ?? typeof(MonoBehaviour);
                    IRuntimeObjectProvider? runtimeProvider = RuntimeObjectProvider.Current;
                    if (target.NativeEntityId != 0 && runtimeProvider != null)
                    {
                        RuntimeObjectProvider.RegisterScriptFactory(
                            scriptType,
                            () =>
                            {
                                MonoBehaviour cloned = (MonoBehaviour)behaviour.ShallowClone();
                                cloned.AssignNewInstanceId();
                                return cloned;
                            });
                        Component? runtimeAttached = runtimeProvider.CreateRuntimeScript(target.NativeEntityId, scriptType);
                        if (runtimeAttached != null)
                        {
                            continue;
                        }
                    }

                    MonoBehaviour copy = (MonoBehaviour)behaviour.ShallowClone();
                    copy.AssignNewInstanceId();
                    target.AttachComponent(copy, scriptType);
                    continue;
                }

                if (component is BoxCollider box)
                {
                    BoxCollider copy = new BoxCollider();
                    copy.isTrigger = box.isTrigger;
                    copy.center = box.center;
                    copy.size = box.size;
                    target.AttachComponent(copy, typeof(BoxCollider));
                    target.NotifyRuntimeColliderAttached(copy);
                    continue;
                }

                if (component is SphereCollider sphere)
                {
                    SphereCollider copy = new SphereCollider();
                    copy.isTrigger = sphere.isTrigger;
                    copy.center = sphere.center;
                    copy.radius = sphere.radius;
                    target.AttachComponent(copy, typeof(SphereCollider));
                    target.NotifyRuntimeColliderAttached(copy);
                    continue;
                }

                if (component is CapsuleCollider capsule)
                {
                    CapsuleCollider copy = new CapsuleCollider();
                    copy.isTrigger = capsule.isTrigger;
                    copy.center = capsule.center;
                    copy.radius = capsule.radius;
                    copy.height = capsule.height;
                    copy.direction = capsule.direction;
                    target.AttachComponent(copy, typeof(CapsuleCollider));
                    target.NotifyRuntimeColliderAttached(copy);
                }
            }
        }

        private static void CopyChildren(Transform source, Transform target)
        {
            for (int i = 0; i < source.childCount; i++)
            {
                Transform sourceChild = source.GetChild(i);
                GameObject sourceChildObject = sourceChild.gameObject;
                GameObject targetChildObject = new GameObject(sourceChildObject.name);
                targetChildObject.activeSelf = sourceChildObject.activeSelf;
                targetChildObject.transform.SetParent(target, false);
                targetChildObject.transform.localPosition = sourceChild.localPosition;
                targetChildObject.transform.localRotation = sourceChild.localRotation;
                targetChildObject.transform.localScale = sourceChild.localScale;
                CopyComponents(sourceChildObject, targetChildObject);
                CopyChildren(sourceChild, targetChildObject.transform);
            }
        }
    }
}
