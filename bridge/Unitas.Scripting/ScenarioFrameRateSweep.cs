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
            List<List<string>> logOrders = new List<List<string>>(frameRates.Length);
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
                logOrders.Add(LogOrder(result));
                if (!ratePassed)
                {
                    allPassed = false;
                }

                Console.WriteLine(
                    "sweep rate " + rate.ToString(CultureInfo.InvariantCulture)
                    + ": " + (ratePassed ? "pass" : "fail")
                    + " final_time " + finalTime.ToString("0.######", CultureInfo.InvariantCulture));
            }

            for (int i = 1; i < logOrders.Count; i++)
            {
                if (!SameOrder(logOrders[0], logOrders[i]))
                {
                    allPassed = false;
                    Console.Error.WriteLine(
                        "sweep: log order differs between rate " + frameRates[0].ToString(CultureInfo.InvariantCulture)
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

        private static List<string> LogOrder(ScenarioResult result)
        {
            List<string> order = new List<string>(result.Messages.Count);
            foreach (ScenarioMessage message in result.Messages)
            {
                order.Add(message.Severity + "|" + message.Message);
            }

            return order;
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
