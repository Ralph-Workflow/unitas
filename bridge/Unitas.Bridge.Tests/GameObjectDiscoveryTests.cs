using System;
using System.Collections.Generic;
using Unitas.Bridge;
using UnityEngine;
using UnityEngine.SceneManagement;

namespace Unitas.Bridge.Tests
{
    public class BaseLookupProbe : MonoBehaviour
    {
        public int Id;
    }

    public sealed class DerivedLookupProbe : BaseLookupProbe
    {
        public string Label = string.Empty;
    }

    public sealed class AlternateLookupProbe : MonoBehaviour
    {
        public string Marker = string.Empty;
    }

    public abstract class AbstractLookupComponent : Component
    {
    }

    public sealed class NoDefaultCtorLookupComponent : Component
    {
        public int Value { get; }

        public NoDefaultCtorLookupComponent(int value)
        {
            Value = value;
        }
    }

    public static class GameObjectDiscoveryTests
    {
        public static void Run()
        {
            TestGameObjectFindPlainAndHierarchyPaths();
            TestGameObjectFindExcludesInactiveAndDestroyedAndPreservesOrder();
            TestGameObjectTagAndFindWithTagFamily();
            TestGetComponentsGenericAndNonGeneric();
            TestGetComponentInChildrenGenericAndNonGeneric();
            TestGetComponentInParentGenericAndNonGeneric();
            TestNonGenericGetComponentAndAddComponent();
            TestAddComponentMonoBehaviourDoesNotLeakTemporaryOwner();
            TestSceneUnloadAndSingleLoadDeregisterNonPersistentObjects();
        }

        private static void TestGameObjectFindPlainAndHierarchyPaths()
        {
            List<UnityEngine.Object> created = new List<UnityEngine.Object>();
            try
            {
                GameObject root = Track(created, new GameObject("RootNode"));
                GameObject child = Track(created, new GameObject("ChildNode"));
                child.transform.parent = root.transform;
                GameObject grandchild = Track(created, new GameObject("GrandchildNode"));
                grandchild.transform.parent = child.transform;

                Check.That("Find plain root name returns root", ReferenceEquals(GameObject.Find("RootNode"), root));
                Check.That("Find plain child name returns child", ReferenceEquals(GameObject.Find("ChildNode"), child));
                Check.That("Find plain grandchild name returns grandchild", ReferenceEquals(GameObject.Find("GrandchildNode"), grandchild));
                Check.That("Find two-segment path RootNode/ChildNode returns child", ReferenceEquals(GameObject.Find("RootNode/ChildNode"), child));
                Check.That("Find three-segment path RootNode/ChildNode/GrandchildNode returns grandchild", ReferenceEquals(GameObject.Find("RootNode/ChildNode/GrandchildNode"), grandchild));
                Check.That("Find relative two-segment path ChildNode/GrandchildNode returns grandchild", ReferenceEquals(GameObject.Find("ChildNode/GrandchildNode"), grandchild));
                Check.That("Find leading-slash root path /RootNode/ChildNode returns child", ReferenceEquals(GameObject.Find("/RootNode/ChildNode"), child));
                Check.That("Find leading-slash three-segment path /RootNode/ChildNode/GrandchildNode returns grandchild", ReferenceEquals(GameObject.Find("/RootNode/ChildNode/GrandchildNode"), grandchild));
                Check.That("Find leading-slash non-root /ChildNode/GrandchildNode returns null", GameObject.Find("/ChildNode/GrandchildNode") == null);
                Check.That("Find missing plain name returns null", GameObject.Find("NonExistentObjectName") == null);
                Check.That("Find missing child segment returns null", GameObject.Find("RootNode/MissingChild") == null);
                Check.That("Find empty string returns null", GameObject.Find(string.Empty) == null);
                Check.That("Find slash-only string returns null", GameObject.Find("/") == null);
                Check.That("Find path with empty segment returns null", GameObject.Find("RootNode//ChildNode") == null);
                CheckThrowsArgumentNull("Find(null) throws ArgumentNullException", () => GameObject.Find(null!));
            }
            finally
            {
                DestroyAll(created);
            }
        }

