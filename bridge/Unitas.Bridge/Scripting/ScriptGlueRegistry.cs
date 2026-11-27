using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Runtime.CompilerServices;
using System.Runtime.Loader;
using UnityEngine;

namespace Unitas.Bridge.Scripting
{
    public static class ScriptGlueRegistry
    {
        private readonly struct GlueEntry
        {
            public GlueEntry(Func<MonoBehaviour> factory, Func<MonoBehaviour, IUnitasMessageHandler>? handlerFactory, Type? scriptType)
            {
                Factory = factory;
                HandlerFactory = handlerFactory;
                ScriptType = scriptType;
            }

            public Func<MonoBehaviour> Factory { get; }

            public Func<MonoBehaviour, IUnitasMessageHandler>? HandlerFactory { get; }

            public Type? ScriptType { get; }
        }

        private static readonly Dictionary<(ulong, ulong), GlueEntry> Entries =
            new Dictionary<(ulong, ulong), GlueEntry>();

        private static readonly ConditionalWeakTable<MonoBehaviour, IUnitasMessageHandler> Handlers =
            new ConditionalWeakTable<MonoBehaviour, IUnitasMessageHandler>();

        private sealed class GuidPairBox
        {
            public GuidPairBox(ulong hi, ulong lo)
            {
                Hi = hi;
                Lo = lo;
            }

            public ulong Hi { get; }

            public ulong Lo { get; }
        }

        private static readonly ConditionalWeakTable<MonoBehaviour, GuidPairBox> InstanceGuids =
            new ConditionalWeakTable<MonoBehaviour, GuidPairBox>();

        private static readonly List<Func<MonoBehaviour, IUnitasMessageHandler?>> TypeResolvers =
            new List<Func<MonoBehaviour, IUnitasMessageHandler?>>();

        private static readonly Dictionary<Type, (ulong Hi, ulong Lo)> TypeGuids =
            new Dictionary<Type, (ulong Hi, ulong Lo)>();

        private const ulong DynamicTypeGuidHi = 0xD000000000000000UL;
        private static ulong _nextDynamicTypeGuidLo = 1;

        public static void Register(
            ulong guidHi,
            ulong guidLo,
            Func<MonoBehaviour> factory,
            Func<MonoBehaviour, IUnitasMessageHandler>? handlerFactory = null)
        {
            ArgumentNullException.ThrowIfNull(factory);
            Entries[(guidHi, guidLo)] = new GlueEntry(factory, handlerFactory, null);
        }

        public static void Register(
            string guidHex,
            Func<MonoBehaviour> factory,
            Func<MonoBehaviour, IUnitasMessageHandler>? handlerFactory = null)
        {
            (ulong hi, ulong lo) = ParseGuid(guidHex);
            Register(hi, lo, factory, handlerFactory);
        }

        public static void Register<T>(
            ulong guidHi,
            ulong guidLo,
            Func<T> factory,
            Func<T, IUnitasMessageHandler>? handlerFactory = null)
            where T : MonoBehaviour
        {
            ArgumentNullException.ThrowIfNull(factory);

            Func<MonoBehaviour, IUnitasMessageHandler>? wrappedHandler = null;
            if (handlerFactory != null)
            {
                wrappedHandler = instance => handlerFactory((T)instance);
                TypeResolvers.Add(instance => instance is T typed ? handlerFactory(typed) : null);
            }

            Entries[(guidHi, guidLo)] = new GlueEntry(() => factory(), wrappedHandler, typeof(T));
            TypeGuids[typeof(T)] = (guidHi, guidLo);
        }

