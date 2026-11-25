using Unitas.Editor.Mcp;

namespace Unitas.Editor.Tests.Core
{
    public static class ScopeTests
    {
        public static void Run()
        {
            ScopeSetIsTheFixedAut007Set();
            ScopeNamesRoundTrip();
        }

        private static void ScopeSetIsTheFixedAut007Set()
        {
            string[] names = new string[Scopes.All.Length];
            for (int index = 0; index < Scopes.All.Length; index++)
            {
                names[index] = Scopes.AsString(Scopes.All[index]);
            }

            Check.Equal(
                "scope_set_is_the_fixed_aut_007_set: names",
                "read-project,edit-content,edit-scripts,execute-project-code,import-external-files,"
                    + "change-packages-and-settings,build,sign,deploy,external-network,review",
                string.Join(",", names));
            Check.Equal("scope_set_is_the_fixed_aut_007_set: length", 11, Scopes.All.Length);
        }

        private static void ScopeNamesRoundTrip()
        {
            bool roundTrips = true;
            foreach (Scope scope in Scopes.All)
            {
                if (!Scopes.TryParse(Scopes.AsString(scope), out Scope parsed) || parsed != scope)
                {
                    roundTrips = false;
                }
            }

            Check.That("scope_names_round_trip: every scope round-trips", roundTrips);
            Check.That("scope_names_round_trip: unknown name rejected", !Scopes.TryParse("write-content", out _));
        }
    }
}
