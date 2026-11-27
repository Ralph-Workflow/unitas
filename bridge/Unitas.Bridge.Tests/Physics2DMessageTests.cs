using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using Unitas.Bridge.Scripting;
using UnityEngine;

namespace Unitas.Bridge.Tests
{
    public sealed class TriggerOrderProbe2DScript : MonoBehaviour
    {
        public readonly List<string> Trace = new List<string>();
        public int TriggerEnterCount { get; private set; }
        public int TriggerStayCount { get; private set; }
        public int TriggerExitCount { get; private set; }
        public string LastTriggerName { get; private set; } = string.Empty;

        public override void FixedUpdate()
        {
            Trace.Add("FixedUpdate");
        }

        public override void OnTriggerEnter2D(Collider2D? other)
        {
            TriggerEnterCount++;
            LastTriggerName = other?.gameObject?.name ?? string.Empty;
            Trace.Add("OnTriggerEnter2D");
        }

        public override void OnTriggerStay2D(Collider2D? other)
        {
            TriggerStayCount++;
            Trace.Add("OnTriggerStay2D");
        }

        public override void OnTriggerExit2D(Collider2D? other)
        {
            TriggerExitCount++;
            Trace.Add("OnTriggerExit2D");
        }

        public override void Update()
        {
            Trace.Add("Update");
        }
    }

    public sealed class LandingNormalProbe2DScript : MonoBehaviour
    {
        public int CollisionEnterCount { get; private set; }
        public int ContactCount { get; private set; }
        public Vector2 FirstContactNormal { get; private set; }
        public Vector2 SummaryNormal { get; private set; }
        public string OtherName { get; private set; } = string.Empty;

        public override void OnCollisionEnter2D(Collision2D collision)
        {
            CollisionEnterCount++;
            ContactCount = collision.contactCount;
            SummaryNormal = collision.normal;
            OtherName = collision.gameObject?.name ?? string.Empty;
            if (collision.contactCount > 0)
            {
                FirstContactNormal = collision.GetContact(0).normal;
            }
        }
    }

    public sealed class TimeScaleProbe2DScript : MonoBehaviour
    {
        public int FixedUpdateCount { get; private set; }
        public int UpdateCount { get; private set; }

        public override void FixedUpdate()
        {
            FixedUpdateCount++;
        }

        public override void Update()
        {
            UpdateCount++;
        }
    }

    public sealed class SimulatedFadeProbe2DScript : MonoBehaviour
    {
        public int TriggerEnterCount { get; private set; }
        public int TriggerStayCount { get; private set; }
        public int TriggerExitCount { get; private set; }
        public int CollisionEnterCount { get; private set; }
        public int CollisionStayCount { get; private set; }
        public int CollisionExitCount { get; private set; }

        public void ResetCounts()
        {
            TriggerEnterCount = 0;
            TriggerStayCount = 0;
            TriggerExitCount = 0;
            CollisionEnterCount = 0;
            CollisionStayCount = 0;
            CollisionExitCount = 0;
        }

        public override void OnTriggerEnter2D(Collider2D? other)
        {
            TriggerEnterCount++;
        }

        public override void OnTriggerStay2D(Collider2D? other)
        {
            TriggerStayCount++;
        }

        public override void OnTriggerExit2D(Collider2D? other)
        {
            TriggerExitCount++;
        }

        public override void OnCollisionEnter2D(Collision2D collision)
        {
            CollisionEnterCount++;
        }

        public override void OnCollisionStay2D(Collision2D collision)
        {
            CollisionStayCount++;
        }

        public override void OnCollisionExit2D(Collision2D collision)
        {
            CollisionExitCount++;
        }
    }

    public sealed class KinematicPlatformMover2DScript : MonoBehaviour
    {
        private Rigidbody2D? _body;
        private Vector2 _currentPosition;
        private bool _moving;

        public Vector2 CurrentTargetPosition => _currentPosition;

        public float SpeedUnitsPerSecond { get; set; } = 2f;

        public void BeginWaypointLeg(Vector2 startPosition)
        {
            _body = gameObject.GetComponent<Rigidbody2D>();
            _currentPosition = startPosition;
            _moving = true;
        }

        public override void FixedUpdate()
        {
            if (!_moving || _body == null)
            {
                return;
            }
            _currentPosition = new Vector2(_currentPosition.x + SpeedUnitsPerSecond * Time.fixedDeltaTime, _currentPosition.y);
            _body.MovePosition(_currentPosition);
        }
    }

    public sealed class DynamicRiderProbe2DScript : MonoBehaviour
    {
        public int CollisionEnterCount { get; private set; }
        public int CollisionStayCount { get; private set; }

        public override void OnCollisionEnter2D(Collision2D collision)
        {
            CollisionEnterCount++;
        }

        public override void OnCollisionStay2D(Collision2D collision)
        {
            CollisionStayCount++;
        }
    }

    public static class Physics2DMessageTests
    {
        private const ulong ProbeGuidHi = 0x4400440044004400UL;
        private const ulong TriggerProbeGuidLo = 0x0000000000000001UL;
        private const ulong NormalProbeGuidLo = 0x0000000000000002UL;
        private const ulong TimeScaleProbeGuidLo = 0x0000000000000003UL;
        private const ulong SimulatedProbeGuidLo = 0x0000000000000004UL;
        private const ulong PlatformProbeGuidLo = 0x0000000000000005UL;
        private const ulong RiderProbeGuidLo = 0x0000000000000006UL;
        private const ulong WickEndToEndProbeGuidLo = 0x0000000000000007UL;

        public static void Run()
        {
            TestOnTriggerEnter2DFiresOnceAfterFixedUpdateAndBeforeUpdate();
            TestOnCollisionEnter2DReportsContactNormalPointingAwayFromSurface();
            TestTimeScaleZeroHalts2DSimulationWhileUnscaledTimeAdvances();
            TestSimulatedFalseRemoves2DBodyFromContactsAndQueries();
            TestKinematicMovePositionInFixedUpdateCarriesRestingRiderWithBoundedDrift();
            TestWicksLanternEndToEndPlatformerScenario();
        }

