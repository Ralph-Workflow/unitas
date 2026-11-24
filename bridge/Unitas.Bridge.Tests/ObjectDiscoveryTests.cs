using System;
using System.Collections.Generic;
using UnityEngine;

namespace Unitas.Bridge.Tests
{
    public sealed class DiscoveryProbeScript : MonoBehaviour
    {
        public string TagLabel = string.Empty;
    }

    public sealed class SecondaryDiscoveryProbeScript : MonoBehaviour
    {
        public int Sequence;
    }

    public static class ObjectDiscoveryTests
    {
        public static void Run()
        {
            TestFindObjectsOfTypeFiltersInactiveByDefault();
            TestFindObjectsOfTypeIncludesInactiveWhenRequested();
            TestFindObjectsOfTypeHonorsAncestorActiveState();
            TestDestroyedObjectsAreNeverReturned();
            TestDeterministicCreationOrdering();
            TestFindObjectOfTypeReturnsFirstMatchOrNull();
            TestNonGenericTypeOverloadsAndValidation();
            TestInstantiatedClonesAreDiscoverableInCreationOrder();
        }

        private static void TestFindObjectsOfTypeFiltersInactiveByDefault()
        {
            List<UnityEngine.Object> created = new List<UnityEngine.Object>();
            try
            {
                GameObject activeGo = Track(created, new GameObject("Discovery_ActiveGo"));
                DiscoveryProbeScript activeProbe = activeGo.AddComponent<DiscoveryProbeScript>();
                activeProbe.TagLabel = "active";

                GameObject inactiveGo = Track(created, new GameObject("Discovery_InactiveGo"));
                DiscoveryProbeScript inactiveProbe = inactiveGo.AddComponent<DiscoveryProbeScript>();
                inactiveProbe.TagLabel = "inactive";
                inactiveGo.SetActive(false);

                DiscoveryProbeScript[] probes = UnityEngine.Object.FindObjectsOfType<DiscoveryProbeScript>();
                Check.That("active probe is returned by default FindObjectsOfType<T>()", ContainsReference(probes, activeProbe));
                Check.That("inactive probe is excluded by default FindObjectsOfType<T>()", !ContainsReference(probes, inactiveProbe));

                GameObject[] gameObjects = UnityEngine.Object.FindObjectsOfType<GameObject>();
                Check.That("active GameObject is returned by default FindObjectsOfType<GameObject>()", ContainsReference(gameObjects, activeGo));
                Check.That("inactive GameObject is excluded by default FindObjectsOfType<GameObject>()", !ContainsReference(gameObjects, inactiveGo));
            }
            finally
            {
                DestroyAll(created);
            }
        }

        private static void TestFindObjectsOfTypeIncludesInactiveWhenRequested()
        {
            List<UnityEngine.Object> created = new List<UnityEngine.Object>();
            try
            {
                GameObject activeGo = Track(created, new GameObject("Discovery_Include_Active"));
                DiscoveryProbeScript activeProbe = activeGo.AddComponent<DiscoveryProbeScript>();

                GameObject inactiveGo = Track(created, new GameObject("Discovery_Include_Inactive"));
                DiscoveryProbeScript inactiveProbe = inactiveGo.AddComponent<DiscoveryProbeScript>();
                inactiveGo.SetActive(false);

                DiscoveryProbeScript[] activeOnly = UnityEngine.Object.FindObjectsOfType<DiscoveryProbeScript>(false);
                Check.That("includeInactive false includes active component", ContainsReference(activeOnly, activeProbe));
                Check.That("includeInactive false excludes inactive component", !ContainsReference(activeOnly, inactiveProbe));

                DiscoveryProbeScript[] withInactive = UnityEngine.Object.FindObjectsOfType<DiscoveryProbeScript>(true);
                Check.That("includeInactive true includes active component", ContainsReference(withInactive, activeProbe));
                Check.That("includeInactive true includes inactive component", ContainsReference(withInactive, inactiveProbe));

                GameObject[] allGameObjects = UnityEngine.Object.FindObjectsOfType<GameObject>(true);
                Check.That("includeInactive true includes active GameObject", ContainsReference(allGameObjects, activeGo));
                Check.That("includeInactive true includes inactive GameObject", ContainsReference(allGameObjects, inactiveGo));
            }
            finally
            {
                DestroyAll(created);
            }
        }

