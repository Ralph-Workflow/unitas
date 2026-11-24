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

            GameObject sourceChild = new GameObject("Child");
            sourceChild.transform.parent = source.transform;
            sourceChild.transform.localPosition = new Vector3(4f, 5f, 6f);
            CloneProbeScript childProbe = sourceChild.AddComponent<CloneProbeScript>();
            childProbe.Marker = 7;

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
        }
    }
}
