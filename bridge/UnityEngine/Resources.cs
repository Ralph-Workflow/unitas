using System;
using System.Collections.Generic;
using System.IO;
using System.Text;

namespace UnityEngine
{
    public class TextAsset : Object
    {
        private readonly string _text;
        private readonly byte[] _bytes;

        public TextAsset() : this(string.Empty)
        {
        }

        public TextAsset(string text)
        {
            _text = text ?? string.Empty;
            _bytes = Encoding.UTF8.GetBytes(_text);
        }

        public TextAsset(string text, byte[] bytes)
        {
            _text = text ?? string.Empty;
            _bytes = bytes != null ? (byte[])bytes.Clone() : Encoding.UTF8.GetBytes(_text);
        }

        public string text => _text;

        public byte[] bytes
        {
            get
            {
                byte[] copy = new byte[_bytes.Length];
                Buffer.BlockCopy(_bytes, 0, copy, 0, _bytes.Length);
                return copy;
            }
        }

        public override string ToString() => _text;
    }

    public sealed class ResourceRequest : AsyncOperation
    {
        public ResourceRequest(Object? asset)
        {
            this.asset = asset;
        }

        public Object? asset { get; internal set; }
    }

    public static class Resources
    {
        private static readonly Dictionary<string, List<Object>> RegisteredAssets =
            new Dictionary<string, List<Object>>(StringComparer.OrdinalIgnoreCase);

        private static readonly Dictionary<string, Object> DiskCache =
            new Dictionary<string, Object>(StringComparer.OrdinalIgnoreCase);

        private static readonly List<string> SearchRoots = new List<string>();

        private static readonly List<Object> RootRetainedAssets = new List<Object>();

        private static readonly List<Object> AutoRetainedAssets = new List<Object>();

        private static readonly List<(Object Holder, Object Asset)> OwnerRetainedAssets =
            new List<(Object Holder, Object Asset)>();

        private static readonly List<Action<Object, List<Object>>> ReferenceInspectors =
            new List<Action<Object, List<Object>>>();

        private static bool _referenceTrackingSeen;

        public static int ResidentAssetCount
        {
            get => GetResidentAssets().Length;
        }

        public static int GetResidentAssetCount() => ResidentAssetCount;

        public static bool IsResident(string path)
        {
            if (string.IsNullOrEmpty(path))
            {
                return false;
            }

            string key = NormalizeKey(path);
            if (key.Length == 0)
            {
                return false;
            }

            if (RegisteredAssets.TryGetValue(key, out List<Object>? registered))
            {
                for (int i = 0; i < registered.Count; i++)
                {
                    if (registered[i] != null)
                    {
                        return true;
                    }
                }
            }

            if (DiskCache.TryGetValue(key, out Object? cached) && cached != null)
            {
                return true;
            }

            return false;
        }

        public static bool IsResident(Object? asset)
        {
            if (asset == null)
            {
                return false;
            }

            foreach (KeyValuePair<string, List<Object>> pair in RegisteredAssets)
            {
                for (int i = 0; i < pair.Value.Count; i++)
                {
                    Object candidate = pair.Value[i];
                    if (candidate == null)
                    {
                        continue;
                    }

                    if (ReferenceEquals(candidate, asset))
                    {
                        return true;
                    }

                    if (candidate is GameObject go && asset is Component comp && ReferenceEquals(comp.OwnerOrNull, go))
                    {
                        return true;
                    }
                }
            }

            foreach (KeyValuePair<string, Object> pair in DiskCache)
            {
                Object candidate = pair.Value;
                if (candidate == null)
                {
                    continue;
                }

                if (ReferenceEquals(candidate, asset))
                {
                    return true;
                }

                if (candidate is GameObject go && asset is Component comp && ReferenceEquals(comp.OwnerOrNull, go))
                {
                    return true;
                }
            }

            return false;
        }

