package format

import (
	"fmt"
	"strings"
	"text/tabwriter"
	"time"

	proto "github.com/clintjedwards/gofer/proto/go"

	"github.com/dustin/go-humanize"
	"github.com/fatih/color"
	"golang.org/x/text/cases"
	"golang.org/x/text/language"
)

// UnixMilli returns a humanized version of time given in unix millisecond. The zeroMsg is the string returned when
// the time is 0 and assumed to be not set.
func UnixMilli(unix int64, zeroMsg string, detail bool) string {
	if unix == 0 {
		return zeroMsg
	}

	if !detail {
		return humanize.Time(time.UnixMilli(unix))
	}

	relativeTime := humanize.Time(time.UnixMilli(unix))
	realTime := time.UnixMilli(unix).Format(time.RFC850)

	return fmt.Sprintf("%s (%s)", realTime, relativeTime)
}

// Takes a string enum and turns them into title case. If the value is unknown we turn it into
// a string of your choosing.
func NormalizeEnumValue[s ~string](value s, unknownString string) string {
	toTitle := cases.Title(language.AmericanEnglish)
	toLower := cases.Lower(language.AmericanEnglish)
	state := toTitle.String(toLower.String(string(value)))

	if strings.Contains(strings.ToLower(state), "unknown") {
		return unknownString
	}

	return state
}

func GenerateGenericTable(data [][]string, sep string, indent int) string {
	tableString := &strings.Builder{}
	table := tabwriter.NewWriter(tableString, 0, 2, 1, ' ', tabwriter.TabIndent)

	for _, item := range data {
		fmttedRow := ""

		for i := 1; i < indent; i++ {
			fmttedRow += " "
		}

		fmttedRow += strings.Join(item, fmt.Sprintf("\t%s ", sep))
		fmt.Fprintln(table, fmttedRow)
	}
	table.Flush()
	return tableString.String()
}

// Duration returns a humanized duration time for two epoch milli second times.
func Duration(start, end int64) string {
	if start == 0 {
		return "0s"
	}

	startTime := time.UnixMilli(start)
	endTime := time.Now()

	if end != 0 {
		endTime = time.UnixMilli(end)
	}

	duration := endTime.Sub(startTime)

	if duration > time.Second {
		truncate := time.Second
		return "~" + duration.Truncate(truncate).String()
	}

	return "~" + duration.String()
}

func ColorizePipelineMetadataState(state string) string {
	switch strings.ToUpper(state) {
	case proto.PipelineMetadata_PIPELINE_STATE_UNKNOWN.String():
		return color.YellowString(state)
	case proto.PipelineMetadata_ACTIVE.String():
		return color.GreenString(state)
	case proto.PipelineMetadata_DISABLED.String():
		return color.YellowString(state)
	default:
		return state
	}
}

func ColorizePipelineConfigState(state string) string {
	switch strings.ToUpper(state) {
	case proto.PipelineConfig_PIPELINE_CONFIG_STATE_UNKNOWN.String():
		return color.RedString(state)
	case proto.PipelineConfig_LIVE.String():
		return color.GreenString(state)
	case proto.PipelineConfig_UNRELEASED.String():
		return color.YellowString(state)
	case proto.PipelineConfig_DEPRECATED.String():
		return state
	default:
		return state
	}
}

func ColorizeRunState(state string) string {
	switch strings.ToUpper(state) {
	case proto.Run_RUN_STATE_UNKNOWN.String():
		return color.YellowString(state)
	case proto.Run_PENDING.String():
		return color.YellowString(state)
	case proto.Run_RUNNING.String():
		return color.YellowString(state)
	case proto.Run_COMPLETE.String():
		return color.GreenString(state)
	default:
		return state
	}
}

func ColorizeRunStatus(status string) string {
	switch strings.ToUpper(status) {
	case proto.Run_RUN_STATUS_UNKNOWN.String():
		return color.YellowString(status)
	case proto.Run_SUCCESSFUL.String():
		return color.GreenString(status)
	case proto.Run_FAILED.String():
		return color.RedString(status)
	case proto.Run_CANCELLED.String():
		return status
	default:
		return status
	}
}

func ColorizeTaskRunState(state string) string {
	switch strings.ToUpper(state) {
	case proto.TaskRun_UNKNOWN_STATE.String():
		return color.YellowString(state)
	case proto.TaskRun_PROCESSING.String():
		return color.YellowString(state)
	case proto.TaskRun_WAITING.String():
		return color.YellowString(state)
	case proto.TaskRun_RUNNING.String():
		return color.YellowString(state)
	case proto.TaskRun_COMPLETE.String():
		return color.GreenString(state)
	default:
		return state
	}
}