        private static void TestGameObjectFindExcludesInactiveAndDestroyedAndPreservesOrder()
        {
            List<UnityEngine.Object> created = new List<UnityEngine.Object>();
            try
            {
                GameObject firstDuplicate = Track(created, new GameObject("DuplicateName"));
                GameObject secondDuplicate = Track(created, new GameObject("DuplicateName"));

                Check.That("Find returns first created active object when names collide", ReferenceEquals(GameObject.Find("DuplicateName"), firstDuplicate));

                firstDuplicate.SetActive(false);
                Check.That("Find skips inactive first match and returns second active match", ReferenceEquals(GameObject.Find("DuplicateName"), secondDuplicate));

                firstDuplicate.SetActive(true);
                UnityEngine.Object.Destroy(firstDuplicate);
                Check.That("Find skips destroyed first match and returns surviving match", ReferenceEquals(GameObject.Find("DuplicateName"), secondDuplicate));

                GameObject parent = Track(created, new GameObject("PathParent"));
                GameObject child = Track(created, new GameObject("PathChild"));
                child.transform.parent = parent.transform;

                child.SetActive(false);
                Check.That("Find path returns null when target child is inactive", GameObject.Find("PathParent/PathChild") == null);

                child.SetActive(true);
                parent.SetActive(false);
                Check.That("Find path returns null when ancestor is inactive", GameObject.Find("PathParent/PathChild") == null);
                Check.That("Find plain child name returns null when ancestor is inactive", GameObject.Find("PathChild") == null);
            }
            finally
            {
                DestroyAll(created);
            }
        }

        private static void TestGameObjectTagAndFindWithTagFamily()
        {
            List<UnityEngine.Object> created = new List<UnityEngine.Object>();
            try
            {
                GameObject defaultTagged = Track(created, new GameObject("DefaultTagged"));
                Check.Equal("new GameObject default tag is Untagged", "Untagged", defaultTagged.tag);

                GameObject firstPlayer = Track(created, new GameObject("FirstPlayer"));
                firstPlayer.tag = "Player";
                Check.Equal("tag round-trips custom value", "Player", firstPlayer.tag);

                GameObject secondPlayer = Track(created, new GameObject("SecondPlayer"));
                secondPlayer.tag = "Player";

                GameObject inactivePlayer = Track(created, new GameObject("InactivePlayer"));
                inactivePlayer.tag = "Player";
                inactivePlayer.SetActive(false);

                GameObject enemy = Track(created, new GameObject("EnemyUnit"));
                enemy.tag = "Enemy";

                Check.That("FindWithTag returns first active match in creation order", ReferenceEquals(GameObject.FindWithTag("Player"), firstPlayer));
                Check.That("FindGameObjectWithTag returns first active match in creation order", ReferenceEquals(GameObject.FindGameObjectWithTag("Player"), firstPlayer));
                Check.That("FindWithTag returns null when no active object has tag", GameObject.FindWithTag("NonExistentTag") == null);

                GameObject[] players = GameObject.FindGameObjectsWithTag("Player");
                Check.Equal("FindGameObjectsWithTag returns only active matching objects", 2, players.Length);
                if (players.Length == 2)
                {
                    Check.That("FindGameObjectsWithTag index 0 is first created", ReferenceEquals(players[0], firstPlayer));
                    Check.That("FindGameObjectsWithTag index 1 is second created", ReferenceEquals(players[1], secondPlayer));
                }

                UnityEngine.Object.Destroy(firstPlayer);
                Check.That("FindWithTag skips destroyed object", ReferenceEquals(GameObject.FindWithTag("Player"), secondPlayer));
                GameObject[] survivingPlayers = GameObject.FindGameObjectsWithTag("Player");
                Check.Equal("FindGameObjectsWithTag excludes destroyed object", 1, survivingPlayers.Length);

                GameObject[] missingTag = GameObject.FindGameObjectsWithTag("MissingTag");
                Check.Equal("FindGameObjectsWithTag returns empty array when no object matches", 0, missingTag.Length);

                CheckThrowsArgumentNull("setting tag to null throws ArgumentNullException", () => defaultTagged.tag = null!);
                CheckThrowsArgumentNull("FindWithTag(null) throws ArgumentNullException", () => GameObject.FindWithTag(null!));
                CheckThrowsArgumentNull("FindGameObjectWithTag(null) throws ArgumentNullException", () => GameObject.FindGameObjectWithTag(null!));
                CheckThrowsArgumentNull("FindGameObjectsWithTag(null) throws ArgumentNullException", () => GameObject.FindGameObjectsWithTag(null!));
            }
            finally
            {
                DestroyAll(created);
            }
        }