        public static Object[] GetResidentAssets()
        {
            List<Object> results = new List<Object>();
            foreach (KeyValuePair<string, List<Object>> pair in RegisteredAssets)
            {
                for (int i = 0; i < pair.Value.Count; i++)
                {
                    Object candidate = pair.Value[i];
                    if (candidate != null && !ContainsReference(results, candidate))
                    {
                        results.Add(candidate);
                    }
                }
            }

            foreach (KeyValuePair<string, Object> pair in DiskCache)
            {
                Object candidate = pair.Value;
                if (candidate != null && !ContainsReference(results, candidate))
                {
                    results.Add(candidate);
                }
            }

            return results.ToArray();
        }

        public static string[] GetResidentAssetPaths()
        {
            List<string> paths = new List<string>();
            foreach (KeyValuePair<string, List<Object>> pair in RegisteredAssets)
            {
                for (int i = 0; i < pair.Value.Count; i++)
                {
                    if (pair.Value[i] != null)
                    {
                        AddUniquePath(paths, pair.Key);
                        break;
                    }
                }
            }

            foreach (KeyValuePair<string, Object> pair in DiskCache)
            {
                if (pair.Value != null)
                {
                    AddUniquePath(paths, pair.Key);
                }
            }

            paths.Sort(StringComparer.OrdinalIgnoreCase);
            return paths.ToArray();
        }

        public static void RetainRoot(Object? asset)
        {
            if (asset == null)
            {
                return;
            }

            _referenceTrackingSeen = true;
            AutoRetainedAssets.Clear();
            if (!ContainsReference(RootRetainedAssets, asset))
            {
                RootRetainedAssets.Add(asset);
            }
        }

        public static void ReleaseRoot(Object? asset)
        {
            if (asset is null)
            {
                return;
            }

            _referenceTrackingSeen = true;
            AutoRetainedAssets.Clear();
            for (int i = RootRetainedAssets.Count - 1; i >= 0; i--)
            {
                if (ReferenceEquals(RootRetainedAssets[i], asset))
                {
                    RootRetainedAssets.RemoveAt(i);
                }
            }
        }

        public static void RetainAssetReference(Object? holder, Object? asset)
        {
            if (holder == null || asset == null)
            {
                return;
            }

            _referenceTrackingSeen = true;
            AutoRetainedAssets.Clear();
            for (int i = 0; i < OwnerRetainedAssets.Count; i++)
            {
                if (ReferenceEquals(OwnerRetainedAssets[i].Holder, holder)
                    && ReferenceEquals(OwnerRetainedAssets[i].Asset, asset))
                {
                    return;
                }
            }

            OwnerRetainedAssets.Add((holder, asset));
        }

        public static void ReleaseAssetReference(Object? holder, Object? asset)
        {
            if (holder is null || asset is null)
            {
                return;
            }

            _referenceTrackingSeen = true;
            AutoRetainedAssets.Clear();
            for (int i = OwnerRetainedAssets.Count - 1; i >= 0; i--)
            {
                if (ReferenceEquals(OwnerRetainedAssets[i].Holder, holder)
                    && ReferenceEquals(OwnerRetainedAssets[i].Asset, asset))
                {
                    OwnerRetainedAssets.RemoveAt(i);
                }
            }
        }

        public static void ClearAssetReferences(Object? holder)
        {
            if (holder is null)
            {
                return;
            }

            _referenceTrackingSeen = true;
            AutoRetainedAssets.Clear();
            for (int i = OwnerRetainedAssets.Count - 1; i >= 0; i--)
            {
                if (ReferenceEquals(OwnerRetainedAssets[i].Holder, holder))
                {
                    OwnerRetainedAssets.RemoveAt(i);
                }
            }
        }

        public static void RegisterReferenceInspector<THolder>(Action<THolder, List<Object>> inspector)
            where THolder : Object
        {
            if (inspector == null)
            {
                throw new ArgumentNullException(nameof(inspector));
            }

            _referenceTrackingSeen = true;
            AutoRetainedAssets.Clear();
            ReferenceInspectors.Add((holder, results) =>
            {
                if (holder is THolder typed)
                {
                    inspector(typed, results);
                }
            });
        }

