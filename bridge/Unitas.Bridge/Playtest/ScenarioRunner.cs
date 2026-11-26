using System;
using System.Collections.Generic;
using System.IO;
using System.Text;
using UnityEngine;

namespace Unitas.Bridge.Playtest
{
    public sealed partial class ScenarioRunner
    {
        private const string CaptureFolderName = "unitas-playtest";

        public ScenarioResult Run(ScenarioDocument scenario, Func<BridgeHost>? hostFactory = null)
        {
            ArgumentNullException.ThrowIfNull(scenario);
            Func<BridgeHost> factory = hostFactory ?? (() => new BridgeHost());
            List<ScenarioStepRecord> stepRecords = new List<ScenarioStepRecord>();
            List<ScenarioMessage> messages = new List<ScenarioMessage>();
            List<string> captures = new List<string>();
            CheckpointResult?[] checkpointResults = new CheckpointResult?[scenario.Checkpoints.Count];
            LogRecorder.Clear();
            // Each scenario run starts on a fresh clock so repeated runs observe identical values.
            Time.Reset();
            BridgeHost host = factory();
            try
            {
                string? setupScene = null;
                string? setupError = null;
                if (!string.IsNullOrEmpty(scenario.Scene))
                {
                    setupScene = scenario.Scene;
                    try
                    {
                        host.LoadScene(scenario.Scene);
                        if (host.SceneCount == 0)
                        {
                            string? failure = host.GetErrorMessage();
                            setupError = string.IsNullOrEmpty(failure)
                                ? "scene was not loaded: " + scenario.Scene
                                : failure;
                        }
                    }
                    catch (Exception ex)
                    {
                        setupError = ex.Message;
                    }
                }

                if (setupError != null)
                {
                    AppendRecordedMessages(messages);
                    return new ScenarioResult(scenario.Name, ScenarioStatus.FailedSetup, false, setupScene, setupError, stepRecords, new List<CheckpointResult>(), messages, captures, null);
                }

                int frameCounter = 0;
                int executedSteps = 0;
                EvaluateCheckpoints(scenario, checkpointResults, executedSteps, frameCounter, captures);
                foreach (ScenarioStep step in scenario.Steps)
                {
                    int frameStart = frameCounter;
                    int? frames;
                    try
                    {
                        frames = ExecuteStep(host, step, ref frameCounter);
                    }
                    catch (Exception ex)
                    {
                        messages.Add(new ScenarioMessage("error", frameStart, ex.Message));
                        break;
                    }

                    stepRecords.Add(new ScenarioStepRecord(executedSteps, StepKindName(step.Kind), frames, frameStart, frameCounter));
                    executedSteps++;
                    EvaluateCheckpoints(scenario, checkpointResults, executedSteps, frameCounter, captures);
                }

                for (int index = 0; index < checkpointResults.Length; index++)
                {
                    if (checkpointResults[index] == null)
                    {
                        checkpointResults[index] = new CheckpointResult(scenario.Checkpoints[index].Id, CheckpointStatus.Unevaluable, null, null, "checkpoint boundary not reached");
                    }
                }

                List<CheckpointResult> checkpoints = Compact(checkpointResults);
                bool passed = true;
                foreach (CheckpointResult checkpoint in checkpoints)
                {
                    if (checkpoint.Status != CheckpointStatus.Passed)
                    {
                        passed = false;
                        break;
                    }
                }

                AppendRecordedMessages(messages);
                ScenarioStatus status = passed ? ScenarioStatus.Passed : ScenarioStatus.Failed;
                return new ScenarioResult(scenario.Name, status, true, setupScene, null, stepRecords, checkpoints, messages, captures, null);
            }
            finally
            {
                host.Dispose();
            }
        }

        private static int? ExecuteStep(BridgeHost host, ScenarioStep step, ref int frameCounter)
        {
            if (step is LoadSceneStep loadSceneStep)
            {
                host.LoadScene(loadSceneStep.Path, loadSceneStep.Mode);
                frameCounter += 1;
                return null;
            }

            if (step is TickStep tickStep)
            {
                for (int frame = 0; frame < tickStep.Frames; frame++)
                {
                    host.Tick(tickStep.DeltaSeconds);
                }

                frameCounter += tickStep.Frames;
                return tickStep.Frames;
            }

            if (step is InjectKeyStep injectKeyStep)
            {
                if (!Enum.TryParse(injectKeyStep.Key, true, out KeyCode key))
                {
                    throw new InvalidOperationException("unknown key code: " + injectKeyStep.Key);
                }

                host.InjectKey(key, injectKeyStep.Pressed);
                return null;
            }

            if (step is InjectMouseButtonStep mouseButtonStep)
            {
                host.InjectMouseButton(mouseButtonStep.Button, mouseButtonStep.Pressed);
                return null;
            }

            if (step is InjectPointerStep pointerStep)
            {
                host.InjectPointerPosition(pointerStep.X, pointerStep.Y);
                return null;
            }

            if (step is InjectScrollStep scrollStep)
            {
                host.InjectScroll(scrollStep.X, scrollStep.Y);
                return null;
            }

            if (step is SetTimeScaleStep timeScaleStep)
            {
                UnityEngine.Time.timeScale = (float)timeScaleStep.Value;
                return null;
            }

            if (step is InjectGamepadAxisStep gamepadAxisStep)
            {
                host.InjectAxis(gamepadAxisStep.Axis, gamepadAxisStep.Value);
                return null;
            }

            throw new InvalidOperationException("unsupported scenario step: " + step.Kind);
        }

