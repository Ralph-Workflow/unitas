using System.Collections.Generic;
using UnityEngine;

namespace Unitas.Bridge.Tests
{
    public sealed class ActivationProbeScript : MonoBehaviour
    {
        private readonly List<string> _events = new List<string>();

        public int EnableCount { get; private set; }

        public int DisableCount { get; private set; }

        public int UpdateCount { get; private set; }

        public string ObservedOrder => string.Join(", ", _events);

        public override void OnEnable()
        {
            EnableCount++;
            _events.Add("OnEnable");
        }

        public override void OnDisable()
        {
            DisableCount++;
            _events.Add("OnDisable");
        }

        public override void Update()
        {
            UpdateCount++;
            _events.Add("Update");
        }
    }

    public static class ActivationTests
    {
        public static void Run()
        {
            TestLoneObjectActivation();
            TestSetActiveWithCurrentValueIsNoOp();
            TestParentChildActivation();
            TestDisabledChildBehaviourIsPreserved();
            TestBehaviourUnderInactiveAncestor();
            TestOwnActivationUnderInactiveAncestorRaisesNoCallback();
            TestGrandchildIsNotified();
        }

        private static void TestLoneObjectActivation()
        {
            GameObject lone = new GameObject("lone");
            ActivationProbeScript probe = lone.AddComponent<ActivationProbeScript>();

            Check.That("Lone object is active in hierarchy", lone.activeInHierarchy);
            Check.That("Lone behaviour is active and enabled", probe.isActiveAndEnabled);

            lone.SetActive(false);
            Check.That("Deactivated lone object is not active in hierarchy", !lone.activeInHierarchy);
            Check.That("Deactivated lone behaviour is not active and enabled", !probe.isActiveAndEnabled);
            Check.Equal("Deactivating a lone object fires OnDisable once", 1, probe.DisableCount);
            Check.Equal("Deactivating a lone object fires no OnEnable", 0, probe.EnableCount);

            lone.SetActive(true);
            Check.That("Reactivated lone object is active in hierarchy", lone.activeInHierarchy);
            Check.That("Reactivated lone behaviour is active and enabled", probe.isActiveAndEnabled);
            Check.Equal("Reactivating a lone object fires OnEnable once", 1, probe.EnableCount);
            Check.Equal("Reactivating a lone object keeps OnDisable count at one", 1, probe.DisableCount);
            Check.Equal("Lone object observes disable before enable", "OnDisable, OnEnable", probe.ObservedOrder);

            probe.Update();
            Check.Equal("Probe records Update calls", 1, probe.UpdateCount);
            Check.Equal("Probe observes Update after the activation events", "OnDisable, OnEnable, Update", probe.ObservedOrder);
        }

        private static void TestSetActiveWithCurrentValueIsNoOp()
        {
            GameObject lone = new GameObject("no-op");
            ActivationProbeScript probe = lone.AddComponent<ActivationProbeScript>();

            lone.SetActive(true);
            Check.Equal("SetActive with the current active value fires no callback", 0, probe.EnableCount + probe.DisableCount);

            lone.SetActive(false);
            Check.Equal("First deactivation fires OnDisable once", 1, probe.DisableCount);

            lone.SetActive(false);
            Check.Equal("Repeated SetActive(false) fires no extra OnDisable", 1, probe.DisableCount);

            lone.SetActive(true);
            Check.Equal("First reactivation fires OnEnable once", 1, probe.EnableCount);

            lone.SetActive(true);
            Check.Equal("Repeated SetActive(true) fires no extra OnEnable", 1, probe.EnableCount);
            Check.Equal("Repeated SetActive keeps the total callback count at two", 2, probe.EnableCount + probe.DisableCount);
        }

        private static void TestParentChildActivation()
        {
            GameObject parent = new GameObject("parent");
            GameObject child = new GameObject("child");
            child.transform.parent = parent.transform;

            ActivationProbeScript parentProbe = parent.AddComponent<ActivationProbeScript>();
            ActivationProbeScript childProbe = child.AddComponent<ActivationProbeScript>();

            Check.That("Child is active in hierarchy while the parent is active", child.activeInHierarchy);
            Check.That("Child behaviour is active and enabled while the parent is active", childProbe.isActiveAndEnabled);

            parent.SetActive(false);
            Check.That("Child is not active in hierarchy under an inactive parent", !child.activeInHierarchy);
            Check.That("Child behaviour is not active and enabled under an inactive parent", !childProbe.isActiveAndEnabled);
            Check.Equal("Deactivating the parent fires OnDisable on the parent behaviour", 1, parentProbe.DisableCount);
            Check.Equal("Deactivating the parent fires OnDisable on the child behaviour", 1, childProbe.DisableCount);
            Check.Equal("Deactivating the parent fires no OnEnable", 0, parentProbe.EnableCount + childProbe.EnableCount);
            Check.That("The child behaviour keeps its individual enabled flag", childProbe.enabled);

            parent.SetActive(true);
            Check.That("Child is active in hierarchy after the parent is reactivated", child.activeInHierarchy);
            Check.That("Child behaviour is active and enabled after the parent is reactivated", childProbe.isActiveAndEnabled);
            Check.Equal("Reactivating the parent fires OnEnable on the parent behaviour", 1, parentProbe.EnableCount);
            Check.Equal("Reactivating the parent fires OnEnable on the child behaviour", 1, childProbe.EnableCount);
            Check.That("The child behaviour is still individually enabled", childProbe.enabled);
        }