        private static void TestOnTriggerEnter2DFiresOnceAfterFixedUpdateAndBeforeUpdate()
        {
            string tempDir = CreateTempSceneDirectory("trigger_order");
            string scenePath = Path.Combine(tempDir, "trigger_order.unity");
            File.WriteAllText(scenePath, TriggerOverlapSceneYaml);

            float savedScale = Time.timeScale;
            Time.timeScale = 1f;
            TriggerOrderProbe2DScript probe = new TriggerOrderProbe2DScript();
            ScriptGlueRegistry.Clear();
            ScriptGlueRegistry.Register(ProbeGuidHi, TriggerProbeGuidLo, () => probe);

            try
            {
                using (BridgeHost host = new BridgeHost())
                {
                    host.LoadScene(scenePath);
                    host.Tick(0.02);
                    probe.Trace.Clear();

                    host.Tick(0.02);
                    Check.Equal(
                        "Frame 1 dispatches FixedUpdate then OnTriggerEnter2D then Update",
                        "FixedUpdate, OnTriggerEnter2D, Update",
                        string.Join(", ", probe.Trace));
                    Check.Equal("OnTriggerEnter2D fires once on entry frame", 1, probe.TriggerEnterCount);
                    Check.Equal("OnTriggerEnter2D resolves the overlapping trigger object", "EmberTrigger2D", probe.LastTriggerName);

                    probe.Trace.Clear();
                    host.Tick(0.02);
                    host.Tick(0.02);
                    Check.Equal("OnTriggerEnter2D does not fire again while still overlapping", 1, probe.TriggerEnterCount);
                    Check.That("Subsequent overlapping frames fire OnTriggerStay2D instead", probe.TriggerStayCount >= 2);
                    Check.That(
                        "Subsequent frame trace contains no OnTriggerEnter2D",
                        !probe.Trace.Contains("OnTriggerEnter2D"));
                    host.Shutdown();
                }
            }
            finally
            {
                ScriptGlueRegistry.Clear();
                Time.timeScale = savedScale;
                CleanupDirectory(tempDir);
            }
        }

        private static void TestOnCollisionEnter2DReportsContactNormalPointingAwayFromSurface()
        {
            string tempDir = CreateTempSceneDirectory("landing_normal");
            string scenePath = Path.Combine(tempDir, "landing_normal.unity");
            File.WriteAllText(scenePath, LandingNormalSceneYaml);

            float savedScale = Time.timeScale;
            Time.timeScale = 1f;
            LandingNormalProbe2DScript probe = new LandingNormalProbe2DScript();
            ScriptGlueRegistry.Clear();
            ScriptGlueRegistry.Register(ProbeGuidHi, NormalProbeGuidLo, () => probe);

            try
            {
                using (BridgeHost host = new BridgeHost())
                {
                    host.LoadScene(scenePath);
                    for (int tick = 0; tick < 30; tick++)
                    {
                        host.Tick(0.02);
                        if (probe.CollisionEnterCount > 0)
                        {
                            break;
                        }
                    }
                    host.Shutdown();
                }

                Check.Equal("Landing falling body receives OnCollisionEnter2D", 1, probe.CollisionEnterCount);
                Check.Equal("Landing collision resolves Ground2D", "Ground2D", probe.OtherName);
                Check.That("Landing collision carries at least one contact point", probe.ContactCount > 0);
                Check.That(
                    "Landing GetContact(0).normal points upward away from ground surface (y > 0.9, |x| < 0.05)",
                    probe.FirstContactNormal.y > 0.9f && Math.Abs(probe.FirstContactNormal.x) < 0.05f);
                Check.That(
                    "Landing collision.normal points upward away from ground surface (y > 0.9, |x| < 0.05)",
                    probe.SummaryNormal.y > 0.9f && Math.Abs(probe.SummaryNormal.x) < 0.05f);
            }
            finally
            {
                ScriptGlueRegistry.Clear();
                Time.timeScale = savedScale;
                CleanupDirectory(tempDir);
            }
        }

        private static void TestTimeScaleZeroHalts2DSimulationWhileUnscaledTimeAdvances()
        {
            string tempDir = CreateTempSceneDirectory("timescale_zero");
            string scenePath = Path.Combine(tempDir, "timescale_zero.unity");
            File.WriteAllText(scenePath, TimeScaleFallingSceneYaml);

            float savedScale = Time.timeScale;
            TimeScaleProbe2DScript probe = new TimeScaleProbe2DScript();
            ScriptGlueRegistry.Clear();
            ScriptGlueRegistry.Register(ProbeGuidHi, TimeScaleProbeGuidLo, () => probe);

            try
            {
                Time.timeScale = 1f;
                using (BridgeHost host = new BridgeHost())
                {
                    host.LoadScene(scenePath);
                    host.Tick(0.02);
                    host.Tick(0.02);

                    Rigidbody2D? body = probe.gameObject.GetComponent<Rigidbody2D>();
                    Check.That("Falling 2D body has Rigidbody2D", body != null);
                    Vector2 velocityBeforePause = body!.velocity;
                    Check.That("Falling 2D body gains downward velocity at timeScale=1", velocityBeforePause.y < -0.1f);
                    Check.That("Body is found near starting height before pause", Physics2D.OverlapPoint(new Vector2(0f, 3.99f)).Length == 1);

                    Time.timeScale = 0f;
                    int fixedBeforePause = probe.FixedUpdateCount;
                    int updateBeforePause = probe.UpdateCount;
                    float unscaledBeforePause = Time.unscaledTime;
                    float scaledBeforePause = Time.time;

                    for (int frame = 0; frame < 20; frame++)
                    {
                        host.Tick(0.02);
                    }

                    Check.Equal("timeScale=0 stops FixedUpdate from running", fixedBeforePause, probe.FixedUpdateCount);
                    Check.Equal("timeScale=0 still runs Update every frame", updateBeforePause + 20, probe.UpdateCount);
                    Check.Equal("timeScale=0 holds scaled Time.time constant", scaledBeforePause, Time.time);
                    Check.That("timeScale=0 advances Time.unscaledTime across frames", Time.unscaledTime >= unscaledBeforePause + 0.39f);
                    Check.Equal("timeScale=0 reports Time.unscaledDeltaTime for the frame", 0.02f, Time.unscaledDeltaTime);
                    Check.Equal("timeScale=0 freezes 2D body vertical velocity", velocityBeforePause.y, body.velocity.y);
                    Check.Equal(
                        "timeScale=0 freezes 2D body position in World2D",
                        1,
                        Physics2D.OverlapPoint(new Vector2(0f, 3.99f)).Length);

                    Time.timeScale = 1f;
                    for (int frame = 0; frame < 25; frame++)
                    {
                        host.Tick(0.02);
                    }

                    Check.That("Resuming timeScale=1 resumes FixedUpdate", probe.FixedUpdateCount > fixedBeforePause);
                    Check.That("Resuming timeScale=1 accelerates body further downward", body.velocity.y < velocityBeforePause.y - 1f);
                    Check.Equal(
                        "Resuming timeScale=1 moves body away from frozen position",
                        0,
                        Physics2D.OverlapPoint(new Vector2(0f, 3.99f)).Length);
                    host.Shutdown();
                }
            }
            finally
            {
                ScriptGlueRegistry.Clear();
                Time.timeScale = savedScale;
                CleanupDirectory(tempDir);
            }
        }

