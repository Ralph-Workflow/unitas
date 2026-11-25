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
            public GlueEntry(Func<MonoBehaviour> factory, Func<MonoBehaviour, IUnitasMessageHandler>? handlerFactory)
            {
                Factory = factory;
                HandlerFactory = handlerFactory;
            }

            public Func<MonoBehaviour> Factory { get; }

            public Func<MonoBehaviour, IUnitasMessageHandler>? HandlerFactory { get; }
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

        public static void Register(
            ulong guidHi,
            ulong guidLo,
            Func<MonoBehaviour> factory,
            Func<MonoBehaviour, IUnitasMessageHandler>? handlerFactory = null)
        {
            ArgumentNullException.ThrowIfNull(factory);
            Entries[(guidHi, guidLo)] = new GlueEntry(factory, handlerFactory);
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

            Entries[(guidHi, guidLo)] = new GlueEntry(() => factory(), wrappedHandler);
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

        public static void LoadGlueAssembly(string assemblyPath)
        {
            if (string.IsNullOrEmpty(assemblyPath))
            {
                throw new ArgumentException("Assembly path must be non-empty.", nameof(assemblyPath));
            }

            string fullPath = Path.GetFullPath(assemblyPath);
            if (!File.Exists(fullPath))
            {
                throw new FileNotFoundException("Glue assembly was not found.", fullPath);
            }

            string baseDir = Path.GetDirectoryName(fullPath) ?? string.Empty;
            AssemblyLoadContext context = new AssemblyLoadContext(
                "UnitasGlue_" + Guid.NewGuid().ToString("N", CultureInfo.InvariantCulture),
                isCollectible: true);
            context.Resolving += (alc, assemblyName) =>
            {
                string simpleName = assemblyName.Name ?? string.Empty;
                var loadedInDefault = AssemblyLoadContext.Default.Assemblies;
                foreach (var existing in loadedInDefault)
                {
                    if (string.Equals(existing.GetName().Name, simpleName, StringComparison.OrdinalIgnoreCase))
                    {
                        return existing;
                    }
                }

                if (!string.IsNullOrEmpty(baseDir))
                {
                    string siblingPath = Path.Combine(baseDir, simpleName + ".dll");
                    if (File.Exists(siblingPath))
                    {
                        using FileStream siblingStream = File.OpenRead(siblingPath);
                        return alc.LoadFromStream(siblingStream);
                    }
                }

                return null;
            };

            using FileStream stream = File.OpenRead(fullPath);
            var assembly = context.LoadFromStream(stream);
            RuntimeHelpers.RunModuleConstructor(assembly.ManifestModule.ModuleHandle);
            assembly.CreateInstance("UnitasGenerated.GlueInitializer");
            assembly.CreateInstance("UnitasGeneratedGlue");
        }

        public static void Clear()
        {
            Entries.Clear();
            Handlers.Clear();
            InstanceGuids.Clear();
            TypeResolvers.Clear();
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
