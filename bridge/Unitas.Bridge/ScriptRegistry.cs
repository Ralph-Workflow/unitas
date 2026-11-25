using System;
using Unitas.Bridge.Scripting;
using UnityEngine;

namespace Unitas.Bridge
{
    public static class ScriptRegistry
    {
        public static MonoBehaviour? Create(ulong guidHi, ulong guidLo)
        {
            return ScriptGlueRegistry.Create(guidHi, guidLo);
        }

        public static MonoBehaviour? Create(string guidHex)
        {
            return ScriptGlueRegistry.Create(guidHex);
        }

        public static void Clear()
        {
            ScriptGlueRegistry.Clear();
        }

        public static (ulong guidHi, ulong guidLo) ParseGuid(string guidHex)
        {
            return ScriptGlueRegistry.ParseGuid(guidHex);
        }
    }
}
