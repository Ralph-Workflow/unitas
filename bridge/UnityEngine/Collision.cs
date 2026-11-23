namespace UnityEngine
{
    public class Collision
    {
        public Collision(GameObject? gameObject)
        {
            this.gameObject = gameObject;
        }

        public GameObject? gameObject { get; }
    }
}