        public static void Register(string resourcePath, Object asset)
        {
            if (string.IsNullOrEmpty(resourcePath))
            {
                throw new ArgumentException("Resource path cannot be null or empty.", nameof(resourcePath));
            }

            if (asset is null)
            {
                throw new ArgumentNullException(nameof(asset));
            }

            string key = NormalizeKey(resourcePath);
            if (string.IsNullOrEmpty(asset.name))
            {
                asset.name = ExtractLeafName(key);
            }

            if (!RegisteredAssets.TryGetValue(key, out List<Object>? list))
            {
                list = new List<Object>();
                RegisteredAssets[key] = list;
            }

            for (int i = 0; i < list.Count; i++)
            {
                if (ReferenceEquals(list[i], asset))
                {
                    return;
                }
            }

            list.Add(asset);
            AddAutoRetained(asset);
            Object.RegisterLiveObject(asset);
        }

        public static void AddSearchRoot(string rootDirectory)
        {
            if (string.IsNullOrEmpty(rootDirectory))
            {
                return;
            }

            string full = Path.GetFullPath(rootDirectory);
            for (int i = 0; i < SearchRoots.Count; i++)
            {
                if (string.Equals(SearchRoots[i], full, StringComparison.OrdinalIgnoreCase))
                {
                    return;
                }
            }

            SearchRoots.Add(full);
        }

        public static void Clear()
        {
            RegisteredAssets.Clear();
            DiskCache.Clear();
            SearchRoots.Clear();
            RootRetainedAssets.Clear();
            AutoRetainedAssets.Clear();
            OwnerRetainedAssets.Clear();
            ReferenceInspectors.Clear();
            _referenceTrackingSeen = false;
        }

        public static Object? Load(string path) => Load(path, typeof(Object));

        public static Object? Load(string path, Type systemTypeInstance)
        {
            if (systemTypeInstance == null)
            {
                throw new ArgumentNullException(nameof(systemTypeInstance));
            }

            if (string.IsNullOrEmpty(path))
            {
                return null;
            }

            string key = NormalizeKey(path);
            if (key.Length == 0)
            {
                return null;
            }

            if (RegisteredAssets.TryGetValue(key, out List<Object>? registered))
            {
                for (int i = 0; i < registered.Count; i++)
                {
                    Object candidate = registered[i];
                    if (candidate != null && MatchesRequestedType(candidate, systemTypeInstance, out Object? converted))
                    {
                        return converted;
                    }
                }
            }

            if (DiskCache.TryGetValue(key, out Object? cached) && cached != null)
            {
                if (MatchesRequestedType(cached, systemTypeInstance, out Object? converted))
                {
                    return converted;
                }

                return null;
            }

            Object? loaded = TryLoadFromDisk(key);
            if (loaded == null)
            {
                return null;
            }

            DiskCache[key] = loaded;
            AddAutoRetained(loaded);
            Object.RegisterLiveObject(loaded);
            if (MatchesRequestedType(loaded, systemTypeInstance, out Object? match))
            {
                return match;
            }

            return null;
        }

        public static T? Load<T>(string path) where T : Object
        {
            Object? loaded = Load(path, typeof(T));
            return loaded as T;
        }

        public static ResourceRequest LoadAsync(string path) => LoadAsync(path, typeof(Object));

        public static ResourceRequest LoadAsync(string path, Type type)
        {
            if (type == null)
            {
                throw new ArgumentNullException(nameof(type));
            }

            Object? resolved = Load(path, type);
            ResourceRequest request = new ResourceRequest(resolved);
            request.MarkComplete();
            return request;
        }

        public static ResourceRequest LoadAsync<T>(string path) where T : Object =>
            LoadAsync(path, typeof(T));

