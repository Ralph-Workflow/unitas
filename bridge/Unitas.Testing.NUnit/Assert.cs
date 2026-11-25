using System;
using System.Collections.Generic;
using System.Globalization;

namespace NUnit.Framework
{
    public class AssertionException : Exception
    {
        public AssertionException(string message)
            : base(message)
        {
        }
    }

    public static class Assert
    {
        public static void AreEqual<T>(T expected, T actual)
        {
            AreEqual(expected, actual, null);
        }

        public static void AreEqual<T>(T expected, T actual, string? message)
        {
            if (!EqualityComparer<T>.Default.Equals(expected, actual))
            {
                throw new AssertionException(
                    Describe(message, "Expected:<" + Format(expected) + "> But was:<" + Format(actual) + ">"));
            }
        }

        public static void AreEqual(double expected, double actual, double delta)
        {
            AreEqual(expected, actual, delta, null);
        }

        public static void AreEqual(double expected, double actual, double delta, string? message)
        {
            if (Math.Abs(expected - actual) > Math.Abs(delta))
            {
                throw new AssertionException(
                    Describe(
                        message,
                        "Expected:<"
                        + expected.ToString(CultureInfo.InvariantCulture)
                        + " +/- "
                        + delta.ToString(CultureInfo.InvariantCulture)
                        + "> But was:<"
                        + actual.ToString(CultureInfo.InvariantCulture)
                        + ">"));
            }
        }

        public static void AreEqual(float expected, float actual, float delta)
        {
            AreEqual(expected, actual, delta, null);
        }

        public static void AreEqual(float expected, float actual, float delta, string? message)
        {
            if (MathF.Abs(expected - actual) > MathF.Abs(delta))
            {
                throw new AssertionException(
                    Describe(
                        message,
                        "Expected:<"
                        + expected.ToString(CultureInfo.InvariantCulture)
                        + " +/- "
                        + delta.ToString(CultureInfo.InvariantCulture)
                        + "> But was:<"
                        + actual.ToString(CultureInfo.InvariantCulture)
                        + ">"));
            }
        }

        public static void AreNotEqual<T>(T notExpected, T actual)
        {
            AreNotEqual(notExpected, actual, null);
        }

        public static void AreNotEqual<T>(T notExpected, T actual, string? message)
        {
            if (EqualityComparer<T>.Default.Equals(notExpected, actual))
            {
                throw new AssertionException(
                    Describe(message, "Expected:<not " + Format(notExpected) + "> But was:<" + Format(actual) + ">"));
            }
        }

        public static void IsTrue(bool condition)
        {
            IsTrue(condition, null);
        }

        public static void IsTrue(bool condition, string? message)
        {
            if (!condition)
            {
                throw new AssertionException(Describe(message, "Expected:<True> But was:<False>"));
            }
        }

        public static void IsFalse(bool condition)
        {
            IsFalse(condition, null);
        }

        public static void IsFalse(bool condition, string? message)
        {
            if (condition)
            {
                throw new AssertionException(Describe(message, "Expected:<False> But was:<True>"));
            }
        }

        public static void IsNull<T>(T? actual)
        {
            IsNull(actual, null);
        }

        public static void IsNull<T>(T? actual, string? message)
        {
            if (!ReferenceEquals(actual, null))
            {
                throw new AssertionException(Describe(message, "Expected:<null> But was:<" + Format(actual) + ">"));
            }
        }

        public static void IsNotNull<T>(T? actual)
        {
            IsNotNull(actual, null);
        }

        public static void IsNotNull<T>(T? actual, string? message)
        {
            if (ReferenceEquals(actual, null))
            {
                throw new AssertionException(Describe(message, "Expected:<not null> But was:<null>"));
            }
        }

        public static void That(bool condition)
        {
            IsTrue(condition, null);
        }

        public static void That(bool condition, string message)
        {
            IsTrue(condition, message);
        }

        public static void Greater<T>(T arg1, T arg2)
            where T : IComparable<T>
        {
            if (arg1.CompareTo(arg2) <= 0)
            {
                throw new AssertionException("Expected:<" + Format(arg1) + "> to be greater than:<" + Format(arg2) + ">");
            }
        }

        public static void Less<T>(T arg1, T arg2)
            where T : IComparable<T>
        {
            if (arg1.CompareTo(arg2) >= 0)
            {
                throw new AssertionException("Expected:<" + Format(arg1) + "> to be less than:<" + Format(arg2) + ">");
            }
        }

        public static TException Throws<TException>(Action code)
            where TException : Exception
        {
            try
            {
                code();
            }
            catch (TException expected)
            {
                return expected;
            }
            catch (Exception other)
            {
                throw new AssertionException(
                    "Expected:<" + typeof(TException).Name + "> But was:<" + other.Message + ">");
            }

            throw new AssertionException("Expected:<" + typeof(TException).Name + "> But was:<no exception>");
        }

        public static void Fail(string message)
        {
            throw new AssertionException(message);
        }

        public static void Fail()
        {
            throw new AssertionException("Test failed.");
        }

        public static void Pass()
        {
        }

        public static void Pass(string message)
        {
        }

        private static string Describe(string? message, string detail)
        {
            return string.IsNullOrEmpty(message) ? detail : message + " " + detail;
        }

        private static string Format<T>(T value)
        {
            if (ReferenceEquals(value, null))
            {
                return "null";
            }

            return value.ToString() ?? string.Empty;
        }
    }
}