        private static void EvaluateCheckpoints(ScenarioDocument scenario, CheckpointResult?[] results, int executedSteps, int frameCounter, List<string> captures)
        {
            for (int index = 0; index < scenario.Checkpoints.Count; index++)
            {
                if (results[index] != null)
                {
                    continue;
                }

                ScenarioCheckpoint checkpoint = scenario.Checkpoints[index];
                if (checkpoint.AfterStep != executedSteps)
                {
                    continue;
                }

                results[index] = EvaluateCheckpoint(scenario.Name, checkpoint, frameCounter, captures);
            }
        }

        private static CheckpointResult EvaluateCheckpoint(string scenarioName, ScenarioCheckpoint checkpoint, int frameCounter, List<string> captures)
        {
            try
            {
                switch (checkpoint.Kind)
                {
                    case CheckpointKind.TransformPosition:
                        return CheckpointEvaluators.EvaluateTransformPosition(checkpoint);
                    case CheckpointKind.Log:
                        return CheckpointEvaluators.EvaluateLog(checkpoint);
                    case CheckpointKind.FrameCount:
                        return CheckpointEvaluators.EvaluateFrameCount(checkpoint, frameCounter);
                    case CheckpointKind.RenderCapture:
                        return EvaluateRenderCapture(scenarioName, checkpoint, captures);
                    default:
                        return new CheckpointResult(checkpoint.Id, CheckpointStatus.Unevaluable, null, null, "unsupported checkpoint kind: " + checkpoint.Kind);
                }
            }
            catch (Exception ex)
            {
                return new CheckpointResult(checkpoint.Id, CheckpointStatus.Unevaluable, null, null, "checkpoint evaluation failed: " + ex.Message);
            }
        }

        private static CheckpointResult EvaluateRenderCapture(string scenarioName, ScenarioCheckpoint checkpoint, List<string> captures)
        {
            CheckpointResult result = CheckpointEvaluators.EvaluateRenderCapture(checkpoint, CapturePath(scenarioName, checkpoint.Id), out string? written);
            if (written != null)
            {
                captures.Add(written);
            }

            return result;
        }

        private static string CapturePath(string scenarioName, string checkpointId)
        {
            return Path.Combine(Path.GetTempPath(), CaptureFolderName, SanitizeFileName(scenarioName) + "-" + SanitizeFileName(checkpointId) + ".bmp");
        }

        private static string SanitizeFileName(string value)
        {
            char[] invalid = Path.GetInvalidFileNameChars();
            StringBuilder builder = new StringBuilder(value.Length);
            foreach (char character in value)
            {
                builder.Append(Array.IndexOf(invalid, character) >= 0 ? '_' : character);
            }

            return builder.ToString();
        }

        private static List<CheckpointResult> Compact(CheckpointResult?[] results)
        {
            List<CheckpointResult> checkpoints = new List<CheckpointResult>();
            foreach (CheckpointResult? result in results)
            {
                if (result != null)
                {
                    checkpoints.Add(result);
                }
            }

            return checkpoints;
        }

        private static void AppendRecordedMessages(List<ScenarioMessage> messages)
        {
            foreach (LogEntry entry in LogRecorder.Entries)
            {
                messages.Add(new ScenarioMessage(SeverityName(entry.Severity), entry.Frame, entry.Message));
            }
        }

        private static string SeverityName(LogSeverity severity)
        {
            switch (severity)
            {
                case LogSeverity.Warning:
                    return "warning";
                case LogSeverity.Error:
                    return "error";
                default:
                    return "log";
            }
        }

        private static string StepKindName(ScenarioStepKind kind)
        {
            switch (kind)
            {
                case ScenarioStepKind.LoadScene:
                    return "load_scene";
                case ScenarioStepKind.Tick:
                    return "tick";
                case ScenarioStepKind.InjectKey:
                    return "inject_key";
                case ScenarioStepKind.InjectMouseButton:
                    return "inject_mouse_button";
                case ScenarioStepKind.InjectPointer:
                    return "inject_pointer";
                case ScenarioStepKind.InjectScroll:
                    return "inject_scroll";
                case ScenarioStepKind.SetTimeScale:
                    return "set_time_scale";
                case ScenarioStepKind.InjectGamepadAxis:
                    return "inject_gamepad_axis";
                default:
                    return kind.ToString();
            }
        }
    }
}