        public static Object[] LoadAll(string path) => LoadAll(path, typeof(Object));

        public static Object[] LoadAll(string path, Type systemTypeInstance)
        {
            if (systemTypeInstance == null)
            {
                throw new ArgumentNullException(nameof(systemTypeInstance));
            }

            string prefix = NormalizeKey(path ?? string.Empty);
            ScanDiskIntoCache(prefix);

            List<Object> results = new List<Object>();
            foreach (KeyValuePair<string, List<Object>> pair in RegisteredAssets)
            {
                if (!KeyMatchesPrefix(pair.Key, prefix))
                {
                    continue;
                }

                for (int i = 0; i < pair.Value.Count; i++)
                {
                    Object candidate = pair.Value[i];
                    if (candidate != null
                        && MatchesRequestedType(candidate, systemTypeInstance, out Object? converted)
                        && converted != null
                        && !ContainsReference(results, converted))
                    {
                        results.Add(converted);
                    }
                }
            }

            foreach (KeyValuePair<string, Object> pair in DiskCache)
            {
                if (!KeyMatchesPrefix(pair.Key, prefix))
                {
                    continue;
                }

                Object candidate = pair.Value;
                if (candidate != null
                    && MatchesRequestedType(candidate, systemTypeInstance, out Object? converted)
                    && converted != null
                    && !ContainsReference(results, converted))
                {
                    results.Add(converted);
                }
            }

            return results.ToArray();
        }

        public static T[] LoadAll<T>(string path) where T : Object
        {
            Object[] raw = LoadAll(path, typeof(T));
            List<T> typed = new List<T>(raw.Length);
            for (int i = 0; i < raw.Length; i++)
            {
                if (raw[i] is T item)
                {
                    typed.Add(item);
                }
            }

            return typed.ToArray();
        }

        public static void UnloadAsset(Object? assetToUnload)
        {
            if (assetToUnload is null)
            {
                return;
            }

            if (assetToUnload is GameObject || assetToUnload is Component)
            {
                throw new ArgumentException("UnloadAsset may only be used on individual assets and can not be used on GameObject's / Components / AssetBundles", nameof(assetToUnload));
            }

            RemoveAssetFromCaches(assetToUnload);
            Object.Destroy(assetToUnload);
        }

        public static AsyncOperation UnloadUnusedAssets()
        {
            List<Object> referencedAssets = CollectReferencedAssets();

            List<string> emptyRegisteredKeys = new List<string>();
            foreach (KeyValuePair<string, List<Object>> pair in RegisteredAssets)
            {
                for (int i = pair.Value.Count - 1; i >= 0; i--)
                {
                    Object candidate = pair.Value[i];
                    if (candidate == null || !IsAssetRetainedBySet(candidate, referencedAssets))
                    {
                        pair.Value.RemoveAt(i);
                        if (candidate != null && !(candidate is GameObject) && !(candidate is Component))
                        {
                            Object.DeregisterLiveObject(candidate);
                        }
                    }
                }

                if (pair.Value.Count == 0)
                {
                    emptyRegisteredKeys.Add(pair.Key);
                }
            }

            for (int i = 0; i < emptyRegisteredKeys.Count; i++)
            {
                RegisteredAssets.Remove(emptyRegisteredKeys[i]);
            }

            List<string> deadCacheKeys = new List<string>();
            foreach (KeyValuePair<string, Object> pair in DiskCache)
            {
                Object candidate = pair.Value;
                if (candidate == null || !IsAssetRetainedBySet(candidate, referencedAssets))
                {
                    deadCacheKeys.Add(pair.Key);
                    if (candidate != null && !(candidate is GameObject) && !(candidate is Component))
                    {
                        Object.DeregisterLiveObject(candidate);
                    }
                }
            }

            for (int i = 0; i < deadCacheKeys.Count; i++)
            {
                DiskCache.Remove(deadCacheKeys[i]);
            }

            AsyncOperation op = new AsyncOperation();
            op.MarkComplete();
            return op;
        }

