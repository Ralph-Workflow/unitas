namespace Unitas.Editor.Tests
{
    public static class Check
    {
        public static int FailureCount { get; private set; }

        public static void That(string name, bool condition)
        {
            if (condition)
            {
                System.Console.WriteLine("PASS: " + name);
            }
            else
            {
                FailureCount++;
                System.Console.WriteLine("FAIL: " + name);
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
                System.Console.WriteLine("FAIL: " + name + " (expected: '" + expected + "', got: '" + actual + "')");
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
                System.Console.WriteLine("FAIL: " + name + " (expected: " + expected + ", got: " + actual + ")");
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
                System.Console.WriteLine("FAIL: " + name + " (expected: " + expected + ", got: " + actual + ")");
            }
        }
    }
}
