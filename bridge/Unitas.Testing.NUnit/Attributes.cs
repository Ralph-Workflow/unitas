using System;
using System.Collections.Generic;

namespace NUnit.Framework
{
    [AttributeUsage(AttributeTargets.Method, AllowMultiple = false, Inherited = true)]
    public sealed class TestAttribute : Attribute
    {
    }

    [AttributeUsage(AttributeTargets.Method, AllowMultiple = true, Inherited = true)]
    public sealed class TestCaseAttribute : Attribute
    {
        private object? _expectedResult;

        public TestCaseAttribute(params object?[] args)
        {
            Arguments = args ?? Array.Empty<object?>();
        }

        public object?[] Arguments { get; }

        public string? TestName { get; set; }

        public string? Category { get; set; }

        public object? ExpectedResult
        {
            get => _expectedResult;
            set
            {
                _expectedResult = value;
                HasExpectedResult = true;
            }
        }

        public bool HasExpectedResult { get; private set; }
    }

    [AttributeUsage(AttributeTargets.Method, AllowMultiple = true, Inherited = true)]
    public sealed class TestCaseSourceAttribute : Attribute
    {
        public TestCaseSourceAttribute(string sourceName)
        {
            SourceName = sourceName ?? throw new ArgumentNullException(nameof(sourceName));
        }

        public TestCaseSourceAttribute(Type sourceType, string sourceName)
        {
            SourceType = sourceType ?? throw new ArgumentNullException(nameof(sourceType));
            SourceName = sourceName ?? throw new ArgumentNullException(nameof(sourceName));
        }

        public TestCaseSourceAttribute(Type sourceType)
        {
            SourceType = sourceType ?? throw new ArgumentNullException(nameof(sourceType));
        }

        public TestCaseSourceAttribute(string sourceName, object?[]? methodParams)
        {
            SourceName = sourceName ?? throw new ArgumentNullException(nameof(sourceName));
            MethodParams = methodParams;
        }

        public TestCaseSourceAttribute(Type sourceType, string sourceName, object?[]? methodParams)
        {
            SourceType = sourceType ?? throw new ArgumentNullException(nameof(sourceType));
            SourceName = sourceName ?? throw new ArgumentNullException(nameof(sourceName));
            MethodParams = methodParams;
        }

        public string? SourceName { get; }

        public Type? SourceType { get; }

        public object?[]? MethodParams { get; }

        public string? Category { get; set; }
    }

    [AttributeUsage(AttributeTargets.Parameter, AllowMultiple = false, Inherited = false)]
    public sealed class ValuesAttribute : Attribute
    {
        public ValuesAttribute()
        {
            Data = Array.Empty<object?>();
            IsAutoValues = true;
        }

        public ValuesAttribute(object? arg1)
        {
            Data = new object?[] { arg1 };
        }

        public ValuesAttribute(object? arg1, object? arg2)
        {
            Data = new object?[] { arg1, arg2 };
        }

        public ValuesAttribute(object? arg1, object? arg2, object? arg3)
        {
            Data = new object?[] { arg1, arg2, arg3 };
        }

        public ValuesAttribute(params object?[]? args)
        {
            Data = args ?? new object?[] { null };
        }

        public object?[] Data { get; }

        public bool IsAutoValues { get; }
    }

    [AttributeUsage(AttributeTargets.Parameter, AllowMultiple = true, Inherited = false)]
    public sealed class ValueSourceAttribute : Attribute
    {
        public ValueSourceAttribute(string sourceName)
        {
            SourceName = sourceName ?? throw new ArgumentNullException(nameof(sourceName));
        }

        public ValueSourceAttribute(Type sourceType, string sourceName)
        {
            SourceType = sourceType ?? throw new ArgumentNullException(nameof(sourceType));
            SourceName = sourceName ?? throw new ArgumentNullException(nameof(sourceName));
        }

        public string SourceName { get; }

        public Type? SourceType { get; }
    }

    public sealed class TestCaseData
    {
        private object? _expectedResult;
        private readonly List<string> _categories = new List<string>();

        public TestCaseData(params object?[]? args)
        {
            Arguments = args ?? new object?[] { null };
        }

        public object?[] Arguments { get; }

        public object? ExpectedResult
        {
            get => _expectedResult;
            set
            {
                _expectedResult = value;
                HasExpectedResult = true;
            }
        }

        public bool HasExpectedResult { get; private set; }

        public string? TestName { get; set; }

        public bool IsIgnored { get; private set; }

        public string? IgnoreReason { get; private set; }

        public bool IsExplicit { get; private set; }

        public string? ExplicitReason { get; private set; }

        public IReadOnlyList<string> Categories => _categories;

        public TestCaseData SetName(string? name)
        {
            TestName = name;
            return this;
        }

        public TestCaseData Returns(object? result)
        {
            ExpectedResult = result;
            return this;
        }

        public TestCaseData SetCategory(string category)
        {
            if (!string.IsNullOrEmpty(category) && !_categories.Contains(category))
            {
                _categories.Add(category);
            }

            return this;
        }

        public TestCaseData Ignore(string reason)
        {
            IsIgnored = true;
            IgnoreReason = reason ?? string.Empty;
            return this;
        }

        public TestCaseData Explicit(string? reason = null)
        {
            IsExplicit = true;
            ExplicitReason = reason ?? string.Empty;
            return this;
        }
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