        public static T[] FindObjectsOfTypeAll<T>() where T : Object =>
            Object.FindObjectsOfType<T>(true);

        public static Object[] FindObjectsOfTypeAll(Type type) =>
            Object.FindObjectsOfType(type, true);

        private static List<Object> CollectReferencedAssets()
        {
            List<Object> referenced = new List<Object>();
            List<Object> liveSnapshot = Object.SnapshotLiveObjects();

            for (int i = RootRetainedAssets.Count - 1; i >= 0; i--)
            {
                Object rootAsset = RootRetainedAssets[i];
                if (rootAsset == null)
                {
                    RootRetainedAssets.RemoveAt(i);
                    continue;
                }

                AddRetainedTarget(referenced, rootAsset);
            }

            for (int i = OwnerRetainedAssets.Count - 1; i >= 0; i--)
            {
                (Object holder, Object asset) = OwnerRetainedAssets[i];
                if (holder == null || asset == null || !IsLiveSceneHolder(holder, liveSnapshot))
                {
                    OwnerRetainedAssets.RemoveAt(i);
                    continue;
                }

                AddRetainedTarget(referenced, asset);
            }

            bool hasSceneReferences = RootRetainedAssets.Count > 0 || OwnerRetainedAssets.Count > 0;
            List<Object> inspectorBuffer = new List<Object>();
            for (int i = 0; i < liveSnapshot.Count; i++)
            {
                Object liveObj = liveSnapshot[i];
                if (!IsLiveSceneHolder(liveObj, liveSnapshot))
                {
                    continue;
                }

                if (liveObj is AudioSource audioSource && audioSource.clip != null && IsManagedResourceAsset(audioSource.clip))
                {
                    hasSceneReferences = true;
                    RemoveAutoRetained(audioSource.clip);
                    AddRetainedTarget(referenced, audioSource.clip);
                }
                else if (liveObj is MeshFilter meshFilter && meshFilter.sharedMesh != null && IsManagedResourceAsset(meshFilter.sharedMesh))
                {
                    hasSceneReferences = true;
                    RemoveAutoRetained(meshFilter.sharedMesh);
                    AddRetainedTarget(referenced, meshFilter.sharedMesh);
                }
                else if (liveObj is MeshRenderer meshRenderer && meshRenderer.sharedMaterial != null && IsManagedResourceAsset(meshRenderer.sharedMaterial))
                {
                    hasSceneReferences = true;
                    RemoveAutoRetained(meshRenderer.sharedMaterial);
                    AddRetainedTarget(referenced, meshRenderer.sharedMaterial);
                }

                for (int r = 0; r < ReferenceInspectors.Count; r++)
                {
                    inspectorBuffer.Clear();
                    ReferenceInspectors[r](liveObj, inspectorBuffer);
                    for (int b = 0; b < inspectorBuffer.Count; b++)
                    {
                        if (inspectorBuffer[b] != null)
                        {
                            hasSceneReferences = true;
                            RemoveAutoRetained(inspectorBuffer[b]);
                            AddRetainedTarget(referenced, inspectorBuffer[b]);
                        }
                    }
                }
            }

            if (hasSceneReferences)
            {
                _referenceTrackingSeen = true;
            }

            if (_referenceTrackingSeen)
            {
                AutoRetainedAssets.Clear();
            }

            for (int i = AutoRetainedAssets.Count - 1; i >= 0; i--)
            {
                Object autoAsset = AutoRetainedAssets[i];
                if (autoAsset == null)
                {
                    AutoRetainedAssets.RemoveAt(i);
                    continue;
                }

                AddRetainedTarget(referenced, autoAsset);
            }

            return referenced;
        }

        private static bool IsManagedResourceAsset(Object? asset) =>
            IsResident(asset);

