# Reference Change Request: Bonus Pickup Type

Add a new pickup type to the Marble Rush sample that:

1. Awards a distinct score value of **5** points (multiplied by the active combo multiplier, distinct from the ordinary pickup value of **1**).
2. Uses a distinct material (`Assets/Materials/BonusPickup.mat` with a distinct magenta color `{r: 0.92, g: 0.22, b: 0.85, a: 1}`) and a distinct binary collection sound (`Assets/Audio/BonusChime.wav`, both created and imported through `assets.import`), playing the collection sound (`CollectChime`) at a distinct pitch (`1.35`) and volume (`0.95`) when collected.
3. Appears at the specified position `(0, 0.75, -2.25)` in the arena as `BonusPickup_1` under the `Pickups` root in `Assets/Scenes/MarbleRush.unity`.
4. Shows a score-feedback message in the UI (`Bonus +<award>!`) when collected and logs `UI feedback: Bonus +<award>!`.
5. Includes a project test (`Assets/Tests/BonusPickupTests.cs` in `Assets/Tests/MarbleRush.Tests.asmdef`) that checks the awarded score (`5` points).
