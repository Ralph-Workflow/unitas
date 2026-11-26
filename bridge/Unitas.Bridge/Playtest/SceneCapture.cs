using System;
using UnityEngine;

namespace Unitas.Bridge.Playtest
{
    /// <summary>
    /// Makes the loaded scene's collider objects drawable so a render capture shows the real scene.
    /// The scene loader keeps collider shapes but imports no renderable mesh, so every collider
    /// object gets proxy geometry sized to its collider and drawn by the engine's own frame path.
    /// </summary>
    internal static class SceneCapture
    {
        private const float SceneProbeRadius = 1000f;
        private const int SphereSegments = 12;
        private const int SphereRings = 8;
        private const float MinScale = 0.0001f;

        private static readonly Color BoxColor = new Color(0.30f, 0.35f, 0.42f);
        private static readonly Color SphereColor = new Color(0.92f, 0.45f, 0.25f);
        private static readonly Color CapsuleColor = new Color(0.25f, 0.65f, 0.55f);

        /// <summary>Gives every collider object in the loaded scene drawable geometry.</summary>
        internal static void EnsureSceneRenderables()
        {
            Collider[] colliders = Physics.OverlapSphere(Vector3.zero, SceneProbeRadius, Physics.DefaultRaycastLayers);
            for (int index = 0; index < colliders.Length; index++)
            {
                Collider collider = colliders[index];
                if (collider == null || collider.isTrigger)
                {
                    continue;
                }

                GameObject? owner = collider.gameObject;
                if (owner == null || owner.GetComponent<MeshRenderer>() != null)
                {
                    continue;
                }

                MeshFilter filter = owner.GetComponent<MeshFilter>() ?? owner.AddComponent<MeshFilter>();
                filter.sharedMesh = ProxyMesh(collider);
                MeshRenderer renderer = owner.GetComponent<MeshRenderer>() ?? owner.AddComponent<MeshRenderer>();
                renderer.sharedMaterial = ProxyMaterial(collider);
            }
        }

        private static Mesh ProxyMesh(Collider collider)
        {
            Mesh mesh = new Mesh();
            Vector3 scale = collider.transform.lossyScale;
            if (collider is BoxCollider box)
            {
                // The runtime keeps collider shapes in world units, so the proxy is sized in world
                // units and divided by the transform scale the render model applies.
                BuildBox(mesh, Divide(box.size, scale), box.center);
                return mesh;
            }

            if (collider is SphereCollider sphere)
            {
                BuildSphere(mesh, sphere.radius / Average(scale), sphere.center);
                return mesh;
            }

            if (collider is CapsuleCollider capsule)
            {
                BuildCapsule(mesh, capsule.radius / Average(scale), capsule.height / Average(scale), capsule.direction, capsule.center);
                return mesh;
            }

            return mesh;
        }

        private static Material ProxyMaterial(Collider collider)
        {
            Material material = new Material();
            if (collider is BoxCollider)
            {
                material.color = BoxColor;
            }
            else if (collider is SphereCollider)
            {
                material.color = SphereColor;
            }
            else
            {
                material.color = CapsuleColor;
            }

            return material;
        }

        private static Vector3 Divide(Vector3 value, Vector3 scale)
        {
            return new Vector3(
                value.x / Math.Max(Math.Abs(scale.x), MinScale),
                value.y / Math.Max(Math.Abs(scale.y), MinScale),
                value.z / Math.Max(Math.Abs(scale.z), MinScale));
        }

        private static float Average(Vector3 scale)
        {
            return Math.Max((Math.Abs(scale.x) + Math.Abs(scale.y) + Math.Abs(scale.z)) / 3f, MinScale);
        }

        private static void BuildBox(Mesh mesh, Vector3 size, Vector3 center)
        {
            float hx = size.x * 0.5f;
            float hy = size.y * 0.5f;
            float hz = size.z * 0.5f;
            Vector3[] corners =
            {
                new Vector3(-hx, -hy, -hz),
                new Vector3(hx, -hy, -hz),
                new Vector3(hx, hy, -hz),
                new Vector3(-hx, hy, -hz),
                new Vector3(-hx, -hy, hz),
                new Vector3(hx, -hy, hz),
                new Vector3(hx, hy, hz),
                new Vector3(-hx, hy, hz),
            };
            Vector3[] faceNormals =
            {
                new Vector3(0f, 0f, -1f),
                new Vector3(0f, 0f, 1f),
                new Vector3(-1f, 0f, 0f),
                new Vector3(1f, 0f, 0f),
                new Vector3(0f, -1f, 0f),
                new Vector3(0f, 1f, 0f),
            };
            int[][] faceCorners =
            {
                new[] { 0, 3, 2, 1 },
                new[] { 4, 5, 6, 7 },
                new[] { 0, 4, 7, 3 },
                new[] { 1, 2, 6, 5 },
                new[] { 0, 1, 5, 4 },
                new[] { 3, 7, 6, 2 },
            };

            Vector3[] vertices = new Vector3[24];
            Vector3[] normals = new Vector3[24];
            int[] triangles = new int[36];
            for (int face = 0; face < faceCorners.Length; face++)
            {
                for (int corner = 0; corner < 4; corner++)
                {
                    int vertex = (face * 4) + corner;
                    vertices[vertex] = corners[faceCorners[face][corner]] + center;
                    normals[vertex] = faceNormals[face];
                }

                int baseIndex = face * 4;
                triangles[face * 6] = baseIndex;
                triangles[(face * 6) + 1] = baseIndex + 1;
                triangles[(face * 6) + 2] = baseIndex + 2;
                triangles[(face * 6) + 3] = baseIndex;
                triangles[(face * 6) + 4] = baseIndex + 2;
                triangles[(face * 6) + 5] = baseIndex + 3;
            }

            mesh.vertices = vertices;
            mesh.normals = normals;
            mesh.triangles = triangles;
        }