        private static void TestSimulatedFalseRemoves2DBodyFromContactsAndQueries()
        {
            string tempDir = CreateTempSceneDirectory("simulated_fade");
            string scenePath = Path.Combine(tempDir, "simulated_fade.unity");
            File.WriteAllText(scenePath, SimulatedFadeSceneYaml);

            float savedScale = Time.timeScale;
            Time.timeScale = 1f;
            SimulatedFadeProbe2DScript probe = new SimulatedFadeProbe2DScript();
            ScriptGlueRegistry.Clear();
            ScriptGlueRegistry.Register(ProbeGuidHi, SimulatedProbeGuidLo, () => probe);

            try
            {
                using (BridgeHost host = new BridgeHost())
                {
                    host.LoadScene(scenePath);
                    host.Tick(0.02);
                    host.Tick(0.02);

                    Rigidbody2D? body = probe.gameObject.GetComponent<Rigidbody2D>();
                    Check.That("Simulated-fade body has Rigidbody2D", body != null);
                    Check.Equal("Body enters trigger while simulated=true", 1, probe.TriggerEnterCount);
                    Check.Equal("Body enters ground collision while simulated=true", 1, probe.CollisionEnterCount);

                    Collider2D[] beforeDisable = Physics2D.OverlapPoint(new Vector2(0f, 0.75f));
                    Check.That(
                        "OverlapPoint finds the body collider while simulated=true",
                        ContainsGameObjectNamed(beforeDisable, "WickBody2D"));

                    probe.ResetCounts();
                    body!.simulated = false;
                    Check.Equal("Rigidbody2D.simulated reads back false", false, body.simulated);

                    Collider2D[] afterDisableImmediate = Physics2D.OverlapPoint(new Vector2(0f, 0.75f));
                    Check.That(
                        "Setting simulated=false immediately removes body collider from 2D overlap queries",
                        !ContainsGameObjectNamed(afterDisableImmediate, "WickBody2D"));

                    host.Tick(0.02);
                    Check.Equal("Setting simulated=false emits OnTriggerExit2D for active trigger contact", 1, probe.TriggerExitCount);
                    Check.Equal("Setting simulated=false emits OnCollisionExit2D for active collision contact", 1, probe.CollisionExitCount);
                    Check.Equal("Setting simulated=false does not emit OnTriggerStay2D", 0, probe.TriggerStayCount);
                    Check.Equal("Setting simulated=false does not emit OnCollisionStay2D", 0, probe.CollisionStayCount);

                    probe.ResetCounts();
                    host.Tick(0.02);
                    host.Tick(0.02);
                    Check.Equal("While simulated=false no trigger stay events fire", 0, probe.TriggerStayCount);
                    Check.Equal("While simulated=false no collision stay events fire", 0, probe.CollisionStayCount);
                    Check.Equal("While simulated=false no exit events repeat", 0, probe.TriggerExitCount + probe.CollisionExitCount);

                    host.Shutdown();
                }
            }
            finally
            {
                ScriptGlueRegistry.Clear();
                Time.timeScale = savedScale;
                CleanupDirectory(tempDir);
            }
        }

        private static void TestKinematicMovePositionInFixedUpdateCarriesRestingRiderWithBoundedDrift()
        {
            string tempDir = CreateTempSceneDirectory("kinematic_rider");
            string scenePath = Path.Combine(tempDir, "kinematic_rider.unity");
            File.WriteAllText(scenePath, KinematicRiderSceneYaml);

            float savedScale = Time.timeScale;
            Time.timeScale = 1f;
            KinematicPlatformMover2DScript platformScript = new KinematicPlatformMover2DScript();
            DynamicRiderProbe2DScript riderScript = new DynamicRiderProbe2DScript();
            ScriptGlueRegistry.Clear();
            ScriptGlueRegistry.Register(ProbeGuidHi, PlatformProbeGuidLo, () => platformScript);
            ScriptGlueRegistry.Register(ProbeGuidHi, RiderProbeGuidLo, () => riderScript);

            try
            {
                using (BridgeHost host = new BridgeHost())
                {
                    host.LoadScene(scenePath);
                    for (int settle = 0; settle < 25; settle++)
                    {
                        host.Tick(0.02);
                    }

                    Check.That("Rider lands on kinematic platform before waypoint leg", riderScript.CollisionEnterCount > 0);
                    Vector3 riderBeforeLeg = riderScript.transform.position;
                    Vector3 platformBeforeLeg = platformScript.transform.position;
                    float initialOffsetX = riderBeforeLeg.x - platformBeforeLeg.x;

                    platformScript.BeginWaypointLeg(new Vector2(platformBeforeLeg.x, platformBeforeLeg.y));
                    for (int step = 0; step < 50; step++)
                    {
                        host.Tick(0.02);
                    }

                    Vector2 platformAfterLeg = platformScript.CurrentTargetPosition;
                    Vector3 riderAfterLeg = riderScript.transform.position;
                    float platformTravelX = platformAfterLeg.x - platformBeforeLeg.x;
                    float expectedRiderX = platformAfterLeg.x + initialOffsetX;
                    float horizontalDrift = Math.Abs(riderAfterLeg.x - expectedRiderX);

                    Check.That(
                        "Kinematic platform moved 2.0 units over the waypoint leg (got " + platformTravelX.ToString(CultureInfo.InvariantCulture) + ")",
                        Math.Abs(platformTravelX - 2.0f) < 0.01f);
                    Check.That(
                        "Dynamic rider transform.position is carried horizontally by kinematic MovePosition (rider.x="
                        + riderAfterLeg.x.ToString(CultureInfo.InvariantCulture)
                        + ", expected="
                        + expectedRiderX.ToString(CultureInfo.InvariantCulture)
                        + ", drift="
                        + horizontalDrift.ToString(CultureInfo.InvariantCulture)
                        + ")",
                        horizontalDrift < 0.0625f);
                    host.Shutdown();
                }
            }
            finally
            {
                ScriptGlueRegistry.Clear();
                Time.timeScale = savedScale;
                CleanupDirectory(tempDir);
            }
        }