        private static bool IsLiveSceneHolder(Object holder, List<Object> liveSnapshot)
        {
            if (holder == null || !ContainsReference(liveSnapshot, holder))
            {
                return false;
            }

            if (holder is Component comp)
            {
                GameObject? owner = comp.OwnerOrNull;
                if (owner != null && (owner == null || !ContainsReference(liveSnapshot, owner)))
                {
                    return false;
                }
            }

            return true;
        }

        private static void AddRetainedTarget(List<Object> list, Object asset)
        {
            if (asset == null)
            {
                return;
            }

            if (!ContainsReference(list, asset))
            {
                list.Add(asset);
            }

            if (asset is Component comp && comp.OwnerOrNull != null && !ContainsReference(list, comp.OwnerOrNull))
            {
                list.Add(comp.OwnerOrNull);
            }
            else if (asset is GameObject go)
            {
                IReadOnlyList<Component> comps = go.Components;
                for (int i = 0; i < comps.Count; i++)
                {
                    Component c = comps[i];
                    if (c != null && !ContainsReference(list, c))
                    {
                        list.Add(c);
                    }
                }
            }
        }

        private static bool IsAssetRetainedBySet(Object asset, List<Object> referencedAssets)
        {
            if (ContainsReference(referencedAssets, asset))
            {
                return true;
            }

            if (asset is GameObject go)
            {
                IReadOnlyList<Component> comps = go.Components;
                for (int i = 0; i < comps.Count; i++)
                {
                    if (comps[i] != null && ContainsReference(referencedAssets, comps[i]))
                    {
                        return true;
                    }
                }
            }
            else if (asset is Component comp && comp.OwnerOrNull != null)
            {
                if (ContainsReference(referencedAssets, comp.OwnerOrNull))
                {
                    return true;
                }
            }

            return false;
        }

        private static void AddUniquePath(List<string> list, string path)
        {
            for (int i = 0; i < list.Count; i++)
            {
                if (string.Equals(list[i], path, StringComparison.OrdinalIgnoreCase))
                {
                    return;
                }
            }

            list.Add(path);
        }

        private static void AddAutoRetained(Object? asset)
        {
            if (asset == null || _referenceTrackingSeen)
            {
                return;
            }

            if (!ContainsReference(AutoRetainedAssets, asset))
            {
                AutoRetainedAssets.Add(asset);
            }
        }

        private static void RemoveAutoRetained(Object? asset)
        {
            if (asset is null)
            {
                return;
            }

            for (int i = AutoRetainedAssets.Count - 1; i >= 0; i--)
            {
                if (ReferenceEquals(AutoRetainedAssets[i], asset))
                {
                    AutoRetainedAssets.RemoveAt(i);
                }
            }
        }

        private static void RemoveAssetFromCaches(Object target)
        {
            for (int i = RootRetainedAssets.Count - 1; i >= 0; i--)
            {
                if (ReferenceEquals(RootRetainedAssets[i], target))
                {
                    RootRetainedAssets.RemoveAt(i);
                }
            }

            RemoveAutoRetained(target);
            for (int i = OwnerRetainedAssets.Count - 1; i >= 0; i--)
            {
                if (ReferenceEquals(OwnerRetainedAssets[i].Asset, target)
                    || ReferenceEquals(OwnerRetainedAssets[i].Holder, target))
                {
                    OwnerRetainedAssets.RemoveAt(i);
                }
            }

            foreach (KeyValuePair<string, List<Object>> pair in RegisteredAssets)
            {
                for (int i = pair.Value.Count - 1; i >= 0; i--)
                {
                    if (ReferenceEquals(pair.Value[i], target))
                    {
                        pair.Value.RemoveAt(i);
                    }
                }
            }

            List<string> removeKeys = new List<string>();
            foreach (KeyValuePair<string, Object> pair in DiskCache)
            {
                if (ReferenceEquals(pair.Value, target))
                {
                    removeKeys.Add(pair.Key);
                }
            }

            for (int i = 0; i < removeKeys.Count; i++)
            {
                DiskCache.Remove(removeKeys[i]);
            }
        }