        private static void TestGetComponentsGenericAndNonGeneric()
        {
            List<UnityEngine.Object> created = new List<UnityEngine.Object>();
            try
            {
                GameObject host = Track(created, new GameObject("ComponentsHost"));
                BaseLookupProbe first = host.AddComponent<BaseLookupProbe>();
                first.Id = 1;
                DerivedLookupProbe second = host.AddComponent<DerivedLookupProbe>();
                second.Id = 2;
                second.Label = "derived";
                BaseLookupProbe third = host.AddComponent<BaseLookupProbe>();
                third.Id = 3;

                BaseLookupProbe[] genericProbes = host.GetComponents<BaseLookupProbe>();
                Check.Equal("GameObject.GetComponents<BaseLookupProbe> returns all matching base and derived components", 3, genericProbes.Length);
                if (genericProbes.Length == 3)
                {
                    Check.That("GetComponents<T> index 0 matches attachment order", ReferenceEquals(genericProbes[0], first));
                    Check.That("GetComponents<T> index 1 matches derived component in attachment order", ReferenceEquals(genericProbes[1], second));
                    Check.That("GetComponents<T> index 2 matches attachment order", ReferenceEquals(genericProbes[2], third));
                }

                BaseLookupProbe[] viaComponent = host.transform.GetComponents<BaseLookupProbe>();
                Check.Equal("Component.GetComponents<T> delegates to GameObject", 3, viaComponent.Length);

                Component[] nonGenericProbes = host.GetComponents(typeof(BaseLookupProbe));
                Check.Equal("GameObject.GetComponents(Type) returns all matching components", 3, nonGenericProbes.Length);
                if (nonGenericProbes.Length == 3)
                {
                    Check.That("GetComponents(Type) index 0 matches attachment order", ReferenceEquals(nonGenericProbes[0], first));
                    Check.That("GetComponents(Type) index 1 matches attachment order", ReferenceEquals(nonGenericProbes[1], second));
                    Check.That("GetComponents(Type) index 2 matches attachment order", ReferenceEquals(nonGenericProbes[2], third));
                }

                Component[] nonGenericViaComponent = host.transform.GetComponents(typeof(DerivedLookupProbe));
                Check.Equal("Component.GetComponents(Type) filters to derived type", 1, nonGenericViaComponent.Length);
                if (nonGenericViaComponent.Length == 1)
                {
                    Check.That("Component.GetComponents(Type) returns derived instance", ReferenceEquals(nonGenericViaComponent[0], second));
                }

                UnityEngine.Object.Destroy(second);
                BaseLookupProbe[] afterDestroy = host.GetComponents<BaseLookupProbe>();
                Check.Equal("GetComponents<T> excludes destroyed component", 2, afterDestroy.Length);
                Check.That("GetComponent<DerivedLookupProbe> returns null after component is destroyed", host.GetComponent<DerivedLookupProbe>() == null);
                Check.That("GetComponent(typeof(DerivedLookupProbe)) returns null after component is destroyed", host.GetComponent(typeof(DerivedLookupProbe)) == null);

                AlternateLookupProbe[] none = host.GetComponents<AlternateLookupProbe>();
                Check.Equal("GetComponents<T> returns empty array when none match", 0, none.Length);

                CheckThrowsArgumentNull("GameObject.GetComponents(null) throws ArgumentNullException", () => host.GetComponents(null!));
                CheckThrowsArgumentException("GameObject.GetComponents(typeof(string)) throws ArgumentException", () => host.GetComponents(typeof(string)));
                CheckThrowsArgumentNull("Component.GetComponents(null) throws ArgumentNullException", () => host.transform.GetComponents(null!));
                CheckThrowsArgumentException("Component.GetComponents(typeof(GameObject)) throws ArgumentException", () => host.transform.GetComponents(typeof(GameObject)));
            }
            finally
            {
                DestroyAll(created);
            }
        }

