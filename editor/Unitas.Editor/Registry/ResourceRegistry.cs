using System;
using System.Collections.Generic;
using System.Text.Json.Nodes;

namespace Unitas.Editor.Mcp
{
    public delegate ResourceContent ResourceReader();

    public sealed record ResourceDefinition(string Uri, string Name, string Description, string MimeType, Scope RequiredScope)
    {
        public JsonNode ToValue()
        {
            return new JsonObject
            {
                ["uri"] = Uri,
                ["name"] = Name,
                ["description"] = Description,
                ["mimeType"] = MimeType,
                ["requiredScope"] = Scopes.AsString(RequiredScope),
            };
        }
    }

    public sealed record ResourceContent(string Origin, JsonNode Payload)
    {
        public static ResourceContent Untrusted(string origin, JsonNode payload)
        {
            return new ResourceContent(origin, payload);
        }

        public JsonNode ToEnvelopedValue()
        {
            return UntrustedContent.Ensure(Origin, Payload.DeepClone());
        }
    }

    internal sealed class RegisteredResource
    {
        public RegisteredResource(ResourceDefinition definition, ResourceReader reader)
        {
            Definition = definition;
            Reader = reader;
        }

        public ResourceDefinition Definition { get; }

        public ResourceReader Reader { get; }
    }

    public sealed class ResourceRegistry
    {
        // Sorted so listing order matches the BTreeMap ordering of the Rust registry.
        private readonly SortedDictionary<string, RegisteredResource> resources =
            new SortedDictionary<string, RegisteredResource>(StringComparer.Ordinal);

        public void Register(ResourceDefinition definition, ResourceReader reader)
        {
            ArgumentNullException.ThrowIfNull(definition);
            ArgumentNullException.ThrowIfNull(reader);
            if (resources.ContainsKey(definition.Uri))
            {
                throw new McpError.DuplicateResource(definition.Uri);
            }

            resources.Add(definition.Uri, new RegisteredResource(definition, reader));
        }

        public IReadOnlyList<ResourceDefinition> List()
        {
            List<ResourceDefinition> definitions = new List<ResourceDefinition>();
            foreach (RegisteredResource resource in resources.Values)
            {
                definitions.Add(resource.Definition);
            }

            return definitions;
        }

        public ResourceDefinition Get(string uri)
        {
            return GetRegistered(uri).Definition;
        }

        internal RegisteredResource GetRegistered(string uri)
        {
            if (!resources.TryGetValue(uri, out RegisteredResource? resource))
            {
                throw new McpError.UnknownResource(uri);
            }

            return resource;
        }
    }
}
