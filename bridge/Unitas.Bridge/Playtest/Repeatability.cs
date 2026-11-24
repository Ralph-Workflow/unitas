using System;
using System.Collections.Generic;
using System.Globalization;

namespace Unitas.Bridge.Playtest
{
    public sealed partial class ScenarioRunner
    {
        public static bool CompareForRepeatability(ScenarioResult first, ScenarioResult second, out string? difference)
        {
            string? reason = CompareRepeatability(first, second);
            difference = reason;
            bool repeatable = reason == null;
            first.Repeatable = repeatable;
            second.Repeatable = repeatable;
            return repeatable;
        }

        private static string? CompareRepeatability(ScenarioResult first, ScenarioResult second)
        {
            if (first.Status != second.Status)
            {
                return "overall status differs: expected '" + first.Status + "', got '" + second.Status + "'";
            }

            IReadOnlyList<CheckpointResult> firstCheckpoints = first.Checkpoints;
            IReadOnlyList<CheckpointResult> secondCheckpoints = second.Checkpoints;
            if (firstCheckpoints.Count != secondCheckpoints.Count)
            {
                return "checkpoint count differs: expected " + RepeatabilityNumber(firstCheckpoints.Count) + ", got " + RepeatabilityNumber(secondCheckpoints.Count);
            }

            for (int index = 0; index < firstCheckpoints.Count; index++)
            {
                CheckpointResult firstCheckpoint = firstCheckpoints[index];
                CheckpointResult secondCheckpoint = secondCheckpoints[index];
                if (!string.Equals(firstCheckpoint.Id, secondCheckpoint.Id, StringComparison.Ordinal))
                {
                    return "checkpoint " + RepeatabilityNumber(index) + " id differs: expected " + RepeatabilityText(firstCheckpoint.Id) + ", got " + RepeatabilityText(secondCheckpoint.Id);
                }
                if (firstCheckpoint.Status != secondCheckpoint.Status)
                {
                    return "checkpoint '" + firstCheckpoint.Id + "' status differs: expected '" + firstCheckpoint.Status + "', got '" + secondCheckpoint.Status + "'";
                }
                if (!string.Equals(firstCheckpoint.Observed, secondCheckpoint.Observed, StringComparison.Ordinal))
                {
                    return "checkpoint '" + firstCheckpoint.Id + "' observed differs: expected " + RepeatabilityText(firstCheckpoint.Observed) + ", got " + RepeatabilityText(secondCheckpoint.Observed);
                }
            }

            IReadOnlyList<ScenarioStepRecord> firstSteps = first.Steps;
            IReadOnlyList<ScenarioStepRecord> secondSteps = second.Steps;
            if (firstSteps.Count != secondSteps.Count)
            {
                return "step count differs: expected " + RepeatabilityNumber(firstSteps.Count) + ", got " + RepeatabilityNumber(secondSteps.Count);
            }

            for (int index = 0; index < firstSteps.Count; index++)
            {
                ScenarioStepRecord firstStep = firstSteps[index];
                ScenarioStepRecord secondStep = secondSteps[index];
                if (firstStep.Index != secondStep.Index)
                {
                    return "step " + RepeatabilityNumber(index) + " index differs: expected " + RepeatabilityNumber(firstStep.Index) + ", got " + RepeatabilityNumber(secondStep.Index);
                }
                if (!string.Equals(firstStep.Kind, secondStep.Kind, StringComparison.Ordinal))
                {
                    return "step " + RepeatabilityNumber(index) + " kind differs: expected " + RepeatabilityText(firstStep.Kind) + ", got " + RepeatabilityText(secondStep.Kind);
                }
                if (firstStep.Frames != secondStep.Frames)
                {
                    return "step " + RepeatabilityNumber(index) + " frames differs: expected " + RepeatabilityNumber(firstStep.Frames) + ", got " + RepeatabilityNumber(secondStep.Frames);
                }
                if (firstStep.FrameStart != secondStep.FrameStart)
                {
                    return "step " + RepeatabilityNumber(index) + " frame_start differs: expected " + RepeatabilityNumber(firstStep.FrameStart) + ", got " + RepeatabilityNumber(secondStep.FrameStart);
                }
                if (firstStep.FrameEnd != secondStep.FrameEnd)
                {
                    return "step " + RepeatabilityNumber(index) + " frame_end differs: expected " + RepeatabilityNumber(firstStep.FrameEnd) + ", got " + RepeatabilityNumber(secondStep.FrameEnd);
                }
            }

            return null;
        }

        private static string RepeatabilityText(string? value)
        {
            return value == null ? "<null>" : "'" + value + "'";
        }

        private static string RepeatabilityNumber(int? value)
        {
            return value.HasValue ? value.Value.ToString(CultureInfo.InvariantCulture) : "<null>";
        }
    }
}
