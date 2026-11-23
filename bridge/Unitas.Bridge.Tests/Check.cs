namespace Unitas.Bridge.Tests
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

        public static void Equal(string name, float expected, float actual)
        {
            float diff = expected - actual;
            if (diff < 0.0001f && diff > -0.0001f)
            {
                System.Console.WriteLine("PASS: " + name);
            }
            else
            {
                FailureCount++;
                System.Console.WriteLine("FAIL: " + name + " (expected: " + expected + ", got: " + actual + ")");
            }
        }

        public static void Equal(string name, double expected, double actual)
        {
            double diff = expected - actual;
            if (diff < 0.0001 && diff > -0.0001)
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