        private static void BuildSphere(Mesh mesh, float radius, Vector3 center)
        {
            int vertexCount = (SphereSegments + 1) * (SphereRings + 1);
            Vector3[] vertices = new Vector3[vertexCount];
            Vector3[] normals = new Vector3[vertexCount];
            for (int ring = 0; ring <= SphereRings; ring++)
            {
                float theta = (float)Math.PI * ring / SphereRings;
                float sinTheta = (float)Math.Sin(theta);
                float cosTheta = (float)Math.Cos(theta);
                for (int segment = 0; segment <= SphereSegments; segment++)
                {
                    float phi = (float)(2.0 * Math.PI * segment / SphereSegments);
                    Vector3 normal = new Vector3(sinTheta * (float)Math.Cos(phi), cosTheta, sinTheta * (float)Math.Sin(phi));
                    int index = (ring * (SphereSegments + 1)) + segment;
                    normals[index] = normal;
                    vertices[index] = (normal * radius) + center;
                }
            }

            int[] triangles = new int[SphereSegments * SphereRings * 6];
            int cursor = 0;
            for (int ring = 0; ring < SphereRings; ring++)
            {
                for (int segment = 0; segment < SphereSegments; segment++)
                {
                    int topLeft = (ring * (SphereSegments + 1)) + segment;
                    int topRight = topLeft + 1;
                    int bottomLeft = ((ring + 1) * (SphereSegments + 1)) + segment;
                    int bottomRight = bottomLeft + 1;
                    if (ring != 0)
                    {
                        triangles[cursor++] = topLeft;
                        triangles[cursor++] = bottomLeft;
                        triangles[cursor++] = bottomRight;
                    }

                    if (ring != SphereRings - 1)
                    {
                        triangles[cursor++] = topLeft;
                        triangles[cursor++] = bottomRight;
                        triangles[cursor++] = topRight;
                    }
                }
            }

            mesh.vertices = vertices;
            mesh.normals = normals;
            mesh.triangles = triangles;
        }

        private static void BuildCapsule(Mesh mesh, float radius, float height, int direction, Vector3 center)
        {
            // A capsule is drawn as the cylinder that spans its two cap centers.
            float halfHeight = Math.Max(height * 0.5f - radius, 0f);
            Vector3 axis = direction == 0 ? new Vector3(1f, 0f, 0f) : direction == 2 ? new Vector3(0f, 0f, 1f) : new Vector3(0f, 1f, 0f);
            Vector3 side = axis == Vector3.up ? Vector3.forward : Vector3.up;
            Vector3[] vertices = new Vector3[8];
            Vector3[] normals = new Vector3[8];
            int[] triangles = new int[24];
            int cursor = 0;
            for (int end = 0; end < 2; end++)
            {
                float offset = end == 0 ? -halfHeight : halfHeight;
                for (int corner = 0; corner < 4; corner++)
                {
                    float angle = (float)(Math.PI * 0.5 * corner);
                    Vector3 radial = (side * (float)Math.Cos(angle)) + (Vector3.Cross(axis, side) * (float)Math.Sin(angle));
                    Vector3 position = (axis * offset) + (radial * radius);
                    int index = (end * 4) + corner;
                    vertices[index] = position + center;
                    normals[index] = radial;
                }
            }

            for (int end = 0; end < 2; end++)
            {
                int baseIndex = end * 4;
                triangles[cursor++] = baseIndex;
                triangles[cursor++] = baseIndex + 1;
                triangles[cursor++] = baseIndex + 2;
                triangles[cursor++] = baseIndex;
                triangles[cursor++] = baseIndex + 2;
                triangles[cursor++] = baseIndex + 3;
            }

            for (int corner = 0; corner < 4; corner++)
            {
                int next = (corner + 1) % 4;
                int bottom = corner;
                int bottomNext = next;
                int top = 4 + corner;
                int topNext = 4 + next;
                triangles[cursor++] = bottom;
                triangles[cursor++] = top;
                triangles[cursor++] = topNext;
                triangles[cursor++] = bottom;
                triangles[cursor++] = topNext;
                triangles[cursor++] = bottomNext;
            }

            mesh.vertices = vertices;
            mesh.normals = normals;
            mesh.triangles = triangles;
        }
    }
}