        private static void TestDisabledChildBehaviourIsPreserved()
        {
            GameObject parent = new GameObject("parent-of-disabled");
            GameObject child = new GameObject("disabled-child");
            child.transform.parent = parent.transform;

            ActivationProbeScript parentProbe = parent.AddComponent<ActivationProbeScript>();
            ActivationProbeScript childProbe = child.AddComponent<ActivationProbeScript>();
            childProbe.enabled = false;

            parent.SetActive(false);
            parent.SetActive(true);

            Check.Equal("A disabled child behaviour receives no OnDisable", 0, childProbe.DisableCount);
            Check.Equal("A disabled child behaviour receives no OnEnable", 0, childProbe.EnableCount);
            Check.That("A disabled child behaviour stays not active and enabled", !childProbe.isActiveAndEnabled);
            Check.That("A disabled child behaviour keeps its individual enabled flag as false", !childProbe.enabled);
            Check.Equal("The enabled parent behaviour still receives OnDisable", 1, parentProbe.DisableCount);
            Check.Equal("The enabled parent behaviour still receives OnEnable", 1, parentProbe.EnableCount);
        }

        private static void TestBehaviourUnderInactiveAncestor()
        {
            GameObject parent = new GameObject("inactive-parent");
            GameObject child = new GameObject("child-under-inactive-parent");
            child.transform.parent = parent.transform;

            ActivationProbeScript childProbe = child.AddComponent<ActivationProbeScript>();

            parent.SetActive(false);
            Check.That("A behaviour under an inactive ancestor is not active in hierarchy", !child.activeInHierarchy);
            Check.That("A behaviour under an inactive ancestor is not active and enabled", !childProbe.isActiveAndEnabled);

            parent.SetActive(true);
            Check.That("A behaviour is active and enabled again after the ancestor is reactivated", childProbe.isActiveAndEnabled);
        }

        private static void TestOwnActivationUnderInactiveAncestorRaisesNoCallback()
        {
            GameObject parent = new GameObject("inactive-ancestor");
            GameObject child = new GameObject("descendant-of-inactive-ancestor");
            child.transform.parent = parent.transform;

            ActivationProbeScript childProbe = child.AddComponent<ActivationProbeScript>();

            parent.SetActive(false);
            Check.Equal("Deactivating the ancestor notifies the descendant once", 1, childProbe.DisableCount);

            child.SetActive(false);
            Check.That("The descendant stays inactive in hierarchy under an inactive ancestor", !child.activeInHierarchy);
            Check.Equal("Changing the own active state under an inactive ancestor fires no OnDisable", 1, childProbe.DisableCount);
            Check.Equal("Changing the own active state under an inactive ancestor fires no OnEnable", 0, childProbe.EnableCount);

            child.SetActive(true);
            Check.That("The descendant stays inactive in hierarchy while the ancestor is inactive", !child.activeInHierarchy);
            Check.Equal("Restoring the own active state under an inactive ancestor fires no OnDisable", 1, childProbe.DisableCount);
            Check.Equal("Restoring the own active state under an inactive ancestor fires no OnEnable", 0, childProbe.EnableCount);
        }

        private static void TestGrandchildIsNotified()
        {
            GameObject parent = new GameObject("grandparent");
            GameObject child = new GameObject("child-of-grandparent");
            GameObject grandchild = new GameObject("grandchild");
            child.transform.parent = parent.transform;
            grandchild.transform.parent = child.transform;

            ActivationProbeScript childProbe = child.AddComponent<ActivationProbeScript>();
            ActivationProbeScript grandchildProbe = grandchild.AddComponent<ActivationProbeScript>();

            Check.That("Grandchild is active in hierarchy while the parent is active", grandchild.activeInHierarchy);

            parent.SetActive(false);
            Check.Equal("Deactivating the parent fires OnDisable on the child behaviour", 1, childProbe.DisableCount);
            Check.Equal("Deactivating the parent fires OnDisable on the grandchild behaviour", 1, grandchildProbe.DisableCount);
            Check.That("Grandchild is not active in hierarchy under an inactive parent", !grandchild.activeInHierarchy);
            Check.That("Grandchild behaviour is not active and enabled under an inactive parent", !grandchildProbe.isActiveAndEnabled);

            parent.SetActive(true);
            Check.Equal("Reactivating the parent fires OnEnable on the child behaviour", 1, childProbe.EnableCount);
            Check.Equal("Reactivating the parent fires OnEnable on the grandchild behaviour", 1, grandchildProbe.EnableCount);
            Check.That("Grandchild is active in hierarchy after the parent is reactivated", grandchild.activeInHierarchy);
            Check.That("Grandchild behaviour is active and enabled after the parent is reactivated", grandchildProbe.isActiveAndEnabled);
        }
    }
}