        private static void TestWicksLanternEndToEndPlatformerScenario()
        {
            string tempDir = CreateTempSceneDirectory("wicks_lantern_e2e");
            string scenePath = Path.Combine(tempDir, "wicks_lantern_e2e.unity");
            File.WriteAllText(scenePath, WicksLanternEndToEndSceneYaml);

            float savedScale = Time.timeScale;
            Time.timeScale = 1f;
            SimulatedFadeProbe2DScript wickProbe = new SimulatedFadeProbe2DScript();
            KinematicPlatformMover2DScript platformMover = new KinematicPlatformMover2DScript();
            DynamicRiderProbe2DScript riderProbe = new DynamicRiderProbe2DScript();
            ScriptGlueRegistry.Clear();
            ScriptGlueRegistry.Register(ProbeGuidHi, WickEndToEndProbeGuidLo, () => wickProbe);
            ScriptGlueRegistry.Register(ProbeGuidHi, PlatformProbeGuidLo, () => platformMover);
            ScriptGlueRegistry.Register(ProbeGuidHi, RiderProbeGuidLo, () => riderProbe);

            try
            {
                using (BridgeHost host = new BridgeHost())
                {
                    host.LoadScene(scenePath);
                    for (int step = 0; step < 15; step++)
                    {
                        host.Tick(0.02);
                    }

                    Rigidbody2D? wickBody = wickProbe.gameObject.GetComponent<Rigidbody2D>();
                    CapsuleCollider2D? wickCapsule = wickProbe.gameObject.GetComponent<CapsuleCollider2D>();
                    Check.That("Wick has Rigidbody2D and CapsuleCollider2D (0.75 x 1.375)", wickBody != null && wickCapsule != null);
                    Check.Equal("Wick capsule width is 0.75", 0.75f, wickCapsule!.size.x);
                    Check.Equal("Wick capsule height is 1.375", 1.375f, wickCapsule.size.y);
                    Check.Equal("Wick collisionDetectionMode is Continuous", (int)CollisionDetectionMode2D.Continuous, (int)wickBody!.collisionDetectionMode);
                    Check.Equal("Wick interpolation is Interpolate", (int)RigidbodyInterpolation2D.Interpolate, (int)wickBody.interpolation);
                    Check.Equal("Wick freezeRotation is true from scene m_Constraints", true, wickBody.freezeRotation);
                    GameObject? oneWayObj = GameObject.Find("OneWayPlatform2D");
                    Check.That("OneWayPlatform2D exists and carries scene layer 8", oneWayObj != null && oneWayObj.layer == 8);

                    ContactFilter2D groundFilter = new ContactFilter2D
                    {
                        useTriggers = false,
                        useOutsideNormalAngle = true,
                        minNormalAngle = 45f,
                        maxNormalAngle = 135f,
                    };
                    Vector3 wickGroundedPos = wickProbe.transform.position;
                    RaycastHit2D[] groundHits = Physics2D.BoxCast(
                        new Vector2(wickGroundedPos.x, wickGroundedPos.y + 0.5f),
                        new Vector2(0.5f, 0.2f),
                        0f,
                        new Vector2(0f, -1f),
                        2.0f,
                        groundFilter,
                        (1 << 0) | (1 << 8));
                    Check.That("Wick BoxCast with ContactFilter2D [45, 135] detects composite ground", groundHits.Length > 0);
                    if (groundHits.Length > 0)
                    {
                        float angleDeg = (float)(Math.Atan2(groundHits[0].normal.y, groundHits[0].normal.x) * (180.0 / Math.PI));
                        Check.That("Ground hit normal angle lies in [45, 135] degrees", angleDeg >= 45f && angleDeg <= 135f);
                    }

                    wickBody.velocity = new Vector2(2.5f, 0f);
                    float maxVyDeltaAcrossSeam = 0f;
                    float prevVy = wickBody.velocity.y;
                    for (int step = 0; step < 30; step++)
                    {
                        host.Tick(0.02);
                        float vy = wickBody.velocity.y;
                        float delta = Math.Abs(vy - prevVy);
                        if (delta > maxVyDeltaAcrossSeam)
                        {
                            maxVyDeltaAcrossSeam = delta;
                        }
                        prevVy = vy;
                    }
                    Check.That(
                        "Crossing CompositeCollider2D seam changes vertical velocity by at most 0.05 u/s (got "
                        + maxVyDeltaAcrossSeam.ToString(CultureInfo.InvariantCulture) + ")",
                        maxVyDeltaAcrossSeam <= 0.05f);
                    Check.Equal("Wick collects EmberTrigger2D once while crossing composite ground", 1, wickProbe.TriggerEnterCount);

                    AssertWicksLanternOneWayConveyorAndRider(host, wickProbe, wickBody, platformMover, riderProbe);
                    host.Shutdown();
                }
            }
            finally
            {
                ScriptGlueRegistry.Clear();
                Time.timeScale = savedScale;
                CleanupDirectory(tempDir);
            }
        }