        private static bool MatchesRequestedType(Object candidate, Type requestedType, out Object? converted)
        {
            if (requestedType.IsInstanceOfType(candidate))
            {
                converted = candidate;
                return true;
            }

            if (candidate is GameObject go && typeof(Component).IsAssignableFrom(requestedType))
            {
                Component? comp = go.GetComponent(requestedType);
                if (comp != null)
                {
                    converted = comp;
                    return true;
                }
            }

            converted = null;
            return false;
        }

        private static Object? TryLoadFromDisk(string normalizedKey)
        {
            List<string> roots = CollectCandidateRoots();
            for (int r = 0; r < roots.Count; r++)
            {
                string root = roots[r];
                if (!Directory.Exists(root))
                {
                    continue;
                }

                List<string> resourcesDirs = new List<string>();
                CollectResourcesDirectories(root, resourcesDirs);
                for (int d = 0; d < resourcesDirs.Count; d++)
                {
                    string? matchedFile = FindMatchingResourceFile(resourcesDirs[d], normalizedKey);
                    if (matchedFile != null)
                    {
                        return LoadAssetFromFile(matchedFile, ExtractLeafName(normalizedKey));
                    }
                }
            }

            return null;
        }

        private static void ScanDiskIntoCache(string prefix)
        {
            List<string> roots = CollectCandidateRoots();
            for (int r = 0; r < roots.Count; r++)
            {
                string root = roots[r];
                if (!Directory.Exists(root))
                {
                    continue;
                }

                List<string> resourcesDirs = new List<string>();
                CollectResourcesDirectories(root, resourcesDirs);
                for (int d = 0; d < resourcesDirs.Count; d++)
                {
                    string resDir = resourcesDirs[d];
                    string[] files = Directory.GetFiles(resDir, "*", SearchOption.AllDirectories);
                    Array.Sort(files, StringComparer.Ordinal);
                    for (int f = 0; f < files.Length; f++)
                    {
                        string file = files[f];
                        if (IsIgnoredResourceFile(file))
                        {
                            continue;
                        }

                        string relKey = RelativeResourceKey(resDir, file);
                        if (relKey.Length == 0 || !KeyMatchesPrefix(relKey, prefix))
                        {
                            continue;
                        }

                        if (!DiskCache.TryGetValue(relKey, out Object? existing) || existing == null)
                        {
                            Object? loaded = LoadAssetFromFile(file, ExtractLeafName(relKey));
                            if (loaded != null)
                            {
                                DiskCache[relKey] = loaded;
                                AddAutoRetained(loaded);
                                Object.RegisterLiveObject(loaded);
                            }
                        }
                    }
                }
            }
        }

        private static List<string> CollectCandidateRoots()
        {
            List<string> roots = new List<string>(SearchRoots);
            string dataPath = Application.dataPath;
            if (!string.IsNullOrEmpty(dataPath) && Directory.Exists(dataPath))
            {
                AddUniqueRoot(roots, dataPath);
            }

            return roots;
        }

        private static void AddUniqueRoot(List<string> roots, string path)
        {
            string full = Path.GetFullPath(path);
            for (int i = 0; i < roots.Count; i++)
            {
                if (string.Equals(roots[i], full, StringComparison.OrdinalIgnoreCase))
                {
                    return;
                }
            }

            roots.Add(full);
        }

        private static void CollectResourcesDirectories(string root, List<string> collected)
        {
            string rootName = Path.GetFileName(root.TrimEnd(Path.DirectorySeparatorChar, Path.AltDirectorySeparatorChar));
            if (string.Equals(rootName, "Resources", StringComparison.OrdinalIgnoreCase))
            {
                collected.Add(root);
            }

            string[] subdirs = Directory.GetDirectories(root, "*", SearchOption.AllDirectories);
            Array.Sort(subdirs, StringComparer.Ordinal);
            for (int i = 0; i < subdirs.Length; i++)
            {
                string dirName = Path.GetFileName(subdirs[i]);
                if (string.Equals(dirName, "Resources", StringComparison.OrdinalIgnoreCase))
                {
                    collected.Add(subdirs[i]);
                }
            }
        }