        private static void TestGetComponentInChildrenGenericAndNonGeneric()
        {
            List<UnityEngine.Object> created = new List<UnityEngine.Object>();
            try
            {
                GameObject root = Track(created, new GameObject("ChildrenRoot"));
                GameObject firstChild = Track(created, new GameObject("FirstChild"));
                firstChild.transform.parent = root.transform;
                GameObject grandchild = Track(created, new GameObject("Grandchild"));
                grandchild.transform.parent = firstChild.transform;
                GameObject secondChild = Track(created, new GameObject("SecondChild"));
                secondChild.transform.parent = root.transform;

                BaseLookupProbe grandchildProbe = grandchild.AddComponent<BaseLookupProbe>();
                grandchildProbe.Id = 20;
                BaseLookupProbe secondChildProbe = secondChild.AddComponent<BaseLookupProbe>();
                secondChildProbe.Id = 30;

                Check.That(
                    "GetComponentInChildren traverses depth-first (grandchild before second sibling)",
                    ReferenceEquals(root.GetComponentInChildren<BaseLookupProbe>(), grandchildProbe));
                Check.That(
                    "Component.GetComponentInChildren<T>() delegates to GameObject",
                    ReferenceEquals(root.transform.GetComponentInChildren<BaseLookupProbe>(), grandchildProbe));
                Check.That(
                    "GetComponentInChildren(Type) finds depth-first match",
                    ReferenceEquals(root.GetComponentInChildren(typeof(BaseLookupProbe)), grandchildProbe));
                Check.That(
                    "Component.GetComponentInChildren(Type) delegates to GameObject",
                    ReferenceEquals(root.transform.GetComponentInChildren(typeof(BaseLookupProbe)), grandchildProbe));

                firstChild.SetActive(false);
                Check.That(
                    "GetComponentInChildren skips inactive subtree when includeInactive is false",
                    ReferenceEquals(root.GetComponentInChildren<BaseLookupProbe>(false), secondChildProbe));
                Check.That(
                    "GetComponentInChildren(Type, false) skips inactive subtree",
                    ReferenceEquals(root.GetComponentInChildren(typeof(BaseLookupProbe), false), secondChildProbe));
                Check.That(
                    "GetComponentInChildren includes inactive subtree when includeInactive is true",
                    ReferenceEquals(root.GetComponentInChildren<BaseLookupProbe>(true), grandchildProbe));
                Check.That(
                    "Component.GetComponentInChildren(Type, true) includes inactive subtree",
                    ReferenceEquals(root.transform.GetComponentInChildren(typeof(BaseLookupProbe), true), grandchildProbe));

                firstChild.SetActive(true);
                BaseLookupProbe rootProbe = root.AddComponent<BaseLookupProbe>();
                rootProbe.Id = 10;
                Check.That(
                    "GetComponentInChildren checks self before children",
                    ReferenceEquals(root.GetComponentInChildren<BaseLookupProbe>(), rootProbe));

                root.SetActive(false);
                Check.That(
                    "GetComponentInChildren returns null on inactive self when includeInactive is false",
                    root.GetComponentInChildren<BaseLookupProbe>(false) == null);
                Check.That(
                    "GetComponentInChildren finds component on inactive self when includeInactive is true",
                    ReferenceEquals(root.GetComponentInChildren<BaseLookupProbe>(true), rootProbe));

                CheckThrowsArgumentNull("GetComponentInChildren(null) throws ArgumentNullException", () => root.GetComponentInChildren(null!));
                CheckThrowsArgumentException("GetComponentInChildren(typeof(string)) throws ArgumentException", () => root.GetComponentInChildren(typeof(string)));
            }
            finally
            {
                DestroyAll(created);
            }
        }