        private static void TestFindObjectsOfTypeHonorsAncestorActiveState()
        {
            List<UnityEngine.Object> created = new List<UnityEngine.Object>();
            try
            {
                GameObject parent = Track(created, new GameObject("Discovery_Parent"));
                GameObject child = Track(created, new GameObject("Discovery_Child"));
                child.transform.parent = parent.transform;
                DiscoveryProbeScript childProbe = child.AddComponent<DiscoveryProbeScript>();

                parent.SetActive(false);
                Check.That("child keeps activeSelf true when parent is deactivated", child.activeSelf);
                Check.That("child activeInHierarchy is false when parent is deactivated", !child.activeInHierarchy);

                DiscoveryProbeScript[] activeOnly = UnityEngine.Object.FindObjectsOfType<DiscoveryProbeScript>(false);
                Check.That("child component under inactive parent is excluded when includeInactive is false", !ContainsReference(activeOnly, childProbe));

                GameObject[] activeGameObjects = UnityEngine.Object.FindObjectsOfType<GameObject>(false);
                Check.That("child GameObject under inactive parent is excluded when includeInactive is false", !ContainsReference(activeGameObjects, child));

                DiscoveryProbeScript[] withInactive = UnityEngine.Object.FindObjectsOfType<DiscoveryProbeScript>(true);
                Check.That("child component under inactive parent is included when includeInactive is true", ContainsReference(withInactive, childProbe));

                GameObject[] allGameObjects = UnityEngine.Object.FindObjectsOfType<GameObject>(true);
                Check.That("child GameObject under inactive parent is included when includeInactive is true", ContainsReference(allGameObjects, child));
            }
            finally
            {
                DestroyAll(created);
            }
        }

        private static void TestDestroyedObjectsAreNeverReturned()
        {
            List<UnityEngine.Object> created = new List<UnityEngine.Object>();
            try
            {
                GameObject survivingGo = Track(created, new GameObject("Discovery_Survivor"));
                DiscoveryProbeScript survivingProbe = survivingGo.AddComponent<DiscoveryProbeScript>();
                SecondaryDiscoveryProbeScript destroyedComponent = survivingGo.AddComponent<SecondaryDiscoveryProbeScript>();

                GameObject doomedGo = Track(created, new GameObject("Discovery_Doomed"));
                DiscoveryProbeScript doomedProbe = doomedGo.AddComponent<DiscoveryProbeScript>();

                UnityEngine.Object.Destroy(destroyedComponent);
                UnityEngine.Object.Destroy(doomedGo);

                DiscoveryProbeScript[] probesWithInactive = UnityEngine.Object.FindObjectsOfType<DiscoveryProbeScript>(true);
                Check.That("surviving probe remains discoverable", ContainsReference(probesWithInactive, survivingProbe));
                Check.That("probe on destroyed GameObject is not discoverable even with includeInactive true", !ContainsReference(probesWithInactive, doomedProbe));

                SecondaryDiscoveryProbeScript[] secondaryWithInactive = UnityEngine.Object.FindObjectsOfType<SecondaryDiscoveryProbeScript>(true);
                Check.That("destroyed component on surviving GameObject is not discoverable", !ContainsReference(secondaryWithInactive, destroyedComponent));

                GameObject[] gameObjectsWithInactive = UnityEngine.Object.FindObjectsOfType<GameObject>(true);
                Check.That("destroyed GameObject is not discoverable even with includeInactive true", !ContainsReference(gameObjectsWithInactive, doomedGo));
            }
            finally
            {
                DestroyAll(created);
            }
        }

        private static void TestDeterministicCreationOrdering()
        {
            List<UnityEngine.Object> created = new List<UnityEngine.Object>();
            try
            {
                GameObject firstGo = Track(created, new GameObject("Zeta"));
                SecondaryDiscoveryProbeScript first = firstGo.AddComponent<SecondaryDiscoveryProbeScript>();
                first.Sequence = 1;

                GameObject secondGo = Track(created, new GameObject("Alpha"));
                SecondaryDiscoveryProbeScript second = secondGo.AddComponent<SecondaryDiscoveryProbeScript>();
                second.Sequence = 2;

                GameObject thirdGo = Track(created, new GameObject("Middle"));
                SecondaryDiscoveryProbeScript third = thirdGo.AddComponent<SecondaryDiscoveryProbeScript>();
                third.Sequence = 3;

                SecondaryDiscoveryProbeScript[] firstCall = UnityEngine.Object.FindObjectsOfType<SecondaryDiscoveryProbeScript>();
                SecondaryDiscoveryProbeScript[] secondCall = UnityEngine.Object.FindObjectsOfType<SecondaryDiscoveryProbeScript>();

                Check.Equal("FindObjectsOfType returns all three created probes", 3, firstCall.Length);
                Check.Equal("consecutive FindObjectsOfType calls return same count", firstCall.Length, secondCall.Length);

                if (firstCall.Length == 3 && secondCall.Length == 3)
                {
                    Check.That("order follows creation index 0", ReferenceEquals(firstCall[0], first));
                    Check.That("order follows creation index 1", ReferenceEquals(firstCall[1], second));
                    Check.That("order follows creation index 2", ReferenceEquals(firstCall[2], third));
                    Check.That(
                        "consecutive calls return identical sequence",
                        ReferenceEquals(firstCall[0], secondCall[0])
                        && ReferenceEquals(firstCall[1], secondCall[1])
                        && ReferenceEquals(firstCall[2], secondCall[2]));
                }
            }
            finally
            {
                DestroyAll(created);
            }
        }

