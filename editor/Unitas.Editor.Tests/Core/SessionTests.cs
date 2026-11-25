using System;
using System.Linq;
using Unitas.Editor.Mcp;

namespace Unitas.Editor.Tests.Core
{
    public static class SessionTests
    {
        public static void Run()
        {
            NewSessionHoldsOnlyReadProject();
            PersonGrantAddsTheScope();
            SessionInheritedGrantIsRefused();
            StandingGrantFromASessionIsRefused();
            StandingGrantAuthorizesAMatchingAgent();
            MissingScopeRefusalNamesTheScope();
        }

        private static SessionIdentity Identity()
        {
            return new SessionIdentity("agent-1", "stdio", "person-1");
        }

        private static void NewSessionHoldsOnlyReadProject()
        {
            Authorizer authorizer = new Authorizer();
            string id = authorizer.CreateSession(Identity());
            Session session = authorizer.GetSession(id);
            Scope[] scopes = session.Scopes.ToArray();
            Check.That(
                "new_session_holds_only_read_project",
                scopes.Length == 1 && scopes[0] == Scope.ReadProject);
        }

        private static void PersonGrantAddsTheScope()
        {
            Authorizer authorizer = new Authorizer();
            string id = authorizer.CreateSession(Identity());
            authorizer.GrantScope(id, Scope.EditContent, GrantAuthority.Person);
            bool authorized;
            try
            {
                authorizer.Authorize(id, Scope.EditContent);
                authorized = true;
            }
            catch (McpError)
            {
                authorized = false;
            }

            Check.That("person_grant_adds_the_scope", authorized);
        }

        private static void SessionInheritedGrantIsRefused()
        {
            Authorizer authorizer = new Authorizer();
            string id = authorizer.CreateSession(Identity());
            McpError.SelfWidening? error = null;
            try
            {
                authorizer.GrantScope(id, Scope.Deploy, GrantAuthority.SessionInherited);
            }
            catch (McpError.SelfWidening failure)
            {
                error = failure;
            }

            Check.That(
                "session_inherited_grant_is_refused: refusal names the scope",
                error != null && error.Message.Contains("deploy", StringComparison.Ordinal));
            Check.That(
                "session_inherited_grant_is_refused: a refused grant must not widen the session",
                !authorizer.GetSession(id).Holds(Scope.Deploy));
        }

        private static void StandingGrantFromASessionIsRefused()
        {
            Authorizer authorizer = new Authorizer();
            bool refused;
            try
            {
                authorizer.GrantStanding("agent-1", Scope.Sign, GrantAuthority.SessionInherited);
                refused = false;
            }
            catch (McpError.SelfWidening)
            {
                refused = true;
            }

            Check.That("standing_grant_from_a_session_is_refused", refused);
        }

        private static void StandingGrantAuthorizesAMatchingAgent()
        {
            Authorizer authorizer = new Authorizer();
            string id = authorizer.CreateSession(Identity());
            authorizer.GrantStanding("agent-1", Scope.Review, GrantAuthority.Person);
            bool authorized;
            try
            {
                authorizer.Authorize(id, Scope.Review);
                authorized = true;
            }
            catch (McpError)
            {
                authorized = false;
            }

            Check.That("standing_grant_authorizes_a_matching_agent", authorized);
        }

        private static void MissingScopeRefusalNamesTheScope()
        {
            Authorizer authorizer = new Authorizer();
            string id = authorizer.CreateSession(Identity());
            McpError.MissingScope? error = null;
            try
            {
                authorizer.Authorize(id, Scope.ExternalNetwork);
            }
            catch (McpError.MissingScope failure)
            {
                error = failure;
            }

            Check.That(
                "missing_scope_refusal_names_the_scope: refusal names the missing scope",
                error != null && error.Message.Contains("external-network", StringComparison.Ordinal));
            Check.That(
                "missing_scope_refusal_names_the_scope: error carries the scope",
                error != null && error.Scope == Scope.ExternalNetwork);
        }
    }
}