        private static void AssertWicksLanternOneWayConveyorAndRider(
            BridgeHost host,
            SimulatedFadeProbe2DScript wickProbe,
            Rigidbody2D wickBody,
            KinematicPlatformMover2DScript platformMover,
            DynamicRiderProbe2DScript riderProbe)
        {
            wickBody.MovePosition(new Vector2(6f, 0.8f));
            host.Tick(0.02);
            wickProbe.ResetCounts();
            wickBody.velocity = new Vector2(0f, 9.5f);
            for (int step = 0; step < 18; step++)
            {
                host.Tick(0.02);
            }
            Check.Equal(
                "Upward pass-through of one-way PlatformEffector2D (200 deg arc) raises no collision event",
                0,
                wickProbe.CollisionEnterCount);
            Check.That(
                "Wick passes upward above the one-way platform (y="
                + wickProbe.transform.position.y.ToString(CultureInfo.InvariantCulture) + ")",
                wickProbe.transform.position.y > 2.1f);

            wickBody.MovePosition(new Vector2(12f, 1.2f));
            wickBody.velocity = Vector2.zero;
            for (int step = 0; step < 20; step++)
            {
                host.Tick(0.02);
            }
            Check.That(
                "SurfaceEffector2D conveyor accelerates Wick horizontally (vx="
                + wickBody.velocity.x.ToString(CultureInfo.InvariantCulture) + ")",
                wickBody.velocity.x > 1.0f);

            Vector3 riderBefore = riderProbe.transform.position;
            Vector3 platformBefore = platformMover.transform.position;
            float offsetBefore = riderBefore.x - platformBefore.x;
            platformMover.BeginWaypointLeg(new Vector2(platformBefore.x, platformBefore.y));
            for (int step = 0; step < 30; step++)
            {
                host.Tick(0.02);
            }
            Vector2 platformAfter = platformMover.CurrentTargetPosition;
            Vector3 riderAfter = riderProbe.transform.position;
            float drift = Math.Abs(riderAfter.x - (platformAfter.x + offsetBefore));
            Check.That(
                "Wick scenario kinematic platform rider drift < 0.0625 units (drift="
                + drift.ToString(CultureInfo.InvariantCulture) + ")",
                drift < 0.0625f);
        }

        private static bool ContainsGameObjectNamed(Collider2D[] colliders, string expectedName)
        {
            for (int index = 0; index < colliders.Length; index++)
            {
                Collider2D? col = colliders[index];
                if (col != null && col.gameObject != null && string.Equals(col.gameObject.name, expectedName, StringComparison.Ordinal))
                {
                    return true;
                }
            }
            return false;
        }

        private static string CreateTempSceneDirectory(string prefix)
        {
            string dir = Path.Combine(Path.GetTempPath(), "unitas_p2d_msg_" + prefix + "_" + Guid.NewGuid().ToString("N"));
            Directory.CreateDirectory(dir);
            return dir;
        }

        private static void CleanupDirectory(string path)
        {
            if (Directory.Exists(path))
            {
                Directory.Delete(path, true);
            }
        }

        private const string TriggerOverlapSceneYaml = @"%YAML 1.1
%TAG !u! tag:unity3d.com,2011:
--- !u!1 &100
GameObject:
  m_ObjectHideFlags: 0
  m_CorrespondingSourceObject: {fileID: 0}
  m_PrefabInstance: {fileID: 0}
  m_PrefabAsset: {fileID: 0}
  serializedVersion: 6
  m_Component:
  - component: {fileID: 101}
  - component: {fileID: 102}
  - component: {fileID: 103}
  - component: {fileID: 104}
  m_Layer: 0
  m_Name: Player2D
  m_TagString: Untagged
  m_Icon: {fileID: 0}
  m_NavMeshLayer: 0
  m_StaticEditorFlags: 0
  m_IsActive: 1
--- !u!4 &101
Transform:
  m_GameObject: {fileID: 100}
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalPosition: {x: 0, y: 0, z: 0}
  m_LocalScale: {x: 1, y: 1, z: 1}
  m_Children: []
  m_Father: {fileID: 0}
--- !u!50 &102
Rigidbody2D:
  m_GameObject: {fileID: 100}
  m_BodyType: 0
  m_Simulated: 1
  m_Mass: 1
  m_GravityScale: 0
  m_Interpolate: 0
  m_CollisionDetection: 0
--- !u!61 &103
BoxCollider2D:
  m_GameObject: {fileID: 100}
  m_IsTrigger: 0
  m_Offset: {x: 0, y: 0}
  m_Size: {x: 1, y: 1}
--- !u!114 &104
MonoBehaviour:
  m_GameObject: {fileID: 100}
  m_Enabled: 1
  m_Script: {fileID: 11500000, guid: 44004400440044000000000000000001, type: 3}
--- !u!1 &200
GameObject:
  m_ObjectHideFlags: 0
  serializedVersion: 6
  m_Component:
  - component: {fileID: 201}
  - component: {fileID: 202}
  m_Layer: 0
  m_Name: EmberTrigger2D
  m_IsActive: 1
--- !u!4 &201
Transform:
  m_GameObject: {fileID: 200}
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalPosition: {x: 0, y: 0, z: 0}
  m_LocalScale: {x: 1, y: 1, z: 1}
  m_Children: []
  m_Father: {fileID: 0}
--- !u!61 &202
BoxCollider2D:
  m_GameObject: {fileID: 200}
  m_IsTrigger: 1
  m_Offset: {x: 0, y: 0}
  m_Size: {x: 2, y: 2}
";

