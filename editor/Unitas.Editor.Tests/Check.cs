namespace Unitas.Editor.Tests
{
    public static class Check
    {
        public static int FailureCount { get; private set; }

        // The runner names the suite that is running so a failure names its suite.
        public static string Suite { get; set; } = string.Empty;

        public static void That(string name, bool condition)
        {
            if (condition)
            {
                System.Console.WriteLine("PASS: " + name);
            }
            else
            {
                FailureCount++;
                System.Console.WriteLine("FAIL: " + Label() + name);
            }
        }

        public static void Equal(string name, string expected, string actual)
        {
            if (expected == actual)
            {
                System.Console.WriteLine("PASS: " + name);
            }
            else
            {
                FailureCount++;
                System.Console.WriteLine("FAIL: " + Label() + name + " (expected: '" + expected + "', got: '" + actual + "')");
            }
        }

        public static void Equal(string name, int expected, int actual)
        {
            if (expected == actual)
            {
                System.Console.WriteLine("PASS: " + name);
            }
            else
            {
                FailureCount++;
                System.Console.WriteLine("FAIL: " + Label() + name + " (expected: " + expected + ", got: " + actual + ")");
            }
        }

        public static void Equal(string name, bool expected, bool actual)
        {
            if (expected == actual)
            {
                System.Console.WriteLine("PASS: " + name);
            }
            else
            {
                FailureCount++;
                System.Console.WriteLine("FAIL: " + Label() + name + " (expected: " + expected + ", got: " + actual + ")");
            }
        }

        private static string Label()
        {
            return Suite.Length == 0 ? string.Empty : "[" + Suite + "] ";
        }
    }
}