        public static void RegisterTypeFactory(Type scriptType, Func<MonoBehaviour> factory)
        {
            ArgumentNullException.ThrowIfNull(scriptType);
            ArgumentNullException.ThrowIfNull(factory);
            if (TypeGuids.TryGetValue(scriptType, out (ulong Hi, ulong Lo) existing)
                && Entries.ContainsKey(existing))
            {
                return;
            }

            foreach (KeyValuePair<(ulong, ulong), GlueEntry> entry in Entries)
            {
                if (entry.Value.ScriptType == scriptType)
                {
                    TypeGuids[scriptType] = entry.Key;
                    return;
                }
            }

            (ulong hi, ulong lo) = (DynamicTypeGuidHi, _nextDynamicTypeGuidLo++);
            Entries[(hi, lo)] = new GlueEntry(factory, null, scriptType);
            TypeGuids[scriptType] = (hi, lo);
        }

        public static bool TryGetOrRegisterGuidForType(Type scriptType, out ulong guidHi, out ulong guidLo)
        {
            guidHi = 0;
            guidLo = 0;
            if (scriptType == null || !typeof(MonoBehaviour).IsAssignableFrom(scriptType) || scriptType.IsAbstract)
            {
                return false;
            }

            if (TypeGuids.TryGetValue(scriptType, out (ulong Hi, ulong Lo) found) && Entries.ContainsKey(found))
            {
                guidHi = found.Hi;
                guidLo = found.Lo;
                return true;
            }

            foreach (KeyValuePair<(ulong, ulong), GlueEntry> entry in Entries)
            {
                if (entry.Value.ScriptType == scriptType)
                {
                    TypeGuids[scriptType] = entry.Key;
                    guidHi = entry.Key.Item1;
                    guidLo = entry.Key.Item2;
                    return true;
                }
            }

            (ulong hi, ulong lo) = (DynamicTypeGuidHi, _nextDynamicTypeGuidLo++);
            Entries[(hi, lo)] = new GlueEntry(
                () => (MonoBehaviour)Activator.CreateInstance(scriptType)!,
                null,
                scriptType);
            TypeGuids[scriptType] = (hi, lo);
            guidHi = hi;
            guidLo = lo;
            return true;
        }

        public static void Register<T>(
            string guidHex,
            Func<T> factory,
            Func<T, IUnitasMessageHandler>? handlerFactory = null)
            where T : MonoBehaviour
        {
            (ulong hi, ulong lo) = ParseGuid(guidHex);
            Register(hi, lo, factory, handlerFactory);
        }

        public enum BuiltInUiComponent
        {
            None = 0,
            Button,
            Text,
            Image,
            CanvasScaler,
        }

        private static class BuiltInUiGuids
        {
            internal const ulong ButtonHi = 0x4e29b1a8efbd4b44UL;
            internal const ulong ButtonLo = 0xbb3f3716e73f07ffUL;
            internal const ulong TextHi = 0x5f7201a12d95ffc4UL;
            internal const ulong TextLo = 0x09449d95f23cf332UL;
            internal const ulong ImageHi = 0xfe87c0e1cc204ed4UL;
            internal const ulong ImageLo = 0x8ad3b37840f39efcUL;
            internal const ulong CanvasScalerHi = 0x0cd44c1031e13a94UL;
            internal const ulong CanvasScalerLo = 0x3bb63640046fad76UL;
        }

        public static void RegisterBuiltInUiComponents()
        {
            Register<UnityEngine.UI.Button>(
                BuiltInUiGuids.ButtonHi,
                BuiltInUiGuids.ButtonLo,
                () => new UnityEngine.UI.Button());
            Register<UnityEngine.UI.Text>(
                BuiltInUiGuids.TextHi,
                BuiltInUiGuids.TextLo,
                () => new UnityEngine.UI.Text());
            Register<UnityEngine.UI.Image>(
                BuiltInUiGuids.ImageHi,
                BuiltInUiGuids.ImageLo,
                () => new UnityEngine.UI.Image());
            Register<UnityEngine.UI.CanvasScaler>(
                BuiltInUiGuids.CanvasScalerHi,
                BuiltInUiGuids.CanvasScalerLo,
                () => new UnityEngine.UI.CanvasScaler());
        }