        private static void TestFindObjectOfTypeReturnsFirstMatchOrNull()
        {
            List<UnityEngine.Object> created = new List<UnityEngine.Object>();
            try
            {
                SecondaryDiscoveryProbeScript? noneActive = UnityEngine.Object.FindObjectOfType<SecondaryDiscoveryProbeScript>();
                SecondaryDiscoveryProbeScript? noneAll = UnityEngine.Object.FindObjectOfType<SecondaryDiscoveryProbeScript>(true);
                Check.That("FindObjectOfType<T>() returns null when no instance exists", noneActive == null);
                Check.That("FindObjectOfType<T>(true) returns null when no instance exists", noneAll == null);

                GameObject firstGo = Track(created, new GameObject("FirstProbeHolder"));
                SecondaryDiscoveryProbeScript firstProbe = firstGo.AddComponent<SecondaryDiscoveryProbeScript>();
                firstGo.SetActive(false);

                GameObject secondGo = Track(created, new GameObject("SecondProbeHolder"));
                SecondaryDiscoveryProbeScript secondProbe = secondGo.AddComponent<SecondaryDiscoveryProbeScript>();

                SecondaryDiscoveryProbeScript? foundActive = UnityEngine.Object.FindObjectOfType<SecondaryDiscoveryProbeScript>();
                Check.That("FindObjectOfType<T>() skips inactive first object and returns first active match", ReferenceEquals(foundActive, secondProbe));

                SecondaryDiscoveryProbeScript? foundIncludingInactive = UnityEngine.Object.FindObjectOfType<SecondaryDiscoveryProbeScript>(true);
                Check.That("FindObjectOfType<T>(true) returns the earliest created match even if inactive", ReferenceEquals(foundIncludingInactive, firstProbe));
            }
            finally
            {
                DestroyAll(created);
            }
        }

