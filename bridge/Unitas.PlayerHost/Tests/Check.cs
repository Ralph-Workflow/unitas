using System;

namespace Unitas.PlayerHost.Tests
{
    public static class Check
    {
        public static int FailureCount { get; private set; }

        public static void That(string name, bool condition)
        {
            if (condition)
            {
                Console.WriteLine("PASS: " + name);
            }
            else
            {
                FailureCount++;
                Console.WriteLine("FAIL: " + name);
            }
        }

        public static void Equal(string name, int expected, int actual)
        {
            That(name + " (expected: " + expected + ", got: " + actual + ")", expected == actual);
        }
    }
}
