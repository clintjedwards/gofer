package format

import (
	"fmt"
	"strings"
	"time"

	"github.com/clintjedwards/gofer/internal/models"
	"github.com/clintjedwards/gofer/proto"
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

func PipelineState(state string) string {
	if state == string(proto.Pipeline_UNKNOWN) {
		return "Never Run"
	}

	// Because of how colorizing a string works we need to
	// do the manipulations on case first or else it will not work.
	toTitle := cases.Title(language.AmericanEnglish)
	toLower := cases.Lower(language.AmericanEnglish)
	state = toTitle.String(toLower.String(state))

	return colorizePipelineState(state)
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

	truncate := 1 * time.Second

	return "~" + duration.Truncate(truncate).String()
}

func colorizePipelineState(state string) string {
	switch strings.ToUpper(state) {
	case proto.Pipeline_UNKNOWN.String():
		return color.RedString(state)
	case proto.Pipeline_ACTIVE.String():
		return color.GreenString(state)
	case proto.Pipeline_DISABLED.String():
		return color.YellowString(state)
	default:
		return state
	}
}

func RunState(state string) string {
	if state == string(proto.Run_UNKNOWN) {
		return "Not Run"
	}

	// Because of how colorizing a string works we need to
	// do the manipulations on case first or else it will not work.
	toTitle := cases.Title(language.AmericanEnglish)
	toLower := cases.Lower(language.AmericanEnglish)
	state = toTitle.String(toLower.String(state))
	return colorizeRunState(state)
}

func colorizeRunState(state string) string {
	switch strings.ToUpper(state) {
	case proto.Run_UNKNOWN.String():
		return color.RedString(state)
	case proto.Run_PROCESSING.String():
		return color.YellowString(state)
	case proto.Run_RUNNING.String():
		return color.YellowString(state)
	case proto.Run_FAILED.String():
		return color.RedString(state)
	case proto.Run_SUCCESS.String():
		return color.GreenString(state)
	case proto.Run_WAITING.String():
		return color.BlueString(state)
	default:
		return state
	}
}

func TaskRunState(state string) string {
	if state == string(proto.TaskRun_UNKNOWN) {
		return "Not Run"
	}

	// Because of how colorizing a string works we need to
	// do the manipulations on case first or else it will not work.
	toTitle := cases.Title(language.AmericanEnglish)
	toLower := cases.Lower(language.AmericanEnglish)
	state = toTitle.String(toLower.String(state))
	return colorizeTaskRunState(state)
}

func colorizeTaskRunState(state string) string {
	switch strings.ToUpper(state) {
	case proto.TaskRun_UNKNOWN.String():
		return color.RedString(state)
	case proto.TaskRun_FAILED.String():
		return color.RedString(state)
	case proto.TaskRun_PROCESSING.String():
		return color.YellowString(state)
	case proto.TaskRun_RUNNING.String():
		return color.YellowString(state)
	case proto.TaskRun_SUCCESS.String():
		return color.GreenString(state)
	default:
		return state
	}
}

func TriggerState(state string) string {
	if state == string(proto.Trigger_UNKNOWN) {
		return "Not Run"
	}

	// Because of how colorizing a string works we need to
	// do the manipulations on case first or else it will not work.
	toTitle := cases.Title(language.AmericanEnglish)
	toLower := cases.Lower(language.AmericanEnglish)
	state = toTitle.String(toLower.String(state))

	return colorizeTriggerState(state)
}

func colorizeTriggerState(state string) string {
	switch strings.ToUpper(state) {
	case proto.Trigger_UNKNOWN.String():
		return color.RedString(state)
	case proto.Trigger_FAILED.String():
		return color.RedString(state)
	case proto.Trigger_PROCESSING.String():
		return color.YellowString(state)
	case proto.Trigger_RUNNING.String():
		return color.GreenString(state)
	case proto.Trigger_SUCCESS.String():
		return color.GreenString(state)
	default:
		return state
	}
}

func PipelineTriggerConfigState(state string) string {
	if state == string(proto.PipelineTriggerConfig_UNKNOWN) {
		return "Unknown"
	}

	// Because of how colorizing a string works we need to
	// do the manipulations on case first or else it will not work.
	toTitle := cases.Title(language.AmericanEnglish)
	toLower := cases.Lower(language.AmericanEnglish)
	state = toTitle.String(toLower.String(state))
	return colorizePipelineTriggerConfigState(state)
}

func colorizePipelineTriggerConfigState(state string) string {
	switch strings.ToUpper(state) {
	case proto.PipelineTriggerConfig_UNKNOWN.String():
		return color.RedString(state)
	case proto.PipelineTriggerConfig_ACTIVE.String():
		return color.GreenString(state)
	case proto.PipelineTriggerConfig_DISABLED.String():
		return color.YellowString(state)
	default:
		return state
	}
}

func SliceJoin(slice []string, msg string) string {
	if len(slice) == 0 {
		return msg
	}

	return strings.Join(slice, ", ")
}

func Health(states []string, emoji bool) string {
	failed := 0
	passed := 0
	for _, state := range states {
		switch state {
		case string(models.RunFailed):
			failed++
		case string(models.RunUnknown):
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

func Dependencies(dependencies map[string]proto.TaskRequiredParentState) []string {
	result := []string{}
	any := []string{}
	successful := []string{}
	failure := []string{}

	for name, state := range dependencies {
		switch state {
		case proto.TaskRequiredParentState_ANY:
			any = append(any, name)
		case proto.TaskRequiredParentState_SUCCESSFUL:
			successful = append(successful, name)
		case proto.TaskRequiredParentState_FAILURE:
			failure = append(failure, name)
		case proto.TaskRequiredParentState_UNKNOWN:
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