        private const string LandingNormalSceneYaml = @"%YAML 1.1
%TAG !u! tag:unity3d.com,2011:
--- !u!1 &100
GameObject:
  m_ObjectHideFlags: 0
  serializedVersion: 6
  m_Component:
  - component: {fileID: 101}
  - component: {fileID: 102}
  - component: {fileID: 103}
  - component: {fileID: 104}
  m_Layer: 0
  m_Name: FallingPlayer2D
  m_IsActive: 1
--- !u!4 &101
Transform:
  m_GameObject: {fileID: 100}
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalPosition: {x: 0, y: 1.2, z: 0}
  m_LocalScale: {x: 1, y: 1, z: 1}
  m_Children: []
  m_Father: {fileID: 0}
--- !u!50 &102
Rigidbody2D:
  m_GameObject: {fileID: 100}
  m_BodyType: 0
  m_Simulated: 1
  m_Mass: 1
  m_GravityScale: 1
  m_Interpolate: 0
  m_CollisionDetection: 0
--- !u!61 &103
BoxCollider2D:
  m_GameObject: {fileID: 100}
  m_IsTrigger: 0
  m_Offset: {x: 0, y: 0}
  m_Size: {x: 1, y: 1}
--- !u!114 &104
MonoBehaviour:
  m_GameObject: {fileID: 100}
  m_Enabled: 1
  m_Script: {fileID: 11500000, guid: 44004400440044000000000000000002, type: 3}
--- !u!1 &200
GameObject:
  m_ObjectHideFlags: 0
  serializedVersion: 6
  m_Component:
  - component: {fileID: 201}
  - component: {fileID: 202}
  m_Layer: 0
  m_Name: Ground2D
  m_IsActive: 1
--- !u!4 &201
Transform:
  m_GameObject: {fileID: 200}
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalPosition: {x: 0, y: 0, z: 0}
  m_LocalScale: {x: 1, y: 1, z: 1}
  m_Children: []
  m_Father: {fileID: 0}
--- !u!61 &202
BoxCollider2D:
  m_GameObject: {fileID: 200}
  m_IsTrigger: 0
  m_Offset: {x: 0, y: 0}
  m_Size: {x: 6, y: 1}
";

        private const string TimeScaleFallingSceneYaml = @"%YAML 1.1
%TAG !u! tag:unity3d.com,2011:
--- !u!1 &100
GameObject:
  m_ObjectHideFlags: 0
  serializedVersion: 6
  m_Component:
  - component: {fileID: 101}
  - component: {fileID: 102}
  - component: {fileID: 103}
  - component: {fileID: 104}
  m_Layer: 0
  m_Name: TimeScaleBody2D
  m_IsActive: 1
--- !u!4 &101
Transform:
  m_GameObject: {fileID: 100}
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalPosition: {x: 0, y: 4, z: 0}
  m_LocalScale: {x: 1, y: 1, z: 1}
  m_Children: []
  m_Father: {fileID: 0}
--- !u!50 &102
Rigidbody2D:
  m_GameObject: {fileID: 100}
  m_BodyType: 0
  m_Simulated: 1
  m_Mass: 1
  m_GravityScale: 1
  m_Interpolate: 0
  m_CollisionDetection: 0
--- !u!61 &103
BoxCollider2D:
  m_GameObject: {fileID: 100}
  m_IsTrigger: 0
  m_Offset: {x: 0, y: 0}
  m_Size: {x: 1, y: 1}
--- !u!114 &104
MonoBehaviour:
  m_GameObject: {fileID: 100}
  m_Enabled: 1
  m_Script: {fileID: 11500000, guid: 44004400440044000000000000000003, type: 3}
";

        private const string SimulatedFadeSceneYaml = @"%YAML 1.1
%TAG !u! tag:unity3d.com,2011:
--- !u!1 &100
GameObject:
  m_ObjectHideFlags: 0
  serializedVersion: 6
  m_Component:
  - component: {fileID: 101}
  - component: {fileID: 102}
  - component: {fileID: 103}
  - component: {fileID: 104}
  m_Layer: 0
  m_Name: WickBody2D
  m_IsActive: 1
--- !u!4 &101
Transform:
  m_GameObject: {fileID: 100}
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalPosition: {x: 0, y: 0.75, z: 0}
  m_LocalScale: {x: 1, y: 1, z: 1}
  m_Children: []
  m_Father: {fileID: 0}
--- !u!50 &102
Rigidbody2D:
  m_GameObject: {fileID: 100}
  m_BodyType: 0
  m_Simulated: 1
  m_Mass: 1
  m_GravityScale: 0
  m_Interpolate: 0
  m_CollisionDetection: 0
--- !u!61 &103
BoxCollider2D:
  m_GameObject: {fileID: 100}
  m_IsTrigger: 0
  m_Offset: {x: 0, y: 0}
  m_Size: {x: 1, y: 1}
--- !u!114 &104
MonoBehaviour:
  m_GameObject: {fileID: 100}
  m_Enabled: 1
  m_Script: {fileID: 11500000, guid: 44004400440044000000000000000004, type: 3}
--- !u!1 &200
GameObject:
  m_ObjectHideFlags: 0
  serializedVersion: 6
  m_Component:
  - component: {fileID: 201}
  - component: {fileID: 202}
  m_Layer: 0
  m_Name: HazardTrigger2D
  m_IsActive: 1
--- !u!4 &201
Transform:
  m_GameObject: {fileID: 200}
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalPosition: {x: 0, y: 0.75, z: 0}
  m_LocalScale: {x: 1, y: 1, z: 1}
  m_Children: []
  m_Father: {fileID: 0}
--- !u!61 &202
BoxCollider2D:
  m_GameObject: {fileID: 200}
  m_IsTrigger: 1
  m_Offset: {x: 0, y: 0}
  m_Size: {x: 2, y: 2}
--- !u!1 &300
GameObject:
  m_ObjectHideFlags: 0
  serializedVersion: 6
  m_Component:
  - component: {fileID: 301}
  - component: {fileID: 302}
  m_Layer: 0
  m_Name: Floor2D
  m_IsActive: 1
--- !u!4 &301
Transform:
  m_GameObject: {fileID: 300}
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalPosition: {x: 0, y: 0, z: 0}
  m_LocalScale: {x: 1, y: 1, z: 1}
  m_Children: []
  m_Father: {fileID: 0}
--- !u!61 &302
BoxCollider2D:
  m_GameObject: {fileID: 300}
  m_IsTrigger: 0
  m_Offset: {x: 0, y: 0}
  m_Size: {x: 4, y: 1}
";

