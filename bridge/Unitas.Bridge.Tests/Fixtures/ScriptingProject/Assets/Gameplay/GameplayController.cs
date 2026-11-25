using MyGame.Core;
using UnityEngine;

namespace MyGame.Gameplay
{
    public class GameplayController : MonoBehaviour
    {
        public string Status = string.Empty;

        public void RefreshStatus()
        {
            Status = CoreExtensionHelper.DescribeCount(42) + ":" + CoreCounter.ActiveProfileSummary();
        }
    }
}
