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

        public static int ResidentAssetCount
        {
            get
            {
                int count = 0;
                foreach (KeyValuePair<string, List<Object>> pair in RegisteredAssets)
                {
                    for (int i = 0; i < pair.Value.Count; i++)
                    {
                        if (pair.Value[i] != null)
                        {
                            count++;
                        }
                    }
                }

                foreach (KeyValuePair<string, Object> pair in DiskCache)
                {
                    if (pair.Value != null)
                    {
                        count++;
                    }
                }

                return count;
            }
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
            List<string> emptyRegisteredKeys = new List<string>();
            foreach (KeyValuePair<string, List<Object>> pair in RegisteredAssets)
            {
                for (int i = pair.Value.Count - 1; i >= 0; i--)
                {
                    if (pair.Value[i] == null)
                    {
                        pair.Value.RemoveAt(i);
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
                if (pair.Value == null)
                {
                    deadCacheKeys.Add(pair.Key);
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

        private static void RemoveAssetFromCaches(Object target)
        {
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