func ColorizeTaskRunStatus(status string) string {
	switch strings.ToUpper(status) {
	case proto.TaskRun_UNKNOWN_STATUS.String():
		return color.YellowString(status)
	case proto.TaskRun_SUCCESSFUL.String():
		return color.GreenString(status)
	case proto.TaskRun_FAILED.String():
		return color.RedString(status)
	case proto.TaskRun_CANCELLED.String():
		return status
	case proto.TaskRun_SKIPPED.String():
		return status
	default:
		return status
	}
}

func ColorizeExtensionState(state string) string {
	switch strings.ToUpper(state) {
	case proto.Extension_UNKNOWN_STATE.String():
		return color.YellowString(state)
	case proto.Extension_PROCESSING.String():
		return color.YellowString(state)
	case proto.Extension_RUNNING.String():
		return color.GreenString(state)
	case proto.Extension_EXITED.String():
		return color.RedString(state)
	default:
		return state
	}
}

func ColorizeExtensionStatus(status string) string {
	switch strings.ToUpper(status) {
	case proto.Extension_UNKNOWN_STATUS.String():
		return color.YellowString(status)
	case proto.Extension_ENABLED.String():
		return color.GreenString(status)
	case proto.Extension_DISABLED.String():
		return color.YellowString(status)
	default:
		return status
	}
}

func ColorizePipelineExtensionSubscriptionStatus(status string) string {
	switch strings.ToUpper(status) {
	case proto.PipelineExtensionSubscription_STATUS_UNKNOWN.String():
		return color.YellowString(status)
	case proto.PipelineExtensionSubscription_ACTIVE.String():
		return color.GreenString(status)
	case proto.PipelineExtensionSubscription_DISABLED.String():
		return color.YellowString(status)
	case proto.PipelineExtensionSubscription_ERROR.String():
		return color.RedString(status)
	default:
		return status
	}
}

func SliceJoin(slice []string, msg string) string {
	if len(slice) == 0 {
		return msg
	}

	return strings.Join(slice, ", ")
}

func Health(states []proto.Run_RunStatus, emoji bool) string {
	failed := 0
	passed := 0
	for _, state := range states {
		switch state {
		case proto.Run_FAILED:
			failed++
		case proto.Run_RUN_STATUS_UNKNOWN:
			failed++
		default:
			passed++
		}
	}

	healthString := ""

	if failed > 0 && passed == 0 {
		if emoji {
			healthString = "☔︎ "
		}
		return color.RedString(healthString + "Poor")
	}

	if failed > 0 && passed > 0 {
		if emoji {
			healthString = "☁︎ "
		}
		return color.YellowString(healthString + "Unstable")
	}

	if emoji {
		healthString = "☀︎ "
	}

	return color.GreenString(healthString + "Good")
}

func Dependencies(dependencies map[string]proto.Task_RequiredParentStatus) []string {
	result := []string{}
	any := []string{}
	successful := []string{}
	failure := []string{}

	for name, state := range dependencies {
		switch state.String() {
		case proto.Task_ANY.String():
			any = append(any, name)
		case proto.Task_SUCCESS.String():
			successful = append(successful, name)
		case proto.Task_FAILURE.String():
			failure = append(failure, name)
		case proto.Task_REQUIRED_PARENT_STATUS_UNKNOWN.String():
		}
	}

	if len(any) > 0 {
		if len(any) == 1 {
			result = append(result, fmt.Sprintf("After task %s has finished.", strings.Join(any, ", ")))
		} else {
			result = append(result, fmt.Sprintf("After tasks %s have finished.", strings.Join(any, ", ")))
		}
	}
	if len(successful) > 0 {
		if len(successful) == 1 {
			result = append(result, fmt.Sprintf("Only after task %s has finished successfully.", strings.Join(successful, ", ")))
		} else {
			result = append(result, fmt.Sprintf("Only after tasks %s have finished successfully.", strings.Join(successful, ", ")))
		}
	}
	if len(failure) > 0 {
		if len(failure) == 1 {
			result = append(result, fmt.Sprintf("Only after task %s has finished with an error.", strings.Join(failure, ", ")))
		} else {
			result = append(result, fmt.Sprintf("Only after tasks %s have finished with an error.", strings.Join(failure, ", ")))
		}
	}

	return result
}