        private static void TestGetComponentInParentGenericAndNonGeneric()
        {
            List<UnityEngine.Object> created = new List<UnityEngine.Object>();
            try
            {
                GameObject root = Track(created, new GameObject("ParentRoot"));
                GameObject middle = Track(created, new GameObject("ParentMiddle"));
                middle.transform.parent = root.transform;
                GameObject leaf = Track(created, new GameObject("ParentLeaf"));
                leaf.transform.parent = middle.transform;

                BaseLookupProbe rootProbe = root.AddComponent<BaseLookupProbe>();
                rootProbe.Id = 100;

                Check.That(
                    "GetComponentInParent walks up to grandparent",
                    ReferenceEquals(leaf.GetComponentInParent<BaseLookupProbe>(), rootProbe));
                Check.That(
                    "Component.GetComponentInParent<T>() delegates to GameObject",
                    ReferenceEquals(leaf.transform.GetComponentInParent<BaseLookupProbe>(), rootProbe));
                Check.That(
                    "GetComponentInParent(Type) walks up to grandparent",
                    ReferenceEquals(leaf.GetComponentInParent(typeof(BaseLookupProbe)), rootProbe));
                Check.That(
                    "Component.GetComponentInParent(Type) delegates to GameObject",
                    ReferenceEquals(leaf.transform.GetComponentInParent(typeof(BaseLookupProbe)), rootProbe));

                BaseLookupProbe middleProbe = middle.AddComponent<BaseLookupProbe>();
                middleProbe.Id = 200;
                Check.That(
                    "GetComponentInParent returns nearest ancestor match",
                    ReferenceEquals(leaf.GetComponentInParent<BaseLookupProbe>(), middleProbe));

                middle.SetActive(false);
                Check.That(
                    "GetComponentInParent skips inactive middle ancestor and finds active root when includeInactive is false",
                    ReferenceEquals(leaf.GetComponentInParent<BaseLookupProbe>(false), rootProbe));
                Check.That(
                    "GetComponentInParent finds inactive middle ancestor first when includeInactive is true",
                    ReferenceEquals(leaf.GetComponentInParent<BaseLookupProbe>(true), middleProbe));
                Check.That(
                    "Component.GetComponentInParent(Type, true) finds inactive middle ancestor",
                    ReferenceEquals(leaf.transform.GetComponentInParent(typeof(BaseLookupProbe), true), middleProbe));

                root.SetActive(false);
                Check.That(
                    "GetComponentInParent returns null when all matching ancestors are inactive and includeInactive is false",
                    leaf.GetComponentInParent<BaseLookupProbe>(false) == null);
                root.SetActive(true);

                middle.SetActive(true);
                BaseLookupProbe leafProbe = leaf.AddComponent<BaseLookupProbe>();
                leafProbe.Id = 300;
                Check.That(
                    "GetComponentInParent checks self first",
                    ReferenceEquals(leaf.GetComponentInParent<BaseLookupProbe>(), leafProbe));

                leaf.SetActive(false);
                Check.That(
                    "GetComponentInParent skips inactive self and returns active parent match when includeInactive is false",
                    ReferenceEquals(leaf.GetComponentInParent<BaseLookupProbe>(false), middleProbe));
                Check.That(
                    "GetComponentInParent returns inactive self when includeInactive is true",
                    ReferenceEquals(leaf.GetComponentInParent<BaseLookupProbe>(true), leafProbe));

                CheckThrowsArgumentNull("GetComponentInParent(null) throws ArgumentNullException", () => leaf.GetComponentInParent(null!));
                CheckThrowsArgumentException("GetComponentInParent(typeof(int)) throws ArgumentException", () => leaf.GetComponentInParent(typeof(int)));
            }
            finally
            {
                DestroyAll(created);
            }
        }

