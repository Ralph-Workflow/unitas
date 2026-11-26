using System;

namespace UnityEngine
{
    // Marks a parameterless static method the editor may invoke by its fully
    // qualified name; the generated glue registers every marked method in the
    // EditorMethodRegistry when the project scripts are compiled.
    [AttributeUsage(AttributeTargets.Method, AllowMultiple = false, Inherited = false)]
    public sealed class EditorCallableMethodAttribute : Attribute
    {
    }
}
