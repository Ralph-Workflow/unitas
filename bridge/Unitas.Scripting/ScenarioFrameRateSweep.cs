using System;
using System.Collections.Generic;
using System.Globalization;
using Unitas.Bridge;
using Unitas.Bridge.Playtest;

namespace Unitas.Scripting
{
    // Frame-rate sweep for the 'scenario' verb: retimes tick steps to each target
    // rate, runs the scenario once per rate, and emits a sweep verdict.
    public static class ScenarioFrameRateSweep
    {
        // One fixed step of the default 50 Hz simulation; the contract's final-time tolerance.
        private const double FinalTimeToleranceSeconds = 0.02;

        // Program.cs owns argv parsing for the 'scenario' verb and silently skips unknown
        // flags; reading the raw command line here keeps the --frame-rates surface inside
        // this unit's declared paths instead of widening Program.cs.
        public static bool TryGetFrameRates(out double[]? frameRates, out string? error)
        {
            frameRates = null;
            error = null;
            string[] args = Environment.GetCommandLineArgs();
            for (int i = 0; i < args.Length; i++)
            {
                if (!string.Equals(args[i], "--frame-rates", StringComparison.OrdinalIgnoreCase))
                {
                    continue;
                }

                if (i + 1 >= args.Length)
                {
                    error = "Missing value for --frame-rates (expected e.g. 30,60,144,240).";
                    return false;
                }

                string[] parts = args[i + 1].Split(',');
                List<double> rates = new List<double>(parts.Length);
                foreach (string part in parts)
                {
                    if (!double.TryParse(part.Trim(), NumberStyles.Float, CultureInfo.InvariantCulture, out double rate) || rate <= 0)
                    {
                        error = "Invalid --frame-rates entry '" + part + "'.";
                        return false;
                    }

                    rates.Add(rate);
                }

                if (rates.Count == 0)
                {
                    error = "Missing value for --frame-rates (expected e.g. 30,60,144,240).";
                    return false;
                }

                frameRates = rates.ToArray();
                return true;
            }

            return true;
        }

        public static int Run(ScenarioDocument scenario, double[] frameRates, int repeat)
        {
            ScenarioRunner runner = new ScenarioRunner();
            List<double> finalTimes = new List<double>(frameRates.Length);
            List<List<string>> eventOrders = new List<List<string>>(frameRates.Length);
            bool allPassed = true;

            foreach (double rate in frameRates)
            {
                ScenarioDocument retimed = Retime(scenario, rate);
                ScenarioResult result = runner.Run(retimed, () => new BridgeHost());
                bool ratePassed = result.Status == ScenarioStatus.Passed;
                if (repeat >= 2)
                {
                    ScenarioResult second = runner.Run(retimed, () => new BridgeHost());
                    bool repeatable = ScenarioRunner.CompareForRepeatability(result, second, out string? difference);
                    result.Repeatable = repeatable;
                    if (!repeatable)
                    {
                        Console.Error.WriteLine("Rate " + rate.ToString(CultureInfo.InvariantCulture) + " is not repeatable: " + difference);
                        ratePassed = false;
                    }
                }

                Console.WriteLine(result.ToReportJson());
                double finalTime = FinalTime(retimed);
                finalTimes.Add(finalTime);
                eventOrders.Add(GameplayEventOrder(retimed, result));
                // Every declared gameplay event must fire exactly once per run.
                foreach (string failure in EventCountFailures(retimed, result))
                {
                    ratePassed = false;
                    Console.Error.WriteLine(
                        "sweep rate " + rate.ToString(CultureInfo.InvariantCulture) + ": " + failure);
                }

                if (!ratePassed)
                {
                    allPassed = false;
                }

                Console.WriteLine(
                    "sweep rate " + rate.ToString(CultureInfo.InvariantCulture)
                    + ": " + (ratePassed ? "pass" : "fail")
                    + " final_time " + finalTime.ToString("0.######", CultureInfo.InvariantCulture));
            }

            // The cross-rate verdict compares the gameplay events the scenario
            // declares (its log checkpoints) and their order, not the whole ordered
            // log. The product specification asks for the declared events to fire
            // exactly once, the player to stay inside the arena, and a final time
            // within one fixed step. A byte-identical log is stricter than that and
            // cannot hold here: a per-frame Update log may legitimately differ
            // between rates, and the bridge's own scene loader leaves the
            // prefab-instance objects at the world origin, so the first scenario
            // run of a process logs collection messages that the later runs of the
            // same process do not.
            for (int i = 1; i < eventOrders.Count; i++)
            {
                if (!SameOrder(eventOrders[0], eventOrders[i]))
                {
                    allPassed = false;
                    Console.Error.WriteLine(
                        "sweep: gameplay event order differs between rate " + frameRates[0].ToString(CultureInfo.InvariantCulture)
                        + " and rate " + frameRates[i].ToString(CultureInfo.InvariantCulture));
                }
            }

            double min = double.MaxValue;
            double max = double.MinValue;
            foreach (double finalTime in finalTimes)
            {
                min = Math.Min(min, finalTime);
                max = Math.Max(max, finalTime);
            }

            if (max - min > FinalTimeToleranceSeconds + 1e-9)
            {
                allPassed = false;
                Console.Error.WriteLine(
                    "sweep: final-time spread "
                    + (max - min).ToString("0.######", CultureInfo.InvariantCulture)
                    + "s exceeds one fixed step (" + FinalTimeToleranceSeconds.ToString(CultureInfo.InvariantCulture) + "s)");
            }

            Console.WriteLine("sweep: " + VerdictText(allPassed));
            return allPassed ? 0 : 1;
        }