        public static BuiltInUiComponent GetBuiltInUiComponent(ulong guidHi, ulong guidLo)
        {
            if (guidHi == BuiltInUiGuids.ButtonHi && guidLo == BuiltInUiGuids.ButtonLo)
            {
                return BuiltInUiComponent.Button;
            }

            if (guidHi == BuiltInUiGuids.TextHi && guidLo == BuiltInUiGuids.TextLo)
            {
                return BuiltInUiComponent.Text;
            }

            if (guidHi == BuiltInUiGuids.ImageHi && guidLo == BuiltInUiGuids.ImageLo)
            {
                return BuiltInUiComponent.Image;
            }

            if (guidHi == BuiltInUiGuids.CanvasScalerHi && guidLo == BuiltInUiGuids.CanvasScalerLo)
            {
                return BuiltInUiComponent.CanvasScaler;
            }

            return BuiltInUiComponent.None;
        }

        public static void RegisterHandler(MonoBehaviour instance, IUnitasMessageHandler handler)
        {
            ArgumentNullException.ThrowIfNull(instance);
            ArgumentNullException.ThrowIfNull(handler);

            instance.MessageHandler = handler;
            Handlers.AddOrUpdate(instance, handler);
        }

        public static void RegisterHandlerFactory<T>(Func<T, IUnitasMessageHandler> handlerFactory)
            where T : MonoBehaviour
        {
            ArgumentNullException.ThrowIfNull(handlerFactory);

            TypeResolvers.Add(instance => instance is T typed ? handlerFactory(typed) : null);
        }

        public static bool TryCreate(
            ulong guidHi,
            ulong guidLo,
            out MonoBehaviour? instance,
            out IUnitasMessageHandler? handler)
        {
            instance = null;
            handler = null;
            if (!Entries.TryGetValue((guidHi, guidLo), out GlueEntry entry))
            {
                return false;
            }

            MonoBehaviour created = entry.Factory();
            if (created is null)
            {
                return false;
            }

            if (entry.ScriptType != null && created.AttachedRuntimeType == null)
            {
                created.AttachedRuntimeType = entry.ScriptType;
            }

            instance = created;
            InstanceGuids.AddOrUpdate(created, new GuidPairBox(guidHi, guidLo));
            if (entry.HandlerFactory != null)
            {
                handler = entry.HandlerFactory(created);
                if (handler != null)
                {
                    created.MessageHandler = handler;
                    Handlers.AddOrUpdate(created, handler);
                }
            }
            else if (TryGetHandler(created, out IUnitasMessageHandler? resolved))
            {
                handler = resolved;
            }

            return true;
        }

        public static bool TryCreate(
            string guidHex,
            out MonoBehaviour? instance,
            out IUnitasMessageHandler? handler)
        {
            (ulong hi, ulong lo) = ParseGuid(guidHex);
            return TryCreate(hi, lo, out instance, out handler);
        }

        public static MonoBehaviour? Create(ulong guidHi, ulong guidLo)
        {
            TryCreate(guidHi, guidLo, out MonoBehaviour? instance, out IUnitasMessageHandler? _);
            return instance;
        }

        public static MonoBehaviour? Create(string guidHex)
        {
            (ulong hi, ulong lo) = ParseGuid(guidHex);
            return Create(hi, lo);
        }

        public static bool TryGetRegisteredGuid(MonoBehaviour? instance, out ulong guidHi, out ulong guidLo)
        {
            guidHi = 0;
            guidLo = 0;
            if (instance != null && InstanceGuids.TryGetValue(instance, out GuidPairBox? box) && box != null)
            {
                guidHi = box.Hi;
                guidLo = box.Lo;
                return true;
            }

            return false;
        }

