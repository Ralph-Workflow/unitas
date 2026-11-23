using System;
using System.Collections.Generic;
using System.Globalization;
using UnityEngine;

namespace Unitas.Bridge
{
    public static class ScriptRegistry
    {
        private static readonly Dictionary<(ulong, ulong), Func<MonoBehaviour>> Factories =
            new Dictionary<(ulong, ulong), Func<MonoBehaviour>>();

        public static void Register(ulong guidHi, ulong guidLo, Func<MonoBehaviour> factory)
        {
            if (factory == null)
            {
                throw new ArgumentNullException(nameof(factory));
            }
            Factories[(guidHi, guidLo)] = factory;
        }

        public static void Register(string guid, Func<MonoBehaviour> factory)
        {
            (ulong hi, ulong lo) = ParseGuid(guid);
            Register(hi, lo, factory);
        }

        public static MonoBehaviour? Create(ulong guidHi, ulong guidLo)
        {
            if (Factories.TryGetValue((guidHi, guidLo), out Func<MonoBehaviour>? factory))
            {
                return factory();
            }
            return null;
        }

        public static MonoBehaviour? Create(string guid)
        {
            (ulong hi, ulong lo) = ParseGuid(guid);
            return Create(hi, lo);
        }

        public static void Clear()
        {
            Factories.Clear();
        }

        public static (ulong guidHi, ulong guidLo) ParseGuid(string guid)
        {
            if (guid == null)
            {
                throw new ArgumentNullException(nameof(guid));
            }
            if (guid.Length != 32)
            {
                throw new ArgumentException("GUID must be a 32-character hexadecimal string.", nameof(guid));
            }
            ulong hi = ulong.Parse(guid.Substring(0, 16), NumberStyles.HexNumber, CultureInfo.InvariantCulture);
            ulong lo = ulong.Parse(guid.Substring(16, 16), NumberStyles.HexNumber, CultureInfo.InvariantCulture);
            return (hi, lo);
        }
    }
}