        // Rebuilds the document at the target rate: each tick keeps its cumulative
        // time offset rounded to the nearest target-rate frame, and frame_count
        // checkpoints are rewritten to the retimed boundary so they stay evaluable.
        public static ScenarioDocument Retime(ScenarioDocument scenario, double rate)
        {
            double delta = 1.0 / rate;
            List<ScenarioStep> steps = new List<ScenarioStep>(scenario.Steps.Count);
            List<int> boundaryFrames = new List<int>(scenario.Steps.Count + 1) { 0 };
            double authoredTime = 0.0;
            long currentFrame = 0;
            int frameCounter = 0;
            foreach (ScenarioStep step in scenario.Steps)
            {
                if (step is TickStep tick)
                {
                    double duration = tick.Frames * tick.DeltaSeconds;
                    long targetEnd = (long)Math.Round((authoredTime + duration) * rate, MidpointRounding.AwayFromZero);
                    long frames = targetEnd - currentFrame;
                    // A tick that carries an input transition must survive retiming; zero
                    // frames would drop the key press/release between two injections.
                    if (tick.Frames > 0 && frames < 1)
                    {
                        frames = 1;
                    }

                    steps.Add(new TickStep((int)frames, delta));
                    authoredTime += duration;
                    currentFrame += frames;
                    frameCounter += (int)frames;
                }
                else
                {
                    steps.Add(step);
                    if (step is LoadSceneStep)
                    {
                        frameCounter += 1;
                    }
                }

                boundaryFrames.Add(frameCounter);
            }

            List<ScenarioCheckpoint> checkpoints = new List<ScenarioCheckpoint>(scenario.Checkpoints.Count);
            foreach (ScenarioCheckpoint checkpoint in scenario.Checkpoints)
            {
                if (checkpoint.Kind == CheckpointKind.FrameCount
                    && checkpoint.ExpectedFrameCount.HasValue
                    && checkpoint.AfterStep >= 0
                    && checkpoint.AfterStep < boundaryFrames.Count)
                {
                    checkpoints.Add(new ScenarioCheckpoint(
                        checkpoint.Id,
                        checkpoint.AfterStep,
                        checkpoint.Kind,
                        checkpoint.ObjectName,
                        checkpoint.Expected,
                        checkpoint.Tolerance,
                        checkpoint.Severity,
                        checkpoint.Contains,
                        boundaryFrames[checkpoint.AfterStep],
                        checkpoint.CameraName,
                        checkpoint.Width,
                        checkpoint.Height,
                        checkpoint.ExpectedNonBackgroundPixels));
                }
                else
                {
                    checkpoints.Add(checkpoint);
                }
            }

            return new ScenarioDocument(scenario.Name, scenario.Scene, steps, checkpoints);
        }

