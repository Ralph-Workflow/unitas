using System.Collections.Generic;
using UnityEngine;
using UnityEngine.SceneManagement;

public class GameManager : MonoBehaviour
{
    public TextMesh scoreText;
    public TextMesh completionText;
    public int totalPickups = 12;

    private readonly HashSet<int> collected = new HashSet<int>();
    private int score;

    private void Start()
    {
        score = 0;
        collected.Clear();
        UpdateScoreText();
        if (completionText != null)
        {
            completionText.gameObject.SetActive(false);
        }
    }

    private void Update()
    {
        if (score >= totalPickups && Input.GetKeyDown(KeyCode.R))
        {
            Restart();
        }
    }

    public void Collect(GameObject pickup)
    {
        if (!collected.Add(pickup.GetInstanceID()))
        {
            return;
        }
        score++;
        UpdateScoreText();
        if (score >= totalPickups && completionText != null)
        {
            completionText.gameObject.SetActive(true);
        }
    }

    public void Restart()
    {
        SceneManager.LoadScene(SceneManager.GetActiveScene().buildIndex);
    }

    private void UpdateScoreText()
    {
        if (scoreText != null)
        {
            scoreText.text = "Score " + score;
        }
    }
}