        public static bool TryGetHandler(MonoBehaviour? instance, out IUnitasMessageHandler? handler)
        {
            handler = null;
            if (instance is null)
            {
                return false;
            }

            if (instance.MessageHandler != null)
            {
                handler = instance.MessageHandler;
                return true;
            }

            if (Handlers.TryGetValue(instance, out IUnitasMessageHandler? cached) && cached != null)
            {
                instance.MessageHandler = cached;
                handler = cached;
                return true;
            }

            for (int index = TypeResolvers.Count - 1; index >= 0; index--)
            {
                IUnitasMessageHandler? candidate = TypeResolvers[index](instance);
                if (candidate != null)
                {
                    instance.MessageHandler = candidate;
                    Handlers.AddOrUpdate(instance, candidate);
                    handler = candidate;
                    return true;
                }
            }

            return false;
        }

        public static bool IsRegistered(string guidHex)
        {
            (ulong hi, ulong lo) = ParseGuid(guidHex);
            return Entries.ContainsKey((hi, lo));
        }

        public static void LoadGlueAssembly(string assemblyPath)
        {
            AssemblyLoadContext context = CreateCollectibleContext("UnitasGlue", Path.GetDirectoryName(Path.GetFullPath(assemblyPath ?? string.Empty)) ?? string.Empty);
            LoadGlueAssembly(assemblyPath ?? string.Empty, context);
        }

        public static AssemblyLoadContext CreateCollectibleContext(string namePrefix, string managedDir)
        {
            string prefix = string.IsNullOrEmpty(namePrefix) ? "UnitasGlue" : namePrefix;
            AssemblyLoadContext context = new AssemblyLoadContext(
                prefix + "_" + Guid.NewGuid().ToString("N", CultureInfo.InvariantCulture),
                isCollectible: true);
            context.Resolving += (alc, assemblyName) =>
            {
                string simpleName = assemblyName.Name ?? string.Empty;
                foreach (var existing in AssemblyLoadContext.Default.Assemblies)
                {
                    if (string.Equals(existing.GetName().Name, simpleName, StringComparison.OrdinalIgnoreCase))
                    {
                        return existing;
                    }
                }

                foreach (var existing in alc.Assemblies)
                {
                    if (string.Equals(existing.GetName().Name, simpleName, StringComparison.OrdinalIgnoreCase))
                    {
                        return existing;
                    }
                }

                if (!string.IsNullOrEmpty(managedDir))
                {
                    string siblingPath = Path.Combine(managedDir, simpleName + ".dll");
                    if (File.Exists(siblingPath))
                    {
                        using FileStream siblingStream = File.OpenRead(siblingPath);
                        return alc.LoadFromStream(siblingStream);
                    }
                }

                return null;
            };
            return context;
        }

        public static void LoadGlueAssembly(string assemblyPath, AssemblyLoadContext context)
        {
            ArgumentNullException.ThrowIfNull(context);
            if (string.IsNullOrEmpty(assemblyPath))
            {
                throw new ArgumentException("Assembly path must be non-empty.", nameof(assemblyPath));
            }

            string fullPath = Path.GetFullPath(assemblyPath);
            if (!File.Exists(fullPath))
            {
                throw new FileNotFoundException("Glue assembly was not found.", fullPath);
            }

            string simpleName = Path.GetFileNameWithoutExtension(fullPath);
            foreach (var existing in AssemblyLoadContext.Default.Assemblies)
            {
                if (string.Equals(existing.GetName().Name, simpleName, StringComparison.OrdinalIgnoreCase))
                {
                    return;
                }
            }

            foreach (var existing in context.Assemblies)
            {
                if (string.Equals(existing.GetName().Name, simpleName, StringComparison.OrdinalIgnoreCase))
                {
                    RuntimeHelpers.RunModuleConstructor(existing.ManifestModule.ModuleHandle);
                    existing.CreateInstance("UnitasGenerated.GlueInitializer");
                    existing.CreateInstance("UnitasGeneratedGlue");
                    return;
                }
            }

            using FileStream stream = File.OpenRead(fullPath);
            var assembly = context.LoadFromStream(stream);
            RuntimeHelpers.RunModuleConstructor(assembly.ManifestModule.ModuleHandle);
            assembly.CreateInstance("UnitasGenerated.GlueInitializer");
            assembly.CreateInstance("UnitasGeneratedGlue");
        }