        private static void TestNonGenericGetComponentAndAddComponent()
        {
            List<UnityEngine.Object> created = new List<UnityEngine.Object>();
            try
            {
                GameObject go = Track(created, new GameObject("NonGenericTarget"));
                Component added = go.AddComponent(typeof(DerivedLookupProbe));
                Check.That("AddComponent(Type) returns created component", added is DerivedLookupProbe);
                Check.That("AddComponent(Type) binds component.gameObject", ReferenceEquals(added.gameObject, go));
                Check.That("AddComponent(Type) binds component.scene", added.scene == go.scene);

                Component? fetchedBase = go.GetComponent(typeof(BaseLookupProbe));
                Check.That("GetComponent(Type) matches derived component by base type", ReferenceEquals(fetchedBase, added));
                Check.That("Component.GetComponent(Type) delegates to GameObject", ReferenceEquals(go.transform.GetComponent(typeof(DerivedLookupProbe)), added));

                Component? addedViaComponent = go.transform.AddComponent(typeof(AlternateLookupProbe));
                Check.That("Component.AddComponent(Type) attaches component to owning GameObject", addedViaComponent is AlternateLookupProbe);
                Check.That("GetComponent(Type) retrieves component added via Component.AddComponent(Type)", ReferenceEquals(go.GetComponent(typeof(AlternateLookupProbe)), addedViaComponent));

                CheckThrowsArgumentNull("GameObject.GetComponent(null) throws ArgumentNullException", () => go.GetComponent(null!));
                CheckThrowsArgumentException("GameObject.GetComponent(typeof(string)) throws ArgumentException", () => go.GetComponent(typeof(string)));
                CheckThrowsArgumentException("GameObject.GetComponent(typeof(GameObject)) throws ArgumentException", () => go.GetComponent(typeof(GameObject)));

                CheckThrowsArgumentNull("GameObject.AddComponent(null) throws ArgumentNullException", () => go.AddComponent(null!));
                CheckThrowsArgumentException("GameObject.AddComponent(typeof(string)) throws ArgumentException", () => go.AddComponent(typeof(string)));
                CheckThrowsArgumentException("GameObject.AddComponent(typeof(GameObject)) throws ArgumentException", () => go.AddComponent(typeof(GameObject)));
                CheckThrowsArgumentException("GameObject.AddComponent(abstract Component) throws ArgumentException", () => go.AddComponent(typeof(AbstractLookupComponent)));
                CheckThrowsArgumentException("GameObject.AddComponent(Component without default ctor) throws ArgumentException", () => go.AddComponent(typeof(NoDefaultCtorLookupComponent)));
            }
            finally
            {
                DestroyAll(created);
            }
        }

        private static void TestAddComponentMonoBehaviourDoesNotLeakTemporaryOwner()
        {
            List<UnityEngine.Object> created = new List<UnityEngine.Object>();
            try
            {
                int scriptedBefore = CountGameObjectsNamed("Scripted Game Object");
                int sphereCollidersBefore = UnityEngine.Object.FindObjectsOfType<SphereCollider>(true).Length;

                GameObject host = Track(created, new GameObject("RealOwner"));
                DerivedLookupProbe genericAdded = host.AddComponent<DerivedLookupProbe>();
                Component typeAdded = host.AddComponent(typeof(AlternateLookupProbe));

                Check.That("generic AddComponent re-parents MonoBehaviour to host", ReferenceEquals(genericAdded.gameObject, host));
                Check.That("non-generic AddComponent re-parents MonoBehaviour to host", ReferenceEquals(typeAdded.gameObject, host));
                Check.Equal("AddComponent<MonoBehaviour> does not leave temporary Scripted Game Object in live registry", scriptedBefore, CountGameObjectsNamed("Scripted Game Object"));
                Check.Equal("AddComponent<MonoBehaviour> does not leak temporary SphereCollider in live registry", sphereCollidersBefore, UnityEngine.Object.FindObjectsOfType<SphereCollider>(true).Length);

                GameObject clonedHost = Track(created, UnityEngine.Object.Instantiate(host));
                Check.Equal("Instantiate keep original MonoBehaviour count on source", 1, host.GetComponents<DerivedLookupProbe>().Length);
                Check.Equal("Instantiate attaches cloned MonoBehaviour to clone", 1, clonedHost.GetComponents<DerivedLookupProbe>().Length);
                Check.That("Instantiate does not remove original MonoBehaviour from source", ReferenceEquals(host.GetComponent<DerivedLookupProbe>(), genericAdded));
            }
            finally
            {
                DestroyAll(created);
            }
        }

