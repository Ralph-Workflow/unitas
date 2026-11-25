using UnityEngine;

public class PlayerController : MonoBehaviour
{
    public float speed = 12f;

    private Rigidbody body;

    private void Start()
    {
        body = GetComponent<Rigidbody>();
    }

    private void FixedUpdate()
    {
        float moveHorizontal = Input.GetAxis("Horizontal");
        float moveVertical = Input.GetAxis("Vertical");
        Vector3 movement = new Vector3(moveHorizontal, 0.0f, moveVertical);
        body.AddForce(movement * speed);
    }

    private void OnTriggerEnter(Collider other)
    {
        if (!other.CompareTag("Pickup"))
        {
            return;
        }
        GameManager manager = FindObjectOfType<GameManager>();
        if (manager != null)
        {
            manager.Collect(other.gameObject);
        }
        other.gameObject.SetActive(false);
    }
}