        private static string? FindMatchingResourceFile(string resourcesDir, string normalizedKey)
        {
            string[] files = Directory.GetFiles(resourcesDir, "*", SearchOption.AllDirectories);
            Array.Sort(files, StringComparer.Ordinal);
            for (int i = 0; i < files.Length; i++)
            {
                if (IsIgnoredResourceFile(files[i]))
                {
                    continue;
                }

                string candidateKey = RelativeResourceKey(resourcesDir, files[i]);
                if (string.Equals(candidateKey, normalizedKey, StringComparison.OrdinalIgnoreCase))
                {
                    return files[i];
                }
            }

            return null;
        }

        private static string RelativeResourceKey(string resourcesDir, string filePath)
        {
            string relative = Path.GetRelativePath(resourcesDir, filePath);
            string extension = Path.GetExtension(relative);
            string withoutExt = extension.Length > 0
                ? relative.Substring(0, relative.Length - extension.Length)
                : relative;
            return NormalizeKey(withoutExt);
        }

        private static bool IsIgnoredResourceFile(string filePath)
        {
            string fileName = Path.GetFileName(filePath);
            if (string.IsNullOrEmpty(fileName) || fileName[0] == '.')
            {
                return true;
            }

            return fileName.EndsWith(".meta", StringComparison.OrdinalIgnoreCase);
        }

        private static Object? LoadAssetFromFile(string filePath, string assetName)
        {
            string extension = Path.GetExtension(filePath).ToLowerInvariant();
            if (extension == ".prefab")
            {
                return LoadPrefabGameObject(filePath, assetName);
            }

            if (extension == ".wav")
            {
                AudioClip? clip = AudioClip.LoadFromFile(filePath);
                if (clip != null)
                {
                    clip.name = assetName;
                    return clip;
                }
                return new AudioClip(assetName, 0f, 1, 44100, 0UL);
            }

            byte[] rawBytes = File.ReadAllBytes(filePath);
            string textContent = Encoding.UTF8.GetString(rawBytes);
            return new TextAsset(textContent, rawBytes)
            {
                name = assetName,
            };
        }

        private static GameObject LoadPrefabGameObject(string filePath, string fallbackName)
        {
            string resolvedName = string.IsNullOrEmpty(fallbackName)
                ? Path.GetFileNameWithoutExtension(filePath)
                : fallbackName;
            return new GameObject(resolvedName);
        }

        private static bool KeyMatchesPrefix(string key, string prefix)
        {
            if (string.IsNullOrEmpty(prefix))
            {
                return true;
            }

            if (string.Equals(key, prefix, StringComparison.OrdinalIgnoreCase))
            {
                return true;
            }

            return key.StartsWith(prefix + "/", StringComparison.OrdinalIgnoreCase);
        }

        private static bool ContainsReference(List<Object> list, Object target)
        {
            for (int i = 0; i < list.Count; i++)
            {
                if (ReferenceEquals(list[i], target))
                {
                    return true;
                }
            }

            return false;
        }

        private static string NormalizeKey(string path)
        {
            if (string.IsNullOrEmpty(path))
            {
                return string.Empty;
            }

            string replaced = path.Trim().Replace('\\', '/');
            string[] parts = replaced.Split('/', StringSplitOptions.RemoveEmptyEntries);
            return string.Join("/", parts);
        }

        private static string ExtractLeafName(string normalizedKey)
        {
            int slash = normalizedKey.LastIndexOf('/');
            return slash >= 0 ? normalizedKey.Substring(slash + 1) : normalizedKey;
        }
    }
}
