namespace Unitas.Editor.Mcp
{
    public enum Scope
    {
        ReadProject,
        EditContent,
        EditScripts,
        ExecuteProjectCode,
        ImportExternalFiles,
        ChangePackagesAndSettings,
        Build,
        Sign,
        Deploy,
        ExternalNetwork,
        Review,
    }

    public static class Scopes
    {
        public static readonly Scope[] All =
        {
            Scope.ReadProject,
            Scope.EditContent,
            Scope.EditScripts,
            Scope.ExecuteProjectCode,
            Scope.ImportExternalFiles,
            Scope.ChangePackagesAndSettings,
            Scope.Build,
            Scope.Sign,
            Scope.Deploy,
            Scope.ExternalNetwork,
            Scope.Review,
        };

        public static string AsString(Scope scope)
        {
            switch (scope)
            {
                case Scope.ReadProject: return "read-project";
                case Scope.EditContent: return "edit-content";
                case Scope.EditScripts: return "edit-scripts";
                case Scope.ExecuteProjectCode: return "execute-project-code";
                case Scope.ImportExternalFiles: return "import-external-files";
                case Scope.ChangePackagesAndSettings: return "change-packages-and-settings";
                case Scope.Build: return "build";
                case Scope.Sign: return "sign";
                case Scope.Deploy: return "deploy";
                case Scope.ExternalNetwork: return "external-network";
                case Scope.Review: return "review";
                default: return scope.ToString();
            }
        }

        public static bool TryParse(string name, out Scope scope)
        {
            foreach (Scope candidate in All)
            {
                if (AsString(candidate) == name)
                {
                    scope = candidate;
                    return true;
                }
            }

            scope = default;
            return false;
        }
    }
}
