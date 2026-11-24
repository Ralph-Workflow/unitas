using UnityEngine;

namespace Unitas.Bridge.Tests
{
    public sealed class CloneProbeScript : MonoBehaviour
    {
        public int Marker;

        public string Label = string.Empty;
    }

    public static class CloneTests
    {
        public static void Run()
        {
            GameObject source = new GameObject("Source");
            CloneProbeScript sourceProbe = source.AddComponent<CloneProbeScript>();
            sourceProbe.Marker = 42;
            sourceProbe.Label = "probe";
            source.transform.localPosition = new Vector3(1f, 2f, 3f);

            BoxCollider sourceBox = source.AddComponent<BoxCollider>();
            sourceBox.isTrigger = true;
            sourceBox.center = new Vector3(1.5f, 2.5f, 3.5f);
            sourceBox.size = new Vector3(10f, 20f, 30f);

            SphereCollider sourceSphere = source.AddComponent<SphereCollider>();
            sourceSphere.isTrigger = true;
            sourceSphere.center = new Vector3(-1f, -2f, -3f);
            sourceSphere.radius = 4.5f;

            GameObject sourceChild = new GameObject("Child");
            sourceChild.transform.parent = source.transform;
            sourceChild.transform.localPosition = new Vector3(4f, 5f, 6f);
            CloneProbeScript childProbe = sourceChild.AddComponent<CloneProbeScript>();
            childProbe.Marker = 7;

            GameObject sourceGrandchild = new GameObject("Grandchild");
            sourceGrandchild.transform.parent = sourceChild.transform;
            sourceGrandchild.transform.localPosition = new Vector3(7f, 8f, 9f);
            CloneProbeScript grandchildProbe = sourceGrandchild.AddComponent<CloneProbeScript>();
            grandchildProbe.Marker = 99;
            grandchildProbe.Label = "grandchild";

            GameObject clone = Object.Instantiate(source);
            Check.That("clone is a different object", !ReferenceEquals(clone, source));
            Check.Equal("clone name", "Source (Clone)", clone.name);
            Check.That("clone has a new instance id", clone.GetInstanceID() != source.GetInstanceID());
            Check.Equal("clone keeps the root local position x", 1f, clone.transform.localPosition.x);
            Check.Equal("clone keeps the root local position y", 2f, clone.transform.localPosition.y);
            Check.That("clone keeps the root active state", clone.activeSelf == source.activeSelf);

            CloneProbeScript? cloneProbe = clone.GetComponent<CloneProbeScript>();
            Check.That("clone carries a cloned behaviour", cloneProbe != null);
            if (cloneProbe != null)
            {
                Check.Equal("cloned behaviour keeps the marker", 42, cloneProbe.Marker);
                Check.Equal("cloned behaviour keeps the label", "probe", cloneProbe.Label);
                Check.That("cloned behaviour is a different instance", !ReferenceEquals(cloneProbe, sourceProbe));
                Check.That("cloned behaviour has a new instance id", cloneProbe.GetInstanceID() != sourceProbe.GetInstanceID());
            }

            BoxCollider? cloneBox = clone.GetComponent<BoxCollider>();
            Check.That("clone carries a cloned box collider", cloneBox != null);
            if (cloneBox != null)
            {
                Check.That("cloned box collider is a different instance", !ReferenceEquals(cloneBox, sourceBox));
                Check.That("cloned box collider has a new instance id", cloneBox.GetInstanceID() != sourceBox.GetInstanceID());
                Check.That("cloned box collider keeps isTrigger", cloneBox.isTrigger);
                Check.Equal("cloned box collider keeps center x", 1.5f, cloneBox.center.x);
                Check.Equal("cloned box collider keeps center y", 2.5f, cloneBox.center.y);
                Check.Equal("cloned box collider keeps center z", 3.5f, cloneBox.center.z);
                Check.Equal("cloned box collider keeps size x", 10f, cloneBox.size.x);
                Check.Equal("cloned box collider keeps size y", 20f, cloneBox.size.y);
                Check.Equal("cloned box collider keeps size z", 30f, cloneBox.size.z);
            }

            SphereCollider? cloneSphere = clone.GetComponent<SphereCollider>();
            Check.That("clone carries a cloned sphere collider", cloneSphere != null);
            if (cloneSphere != null)
            {
                Check.That("cloned sphere collider is a different instance", !ReferenceEquals(cloneSphere, sourceSphere));
                Check.That("cloned sphere collider has a new instance id", cloneSphere.GetInstanceID() != sourceSphere.GetInstanceID());
                Check.That("cloned sphere collider keeps isTrigger", cloneSphere.isTrigger);
                Check.Equal("cloned sphere collider keeps center x", -1f, cloneSphere.center.x);
                Check.Equal("cloned sphere collider keeps center y", -2f, cloneSphere.center.y);
                Check.Equal("cloned sphere collider keeps center z", -3f, cloneSphere.center.z);
                Check.Equal("cloned sphere collider keeps radius", 4.5f, cloneSphere.radius);
            }

            Check.Equal("clone keeps the child count", 1, clone.transform.childCount);
            Transform cloneChild = clone.transform.GetChild(0);
            Check.Equal("cloned child name", "Child", cloneChild.gameObject.name);
            Check.Equal("cloned child local position x", 4f, cloneChild.localPosition.x);
            Check.Equal("cloned child local position z", 6f, cloneChild.localPosition.z);

            CloneProbeScript? cloneChildProbe = cloneChild.gameObject.GetComponent<CloneProbeScript>();
            Check.That("cloned child carries a cloned behaviour", cloneChildProbe != null);
            if (cloneChildProbe != null)
            {
                Check.Equal("cloned child behaviour keeps the marker", 7, cloneChildProbe.Marker);
                Check.That("cloned child behaviour is a different instance", !ReferenceEquals(cloneChildProbe, childProbe));
                Check.That("cloned child behaviour has a new instance id", cloneChildProbe.GetInstanceID() != childProbe.GetInstanceID());
            }

            Check.Equal("cloned child keeps the grandchild count", 1, cloneChild.childCount);
            Transform cloneGrandchild = cloneChild.GetChild(0);
            Check.Equal("cloned grandchild name", "Grandchild", cloneGrandchild.gameObject.name);
            Check.That("cloned grandchild is a different object", !ReferenceEquals(cloneGrandchild.gameObject, sourceGrandchild));
            Check.That("cloned grandchild has a new instance id", cloneGrandchild.gameObject.GetInstanceID() != sourceGrandchild.GetInstanceID());
            Check.Equal("cloned grandchild local position x", 7f, cloneGrandchild.localPosition.x);
            Check.Equal("cloned grandchild local position y", 8f, cloneGrandchild.localPosition.y);
            Check.Equal("cloned grandchild local position z", 9f, cloneGrandchild.localPosition.z);

            CloneProbeScript? cloneGrandchildProbe = cloneGrandchild.gameObject.GetComponent<CloneProbeScript>();
            Check.That("cloned grandchild carries a cloned behaviour", cloneGrandchildProbe != null);
            if (cloneGrandchildProbe != null)
            {
                Check.Equal("cloned grandchild behaviour keeps the marker", 99, cloneGrandchildProbe.Marker);
                Check.Equal("cloned grandchild behaviour keeps the label", "grandchild", cloneGrandchildProbe.Label);
                Check.That("cloned grandchild behaviour is a different instance", !ReferenceEquals(cloneGrandchildProbe, grandchildProbe));
                Check.That("cloned grandchild behaviour has a new instance id", cloneGrandchildProbe.GetInstanceID() != grandchildProbe.GetInstanceID());
            }
        }
    }
}