        private const string KinematicRiderSceneYaml = @"%YAML 1.1
%TAG !u! tag:unity3d.com,2011:
--- !u!1 &100
GameObject:
  m_ObjectHideFlags: 0
  serializedVersion: 6
  m_Component:
  - component: {fileID: 101}
  - component: {fileID: 102}
  - component: {fileID: 103}
  - component: {fileID: 104}
  m_Layer: 0
  m_Name: KinematicPlatform2D
  m_IsActive: 1
--- !u!4 &101
Transform:
  m_GameObject: {fileID: 100}
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalPosition: {x: 0, y: 0, z: 0}
  m_LocalScale: {x: 1, y: 1, z: 1}
  m_Children: []
  m_Father: {fileID: 0}
--- !u!50 &102
Rigidbody2D:
  m_GameObject: {fileID: 100}
  m_BodyType: 1
  m_Simulated: 1
  m_Mass: 1
  m_GravityScale: 0
  m_Interpolate: 0
  m_CollisionDetection: 0
--- !u!61 &103
BoxCollider2D:
  m_GameObject: {fileID: 100}
  m_IsTrigger: 0
  m_Offset: {x: 0, y: 0}
  m_Size: {x: 4, y: 0.5}
--- !u!114 &104
MonoBehaviour:
  m_GameObject: {fileID: 100}
  m_Enabled: 1
  m_Script: {fileID: 11500000, guid: 44004400440044000000000000000005, type: 3}
--- !u!1 &200
GameObject:
  m_ObjectHideFlags: 0
  serializedVersion: 6
  m_Component:
  - component: {fileID: 201}
  - component: {fileID: 202}
  - component: {fileID: 203}
  - component: {fileID: 204}
  m_Layer: 0
  m_Name: DynamicRider2D
  m_IsActive: 1
--- !u!4 &201
Transform:
  m_GameObject: {fileID: 200}
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalPosition: {x: 0, y: 0.55, z: 0}
  m_LocalScale: {x: 1, y: 1, z: 1}
  m_Children: []
  m_Father: {fileID: 0}
--- !u!50 &202
Rigidbody2D:
  m_GameObject: {fileID: 200}
  m_BodyType: 0
  m_Simulated: 1
  m_Mass: 1
  m_GravityScale: 1
  m_Interpolate: 0
  m_CollisionDetection: 0
--- !u!61 &203
BoxCollider2D:
  m_GameObject: {fileID: 200}
  m_IsTrigger: 0
  m_Offset: {x: 0, y: 0}
  m_Size: {x: 0.5, y: 0.5}
--- !u!114 &204
MonoBehaviour:
  m_GameObject: {fileID: 200}
  m_Enabled: 1
  m_Script: {fileID: 11500000, guid: 44004400440044000000000000000006, type: 3}
";