        private static void TestSceneUnloadAndSingleLoadDeregisterNonPersistentObjects()
        {
            List<UnityEngine.Object> created = new List<UnityEngine.Object>();
            FakeSceneProvider provider = new FakeSceneProvider("assets/Level1.unity", "assets/Level2.unity", "assets/Level3.unity");
            FakeSceneBackend backend = new FakeSceneBackend();
            try
            {
                using (BridgeHost host = new BridgeHost())
                {
                    host.Provider = provider;
                    host.Backend = backend;

                    SceneManager.LoadScene("Level1", LoadSceneMode.Single);
                    GameObject level1Transient = Track(created, new GameObject("Level1_Transient"));
                    BaseLookupProbe level1Probe = level1Transient.AddComponent<BaseLookupProbe>();

                    GameObject persistentGo = Track(created, new GameObject("Persistent_Object"));
                    BaseLookupProbe persistentProbe = persistentGo.AddComponent<BaseLookupProbe>();
                    UnityEngine.Object.DontDestroyOnLoad(persistentGo);

                    SceneManager.LoadScene("Level2", LoadSceneMode.Additive);
                    GameObject level2Transient = Track(created, new GameObject("Level2_Transient"));
                    BaseLookupProbe level2Probe = level2Transient.AddComponent<BaseLookupProbe>();

                    Check.That("Level1 object is discoverable before unload", ReferenceEquals(GameObject.Find("Level1_Transient"), level1Transient));
                    Check.That("Level2 object is discoverable before unload", ReferenceEquals(GameObject.Find("Level2_Transient"), level2Transient));
                    Check.That("Persistent object is discoverable before unload", ReferenceEquals(GameObject.Find("Persistent_Object"), persistentGo));

                    SceneManager.UnloadScene("Level1");

                    Check.That("unloaded Level1 GameObject is no longer returned by GameObject.Find", GameObject.Find("Level1_Transient") == null);
                    BaseLookupProbe[] afterUnloadProbes = UnityEngine.Object.FindObjectsOfType<BaseLookupProbe>(true);
                    Check.That("unloaded Level1 component is removed from FindObjectsOfType", !ContainsReference(afterUnloadProbes, level1Probe));
                    Check.That("remaining Level2 component is still discoverable after Level1 unload", ContainsReference(afterUnloadProbes, level2Probe));
                    Check.That("DontDestroyOnLoad component survives Level1 unload", ContainsReference(afterUnloadProbes, persistentProbe));

                    SceneManager.LoadScene("Level3", LoadSceneMode.Single);

                    Check.That("Level2 GameObject is deregistered on LoadSceneMode.Single replacement", GameObject.Find("Level2_Transient") == null);
                    BaseLookupProbe[] afterSingleLoadProbes = UnityEngine.Object.FindObjectsOfType<BaseLookupProbe>(true);
                    Check.That("Level2 component is removed from FindObjectsOfType after LoadSceneMode.Single", !ContainsReference(afterSingleLoadProbes, level2Probe));
                    Check.That("DontDestroyOnLoad GameObject survives LoadSceneMode.Single", ReferenceEquals(GameObject.Find("Persistent_Object"), persistentGo));
                    Check.That("DontDestroyOnLoad component survives LoadSceneMode.Single", ContainsReference(afterSingleLoadProbes, persistentProbe));
                }
            }
            finally
            {
                DestroyAll(created);
            }
        }

        private static int CountGameObjectsNamed(string targetName)
        {
            GameObject[] all = UnityEngine.Object.FindObjectsOfType<GameObject>(true);
            int count = 0;
            for (int i = 0; i < all.Length; i++)
            {
                if (string.Equals(all[i].name, targetName, StringComparison.Ordinal))
                {
                    count++;
                }
            }

            return count;
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