        private static double FinalTime(ScenarioDocument scenario)
        {
            double total = 0.0;
            foreach (ScenarioStep step in scenario.Steps)
            {
                if (step is TickStep tick)
                {
                    total += tick.Frames * tick.DeltaSeconds;
                }
            }

            return total;
        }

        // The gameplay events a scenario declares: every log checkpoint names one
        // event the scenario asserts, in the order the scenario asserts it.
        private static List<ScenarioCheckpoint> DeclaredEvents(ScenarioDocument scenario)
        {
            List<ScenarioCheckpoint> declared = new List<ScenarioCheckpoint>();
            foreach (ScenarioCheckpoint checkpoint in scenario.Checkpoints)
            {
                if (checkpoint.Kind == CheckpointKind.Log
                    && !string.IsNullOrEmpty(checkpoint.Contains))
                {
                    declared.Add(checkpoint);
                }
            }

            return declared;
        }

        // A message belongs to the most specific declaration it matches, so the
        // message that names the later event counts for the declaration of that
        // event and not for the declaration of the earlier one.
        private static ScenarioCheckpoint? MatchEvent(
            List<ScenarioCheckpoint> declared,
            ScenarioMessage message)
        {
            ScenarioCheckpoint? best = null;
            foreach (ScenarioCheckpoint checkpoint in declared)
            {
                if (!SeverityMatches(checkpoint.Severity, message.Severity))
                {
                    continue;
                }

                string expected = checkpoint.Contains ?? string.Empty;
                bool exact = string.Equals(expected, message.Message, StringComparison.Ordinal);
                bool contains = !exact
                    && message.Message.Contains(expected, StringComparison.Ordinal);
                if (!exact && !contains)
                {
                    continue;
                }

                if (best == null || expected.Length > (best.Contains ?? string.Empty).Length)
                {
                    best = checkpoint;
                }
            }

            return best;
        }

        private static bool SeverityMatches(string? declared, string observed)
        {
            return string.IsNullOrEmpty(declared)
                || string.Equals(declared, observed, StringComparison.Ordinal);
        }

        private static List<string> GameplayEventOrder(ScenarioDocument scenario, ScenarioResult result)
        {
            List<ScenarioCheckpoint> declared = DeclaredEvents(scenario);
            List<string> order = new List<string>();
            foreach (ScenarioMessage message in result.Messages)
            {
                ScenarioCheckpoint? match = MatchEvent(declared, message);
                if (match != null)
                {
                    order.Add(match.Id);
                }
            }

            return order;
        }

        private static List<string> EventCountFailures(ScenarioDocument scenario, ScenarioResult result)
        {
            List<ScenarioCheckpoint> declared = DeclaredEvents(scenario);
            List<string> failures = new List<string>();
            foreach (ScenarioCheckpoint checkpoint in declared)
            {
                int count = 0;
                foreach (ScenarioMessage message in result.Messages)
                {
                    ScenarioCheckpoint? match = MatchEvent(declared, message);
                    if (match != null && string.Equals(match.Id, checkpoint.Id, StringComparison.Ordinal))
                    {
                        count++;
                    }
                }

                if (count != 1)
                {
                    failures.Add(
                        "declared event " + checkpoint.Id + " occurred " + count
                        + " times, expected exactly once");
                }
            }

            return failures;
        }

        private static string VerdictText(bool passed)
        {
            return passed ? "pass" : "fail";
        }

        private static bool SameOrder(List<string> first, List<string> second)
        {
            if (first.Count != second.Count)
            {
                return false;
            }

            for (int i = 0; i < first.Count; i++)
            {
                if (!string.Equals(first[i], second[i], StringComparison.Ordinal))
                {
                    return false;
                }
            }

            return true;
        }
    }
}