        private const string WicksLanternEndToEndSceneYaml = @"%YAML 1.1
%TAG !u! tag:unity3d.com,2011:
--- !u!1 &100
GameObject:
  m_ObjectHideFlags: 0
  serializedVersion: 6
  m_Component:
  - component: {fileID: 101}
  - component: {fileID: 102}
  - component: {fileID: 103}
  - component: {fileID: 104}
  m_Layer: 0
  m_Name: WickPlayer2D
  m_IsActive: 1
--- !u!4 &101
Transform:
  m_GameObject: {fileID: 100}
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalPosition: {x: -0.8, y: 0.75, z: 0}
  m_LocalScale: {x: 1, y: 1, z: 1}
  m_Children: []
  m_Father: {fileID: 0}
--- !u!50 &102
Rigidbody2D:
  m_GameObject: {fileID: 100}
  m_BodyType: 0
  m_Simulated: 1
  m_Mass: 1
  m_GravityScale: 1
  m_Interpolate: 1
  m_CollisionDetection: 1
  m_Constraints: 4
--- !u!70 &103
CapsuleCollider2D:
  m_GameObject: {fileID: 100}
  m_IsTrigger: 0
  m_Offset: {x: 0, y: 0}
  m_Size: {x: 0.75, y: 1.375}
  m_Direction: 0
--- !u!114 &104
MonoBehaviour:
  m_GameObject: {fileID: 100}
  m_Enabled: 1
  m_Script: {fileID: 11500000, guid: 44004400440044000000000000000007, type: 3}
--- !u!1 &200
GameObject:
  m_ObjectHideFlags: 0
  serializedVersion: 6
  m_Component:
  - component: {fileID: 201}
  - component: {fileID: 202}
  - component: {fileID: 203}
  - component: {fileID: 204}
  - component: {fileID: 205}
  - component: {fileID: 206}
  m_Layer: 0
  m_Name: CompositeGround2D
  m_IsActive: 1
--- !u!4 &201
Transform:
  m_GameObject: {fileID: 200}
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalPosition: {x: 0, y: -0.5, z: 0}
  m_LocalScale: {x: 1, y: 1, z: 1}
  m_Children:
  - {fileID: 211}
  m_Father: {fileID: 0}
--- !u!50 &202
Rigidbody2D:
  m_GameObject: {fileID: 200}
  m_BodyType: 2
  m_Simulated: 1
--- !u!66 &203
CompositeCollider2D:
  m_GameObject: {fileID: 200}
  m_GeometryType: 0
  m_GenerationType: 0
--- !u!61 &204
BoxCollider2D:
  m_GameObject: {fileID: 200}
  m_UsedByComposite: 1
  m_Offset: {x: -2, y: 0}
  m_Size: {x: 4, y: 1}
--- !u!1839735485 &205
Tilemap:
  m_GameObject: {fileID: 200}
  m_Tiles:
  - m_X: 0
    m_Y: -1
    m_Z: 0
    m_TileIndex: 0
  - m_X: 1
    m_Y: -1
    m_Z: 0
    m_TileIndex: 0
  m_TileAssetArray: []
--- !u!19719996 &206
TilemapCollider2D:
  m_GameObject: {fileID: 200}
  m_UsedByComposite: 1
  m_MaximumTileChangeCount: 1000
  m_ExtrusionFactor: 0
--- !u!1 &210
GameObject:
  m_ObjectHideFlags: 0
  serializedVersion: 6
  m_Component:
  - component: {fileID: 211}
  - component: {fileID: 212}
  m_Layer: 0
  m_Name: GroundRightAbutting2D
  m_IsActive: 1
--- !u!4 &211
Transform:
  m_GameObject: {fileID: 210}
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalPosition: {x: 0, y: 0, z: 0}
  m_LocalScale: {x: 1, y: 1, z: 1}
  m_Children: []
  m_Father: {fileID: 201}
--- !u!61 &212
BoxCollider2D:
  m_GameObject: {fileID: 210}
  m_UsedByComposite: 1
  m_Offset: {x: 2, y: 0}
  m_Size: {x: 4, y: 1}
--- !u!1 &300
GameObject:
  m_ObjectHideFlags: 0
  serializedVersion: 6
  m_Component:
  - component: {fileID: 301}
  - component: {fileID: 302}
  m_Layer: 0
  m_Name: EmberTrigger2D
  m_IsActive: 1
--- !u!4 &301
Transform:
  m_GameObject: {fileID: 300}
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalPosition: {x: 0.4, y: 0.8, z: 0}
  m_LocalScale: {x: 1, y: 1, z: 1}
  m_Children: []
  m_Father: {fileID: 0}
--- !u!58 &302
CircleCollider2D:
  m_GameObject: {fileID: 300}
  m_IsTrigger: 1
  m_Offset: {x: 0, y: 0}
  m_Radius: 0.35
--- !u!1 &400
GameObject:
  m_ObjectHideFlags: 0
  serializedVersion: 6
  m_Component:
  - component: {fileID: 401}
  - component: {fileID: 402}
  - component: {fileID: 403}
  m_Layer: 8
  m_Name: OneWayPlatform2D
  m_IsActive: 1
--- !u!4 &401
Transform:
  m_GameObject: {fileID: 400}
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalPosition: {x: 6, y: 2.0, z: 0}
  m_LocalScale: {x: 1, y: 1, z: 1}
  m_Children: []
  m_Father: {fileID: 0}
--- !u!61 &402
BoxCollider2D:
  m_GameObject: {fileID: 400}
  m_IsTrigger: 0
  m_UsedByEffector: 1
  m_Offset: {x: 0, y: 0}
  m_Size: {x: 3, y: 0.4}
--- !u!251 &403
PlatformEffector2D:
  m_GameObject: {fileID: 400}
  m_UseOneWay: 1
  m_SurfaceArc: 200
  m_RotationalOffset: 0
  m_SideArc: 0
  m_UseSideFriction: 0
  m_UseSideBounce: 0
--- !u!1 &500
GameObject:
  m_ObjectHideFlags: 0
  serializedVersion: 6
  m_Component:
  - component: {fileID: 501}
  - component: {fileID: 502}
  - component: {fileID: 503}
  m_Layer: 0
  m_Name: Conveyor2D
  m_IsActive: 1
--- !u!4 &501
Transform:
  m_GameObject: {fileID: 500}
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalPosition: {x: 12, y: 0, z: 0}
  m_LocalScale: {x: 1, y: 1, z: 1}
  m_Children: []
  m_Father: {fileID: 0}
--- !u!61 &502
BoxCollider2D:
  m_GameObject: {fileID: 500}
  m_IsTrigger: 0
  m_UsedByEffector: 1
  m_Offset: {x: 0, y: 0}
  m_Size: {x: 6, y: 0.5}
--- !u!252 &503
SurfaceEffector2D:
  m_GameObject: {fileID: 500}
  m_Speed: 3.5
  m_ForceScale: 1
  m_UseFriction: 1
  m_UseBounce: 0
--- !u!1 &600
GameObject:
  m_ObjectHideFlags: 0
  serializedVersion: 6
  m_Component:
  - component: {fileID: 601}
  - component: {fileID: 602}
  - component: {fileID: 603}
  - component: {fileID: 604}
  m_Layer: 0
  m_Name: KinematicPlatform2D
  m_IsActive: 1
--- !u!4 &601
Transform:
  m_GameObject: {fileID: 600}
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalPosition: {x: 20, y: 0, z: 0}
  m_LocalScale: {x: 1, y: 1, z: 1}
  m_Children: []
  m_Father: {fileID: 0}
--- !u!50 &602
Rigidbody2D:
  m_GameObject: {fileID: 600}
  m_BodyType: 1
  m_Simulated: 1
  m_Mass: 1
  m_GravityScale: 0
--- !u!61 &603
BoxCollider2D:
  m_GameObject: {fileID: 600}
  m_IsTrigger: 0
  m_Offset: {x: 0, y: 0}
  m_Size: {x: 4, y: 0.5}
--- !u!114 &604
MonoBehaviour:
  m_GameObject: {fileID: 600}
  m_Enabled: 1
  m_Script: {fileID: 11500000, guid: 44004400440044000000000000000005, type: 3}
--- !u!1 &700
GameObject:
  m_ObjectHideFlags: 0
  serializedVersion: 6
  m_Component:
  - component: {fileID: 701}
  - component: {fileID: 702}
  - component: {fileID: 703}
  - component: {fileID: 704}
  m_Layer: 0
  m_Name: PlatformRider2D
  m_IsActive: 1
--- !u!4 &701
Transform:
  m_GameObject: {fileID: 700}
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
  m_LocalPosition: {x: 20, y: 0.55, z: 0}
  m_LocalScale: {x: 1, y: 1, z: 1}
  m_Children: []
  m_Father: {fileID: 0}
--- !u!50 &702
Rigidbody2D:
  m_GameObject: {fileID: 700}
  m_BodyType: 0
  m_Simulated: 1
  m_Mass: 1
  m_GravityScale: 1
--- !u!61 &703
BoxCollider2D:
  m_GameObject: {fileID: 700}
  m_IsTrigger: 0
  m_Offset: {x: 0, y: 0}
  m_Size: {x: 0.5, y: 0.5}
--- !u!114 &704
MonoBehaviour:
  m_GameObject: {fileID: 700}
  m_Enabled: 1
  m_Script: {fileID: 11500000, guid: 44004400440044000000000000000006, type: 3}
";
    }
}
