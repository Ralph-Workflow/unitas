using System;
using System.Collections.Generic;

namespace Unitas.Editor.Mcp
{
    public sealed record SessionIdentity(string Agent, string Client, string AuthorizedBy);

    public enum GrantAuthority
    {
        Person,
        SessionInherited,
    }

    public sealed class Session
    {
        private readonly SortedSet<Scope> _scopes = new SortedSet<Scope>();

        internal Session(string id, SessionIdentity identity)
        {
            Id = id;
            Identity = identity;
        }

        public string Id { get; }

        public SessionIdentity Identity { get; }

        public IReadOnlyCollection<Scope> Scopes => _scopes;

        public bool Holds(Scope scope)
        {
            return _scopes.Contains(scope);
        }

        internal void Grant(Scope scope)
        {
            _scopes.Add(scope);
        }
    }

    public sealed class Authorizer
    {
        private readonly Dictionary<string, Session> _sessions = new Dictionary<string, Session>();
        private readonly Dictionary<string, SortedSet<Scope>> _standing = new Dictionary<string, SortedSet<Scope>>();
        private long _nextSession;

        public string CreateSession(SessionIdentity identity)
        {
            _nextSession++;
            string id = "session-" + _nextSession;
            Session session = new Session(id, identity);
            session.Grant(Scope.ReadProject);
            _sessions.Add(id, session);
            return id;
        }

        public Session GetSession(string id)
        {
            if (!_sessions.TryGetValue(id, out Session? session))
            {
                throw new McpError.UnknownSession(id);
            }

            return session;
        }

        public IEnumerable<Session> Sessions => _sessions.Values;

        public void GrantScope(string id, Scope scope, GrantAuthority authority)
        {
            if (authority == GrantAuthority.SessionInherited)
            {
                throw new McpError.SelfWidening(scope, "session " + id);
            }

            GetSession(id).Grant(scope);
        }

        public void GrantScopeWithCredential(
            string id,
            Scope scope,
            IPersonCredential credential,
            string? candidateToken,
            string? transportName = null)
        {
            ArgumentNullException.ThrowIfNull(credential);
            Session session = GetSession(id);
            string effectiveTransport = string.IsNullOrEmpty(transportName)
                ? session.Identity.Client
                : transportName;
            credential.RequirePerson(candidateToken, effectiveTransport, "grant:" + Scopes.AsString(scope));
            session.Grant(scope);
        }

        public void GrantStanding(string agent, Scope scope, GrantAuthority authority)
        {
            if (authority == GrantAuthority.SessionInherited)
            {
                throw new McpError.SelfWidening(scope, "standing grant for agent " + agent);
            }

            if (!_standing.TryGetValue(agent, out SortedSet<Scope>? scopes))
            {
                scopes = new SortedSet<Scope>();
                _standing.Add(agent, scopes);
            }

            scopes.Add(scope);
        }

        public void GrantStandingWithCredential(
            string agent,
            Scope scope,
            IPersonCredential credential,
            string? candidateToken,
            string? transportName = null)
        {
            ArgumentNullException.ThrowIfNull(credential);
            credential.RequirePerson(candidateToken, transportName, "standing-grant:" + Scopes.AsString(scope));
            GrantStanding(agent, scope, GrantAuthority.Person);
        }

        public void Authorize(string id, Scope scope)
        {
            Session session = GetSession(id);
            if (session.Holds(scope))
            {
                return;
            }

            if (_standing.TryGetValue(session.Identity.Agent, out SortedSet<Scope>? standing)
                && standing.Contains(scope))
            {
                return;
            }

            throw new McpError.MissingScope(scope, id);
        }
    }
}
