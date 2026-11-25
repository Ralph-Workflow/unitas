using System;

namespace NUnit.Framework
{
    [AttributeUsage(AttributeTargets.Method, AllowMultiple = false, Inherited = true)]
    public sealed class TestAttribute : Attribute
    {
    }

    [AttributeUsage(AttributeTargets.Method, AllowMultiple = true, Inherited = true)]
    public sealed class TestCaseAttribute : Attribute
    {
        public TestCaseAttribute(params object?[] args)
        {
            Arguments = args ?? Array.Empty<object?>();
        }

        public object?[] Arguments { get; }

        public string? TestName { get; set; }

        public string? Category { get; set; }
    }

    [AttributeUsage(AttributeTargets.Class | AttributeTargets.Method, AllowMultiple = true, Inherited = true)]
    public sealed class CategoryAttribute : Attribute
    {
        public CategoryAttribute(string name)
        {
            Name = name ?? throw new ArgumentNullException(nameof(name));
        }

        public string Name { get; }
    }

    [AttributeUsage(AttributeTargets.Method, AllowMultiple = false, Inherited = true)]
    public sealed class SetUpAttribute : Attribute
    {
    }

    [AttributeUsage(AttributeTargets.Method, AllowMultiple = false, Inherited = true)]
    public sealed class TearDownAttribute : Attribute
    {
    }

    [AttributeUsage(AttributeTargets.Method, AllowMultiple = false, Inherited = true)]
    public sealed class OneTimeSetUpAttribute : Attribute
    {
    }

    [AttributeUsage(AttributeTargets.Method, AllowMultiple = false, Inherited = true)]
    public sealed class OneTimeTearDownAttribute : Attribute
    {
    }

    [AttributeUsage(AttributeTargets.Class, AllowMultiple = false, Inherited = true)]
    public sealed class TestFixtureAttribute : Attribute
    {
    }

    [AttributeUsage(AttributeTargets.Class | AttributeTargets.Method, AllowMultiple = false, Inherited = true)]
    public sealed class IgnoreAttribute : Attribute
    {
        public IgnoreAttribute(string reason)
        {
            Reason = reason ?? string.Empty;
        }

        public string Reason { get; }
    }

    [AttributeUsage(AttributeTargets.Class | AttributeTargets.Method, AllowMultiple = false, Inherited = true)]
    public sealed class ExplicitAttribute : Attribute
    {
        public ExplicitAttribute()
        {
            Reason = string.Empty;
        }

        public ExplicitAttribute(string reason)
        {
            Reason = reason ?? string.Empty;
        }

        public string Reason { get; }
    }

    [AttributeUsage(AttributeTargets.Method, AllowMultiple = false, Inherited = false)]
    public sealed class RetryAttribute : Attribute
    {
        public RetryAttribute(int tryCount)
        {
            TryCount = tryCount;
        }

        public int TryCount { get; }
    }

    [AttributeUsage(AttributeTargets.Method, AllowMultiple = false, Inherited = false)]
    public sealed class RepeatAttribute : Attribute
    {
        public RepeatAttribute(int count)
        {
            Count = count;
        }

        public int Count { get; }
    }

    [AttributeUsage(AttributeTargets.Class | AttributeTargets.Method, AllowMultiple = false, Inherited = false)]
    public sealed class TimeoutAttribute : Attribute
    {
        public TimeoutAttribute(int timeout)
        {
            Timeout = timeout;
        }

        public int Timeout { get; }
    }
}

namespace UnityEngine.TestTools
{
    [AttributeUsage(AttributeTargets.Method, AllowMultiple = false, Inherited = true)]
    public sealed class UnityTestAttribute : Attribute
    {
    }

    [AttributeUsage(AttributeTargets.Method, AllowMultiple = false, Inherited = true)]
    public sealed class UnitySetUpAttribute : Attribute
    {
    }

    [AttributeUsage(AttributeTargets.Method, AllowMultiple = false, Inherited = true)]
    public sealed class UnityTearDownAttribute : Attribute
    {
    }

    [AttributeUsage(AttributeTargets.Assembly | AttributeTargets.Class | AttributeTargets.Method, AllowMultiple = true, Inherited = false)]
    public sealed class UnityPlatformAttribute : Attribute
    {
        public UnityPlatformAttribute()
        {
            include = Array.Empty<UnityEngine.RuntimePlatform>();
            exclude = Array.Empty<UnityEngine.RuntimePlatform>();
        }

        public UnityPlatformAttribute(params UnityEngine.RuntimePlatform[] include)
        {
            this.include = include ?? Array.Empty<UnityEngine.RuntimePlatform>();
            exclude = Array.Empty<UnityEngine.RuntimePlatform>();
        }

        public UnityEngine.RuntimePlatform[] include { get; set; }

        public UnityEngine.RuntimePlatform[] exclude { get; set; }
    }

    public interface IPrebuildSetup
    {
        void Setup();
    }

    public interface IPostBuildCleanup
    {
        void Cleanup();
    }

    [AttributeUsage(AttributeTargets.Assembly | AttributeTargets.Class | AttributeTargets.Method, AllowMultiple = false, Inherited = false)]
    public sealed class PrebuildSetupAttribute : Attribute
    {
        public PrebuildSetupAttribute(Type targetClass)
        {
            TargetClass = targetClass;
            TargetClassName = targetClass?.FullName ?? string.Empty;
        }

        public PrebuildSetupAttribute(string targetClassName)
        {
            TargetClass = null;
            TargetClassName = targetClassName ?? string.Empty;
        }

        public Type? TargetClass { get; }

        public string TargetClassName { get; }
    }

    [AttributeUsage(AttributeTargets.Assembly | AttributeTargets.Class | AttributeTargets.Method, AllowMultiple = false, Inherited = false)]
    public sealed class PostBuildCleanupAttribute : Attribute
    {
        public PostBuildCleanupAttribute(Type targetClass)
        {
            TargetClass = targetClass;
            TargetClassName = targetClass?.FullName ?? string.Empty;
        }

        public PostBuildCleanupAttribute(string targetClassName)
        {
            TargetClass = null;
            TargetClassName = targetClassName ?? string.Empty;
        }

        public Type? TargetClass { get; }

        public string TargetClassName { get; }
    }
}
