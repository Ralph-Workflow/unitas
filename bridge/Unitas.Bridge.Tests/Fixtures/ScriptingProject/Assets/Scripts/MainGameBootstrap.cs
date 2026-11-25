using MyGame.Core;
using MyGame.Gameplay;
using UnityEngine;

public class MainGameBootstrap : MonoBehaviour
{
    public int TotalScore;

    public void ComputeTotal()
    {
        TotalScore = FirstPassUtility.ComputeBonus(100) + CoreExtensionHelper.DescribeCount(1).Length;
    }
}