        public static void ClearSession()
        {
            List<UnityEngine.Object> liveObjects = UnityEngine.Object.SnapshotLiveObjects();
            for (int i = 0; i < liveObjects.Count; i++)
            {
                UnityEngine.Object candidate = liveObjects[i];
                if (candidate is UnityEngine.UI.Button button)
                {
                    button.onClick.RemoveAllListeners();
                }

                if (candidate is MonoBehaviour behaviour)
                {
                    CoroutineScheduler.OwnerDestroyed(behaviour);
                    InvokeScheduler.OwnerDestroyed(behaviour);
                    Unitas.Bridge.Messaging.MessageDispatcher.UnregisterHandler(behaviour);
                }

                if (candidate is GameObject gameObject)
                {
                    IReadOnlyList<Component> components = gameObject.Components;
                    for (int c = 0; c < components.Count; c++)
                    {
                        Component component = components[c];
                        if (component is UnityEngine.UI.Button childButton)
                        {
                            childButton.onClick.RemoveAllListeners();
                        }

                        if (component is MonoBehaviour childBehaviour)
                        {
                            CoroutineScheduler.OwnerDestroyed(childBehaviour);
                            InvokeScheduler.OwnerDestroyed(childBehaviour);
                            Unitas.Bridge.Messaging.MessageDispatcher.UnregisterHandler(childBehaviour);
                        }

                        UnityEngine.Object.Destroy(component);
                        UnityEngine.Object.DeregisterLiveObject(component);
                    }

                    UnityEngine.Object.DestroySceneObject(gameObject);
                }

                UnityEngine.Object.Destroy(candidate);
                UnityEngine.Object.DeregisterLiveObject(candidate);
            }

            UnityEngine.Object.FlushPendingDestroys(new List<UnityEngine.Object>());
            Transform probeTransform = new Transform();
            _ = probeTransform.GetSiblingIndex();
            UnityEngine.Object.DestroyImmediate(probeTransform);
            UnityEngine.UI.UiButtons.Find(ulong.MaxValue);
            CoroutineScheduler.StopAll();
            InvokeScheduler.StopAll();
            UnitySynchronizationContext.Reset();
            UnityEngine.InputSystem.InputSystem.Reset();
            Application.ClearEvents();
            UnityEngine.SceneManagement.SceneManager.ClearBuildScenes();
            Resources.Clear();
            LogRecorder.Clear();
            LogRecorder.CurrentOrigin = null;
            RuntimeInitializeOnLoadMethodAttribute.ClearRegisteredMethods();
            Component.ClearAttributeRegistrations();
            MonoBehaviour.ClearExecutionOrderOverrides();
            MonoBehaviour.ClearRegisteredDefaultExecutionOrders();
            ExecutionOrderSettings.Clear();
            EditorMethodRegistry.Clear();
            Unitas.Bridge.Messaging.MessageDispatcher.Clear();
            Clear();
        }

        static ScriptGlueRegistry()
        {
            RegisterBuiltInUiComponents();
        }

        public static void Clear()
        {
            Entries.Clear();
            Handlers.Clear();
            InstanceGuids.Clear();
            TypeResolvers.Clear();
            TypeGuids.Clear();
            _nextDynamicTypeGuidLo = 1;
            RegisterBuiltInUiComponents();
        }

        public static (ulong guidHi, ulong guidLo) ParseGuid(string guidHex)
        {
            ArgumentNullException.ThrowIfNull(guidHex);

            string trimmed = guidHex.Trim();
            if (trimmed.Length != 32)
            {
                throw new ArgumentException("GUID must be a 32-character hexadecimal string.", nameof(guidHex));
            }

            ulong hi = ulong.Parse(trimmed.AsSpan(0, 16), NumberStyles.HexNumber, CultureInfo.InvariantCulture);
            ulong lo = ulong.Parse(trimmed.AsSpan(16, 16), NumberStyles.HexNumber, CultureInfo.InvariantCulture);
            return (hi, lo);
        }
    }
}