        private static void TestNonGenericTypeOverloadsAndValidation()
        {
            List<UnityEngine.Object> created = new List<UnityEngine.Object>();
            try
            {
                GameObject firstGo = Track(created, new GameObject("TypeLookup_First"));
                SecondaryDiscoveryProbeScript firstProbe = firstGo.AddComponent<SecondaryDiscoveryProbeScript>();
                firstGo.SetActive(false);

                GameObject secondGo = Track(created, new GameObject("TypeLookup_Second"));
                SecondaryDiscoveryProbeScript secondProbe = secondGo.AddComponent<SecondaryDiscoveryProbeScript>();

                UnityEngine.Object[] activeMatches = UnityEngine.Object.FindObjectsOfType(typeof(SecondaryDiscoveryProbeScript));
                Check.Equal("non-generic FindObjectsOfType(Type) returns active matches", 1, activeMatches.Length);
                if (activeMatches.Length == 1)
                {
                    Check.That("non-generic FindObjectsOfType(Type) matches active probe", ReferenceEquals(activeMatches[0], secondProbe));
                }

                UnityEngine.Object[] allMatches = UnityEngine.Object.FindObjectsOfType(typeof(SecondaryDiscoveryProbeScript), true);
                Check.Equal("non-generic FindObjectsOfType(Type, true) returns active and inactive matches", 2, allMatches.Length);
                if (allMatches.Length == 2)
                {
                    Check.That("non-generic FindObjectsOfType(Type, true) index 0 is first created", ReferenceEquals(allMatches[0], firstProbe));
                    Check.That("non-generic FindObjectsOfType(Type, true) index 1 is second created", ReferenceEquals(allMatches[1], secondProbe));
                }

                UnityEngine.Object? firstActive = UnityEngine.Object.FindObjectOfType(typeof(SecondaryDiscoveryProbeScript));
                Check.That("non-generic FindObjectOfType(Type) returns first active match", ReferenceEquals(firstActive, secondProbe));

                UnityEngine.Object? firstOverall = UnityEngine.Object.FindObjectOfType(typeof(SecondaryDiscoveryProbeScript), true);
                Check.That("non-generic FindObjectOfType(Type, true) returns first overall match", ReferenceEquals(firstOverall, firstProbe));

                CheckThrowsArgumentNull("FindObjectsOfType(null) throws ArgumentNullException", () => UnityEngine.Object.FindObjectsOfType(null!));
                CheckThrowsArgumentNull("FindObjectsOfType(null, true) throws ArgumentNullException", () => UnityEngine.Object.FindObjectsOfType(null!, true));
                CheckThrowsArgumentNull("FindObjectOfType(null) throws ArgumentNullException", () => UnityEngine.Object.FindObjectOfType(null!));
                CheckThrowsArgumentNull("FindObjectOfType(null, true) throws ArgumentNullException", () => UnityEngine.Object.FindObjectOfType(null!, true));

                CheckThrowsArgumentException("FindObjectsOfType(typeof(string)) throws ArgumentException", () => UnityEngine.Object.FindObjectsOfType(typeof(string)));
                CheckThrowsArgumentException("FindObjectsOfType(typeof(string), true) throws ArgumentException", () => UnityEngine.Object.FindObjectsOfType(typeof(string), true));
                CheckThrowsArgumentException("FindObjectOfType(typeof(string)) throws ArgumentException", () => UnityEngine.Object.FindObjectOfType(typeof(string)));
                CheckThrowsArgumentException("FindObjectOfType(typeof(string), true) throws ArgumentException", () => UnityEngine.Object.FindObjectOfType(typeof(string), true));
            }
            finally
            {
                DestroyAll(created);
            }
        }

        private static void TestInstantiatedClonesAreDiscoverableInCreationOrder()
        {
            List<UnityEngine.Object> created = new List<UnityEngine.Object>();
            try
            {
                GameObject original = Track(created, new GameObject("CloneSource"));
                SecondaryDiscoveryProbeScript originalProbe = original.AddComponent<SecondaryDiscoveryProbeScript>();
                originalProbe.Sequence = 10;

                GameObject clone = Track(created, UnityEngine.Object.Instantiate(original));
                SecondaryDiscoveryProbeScript? cloneProbe = clone.GetComponent<SecondaryDiscoveryProbeScript>();
                Check.That("clone has cloned probe component", cloneProbe != null);

                SecondaryDiscoveryProbeScript[] foundProbes = UnityEngine.Object.FindObjectsOfType<SecondaryDiscoveryProbeScript>();
                Check.Equal("both original and cloned MonoBehaviour appear in FindObjectsOfType", 2, foundProbes.Length);
                if (foundProbes.Length == 2 && cloneProbe != null)
                {
                    Check.That("original probe is returned first", ReferenceEquals(foundProbes[0], originalProbe));
                    Check.That("cloned probe is returned second", ReferenceEquals(foundProbes[1], cloneProbe));
                    Check.That("cloned probe has distinct instance ID", originalProbe.GetInstanceID() != cloneProbe.GetInstanceID());
                }
            }
            finally
            {
                DestroyAll(created);
            }
        }

        private static GameObject Track(List<UnityEngine.Object> list, GameObject instance)
        {
            list.Add(instance);
            return instance;
        }

        private static void DestroyAll(List<UnityEngine.Object> list)
        {
            for (int i = list.Count - 1; i >= 0; i--)
            {
                UnityEngine.Object.Destroy(list[i]);
            }
        }

        private static bool ContainsReference<T>(T[] array, T target) where T : class
        {
            for (int i = 0; i < array.Length; i++)
            {
                if (ReferenceEquals(array[i], target))
                {
                    return true;
                }
            }

            return false;
        }

        private static void CheckThrowsArgumentNull(string label, Action action)
        {
            bool threw = false;
            try
            {
                action();
            }
            catch (ArgumentNullException)
            {
                threw = true;
            }

            Check.That(label, threw);
        }

        private static void CheckThrowsArgumentException(string label, Action action)
        {
            bool threw = false;
            try
            {
                action();
            }
            catch (ArgumentNullException)
            {
                threw = false;
            }
            catch (ArgumentException)
            {
                threw = true;
            }

            Check.That(label, threw);
        }
    }
}
