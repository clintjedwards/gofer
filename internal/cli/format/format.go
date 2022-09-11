package format

import (
	"fmt"
	"strings"
	"time"

	"github.com/clintjedwards/gofer/models"
	"github.com/olekukonko/tablewriter"

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

	if state == "Unknown" {
		return unknownString
	}

	return state
}

func GenerateGenericTable(rows int, data [][]string) string {
	tableString := &strings.Builder{}
	table := tablewriter.NewWriter(tableString)

	headers := []string{}
	for i := 1; i < rows; i++ {
		headers = append(headers, "  ")
	}

	table.SetHeader(headers)
	table.SetAutoWrapText(false)
	table.SetAlignment(tablewriter.ALIGN_LEFT)
	table.SetHeaderAlignment(tablewriter.ALIGN_LEFT)
	table.SetHeaderLine(false)
	table.SetBorder(false)
	table.SetAutoFormatHeaders(false)
	table.SetRowSeparator("―")
	table.SetRowLine(false)
	table.SetColumnSeparator("")
	table.SetCenterSeparator("")

	table.AppendBulk(data)

	table.Render()
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

func ColorizePipelineState(state string) string {
	switch strings.ToUpper(state) {
	case string(models.PipelineStateUnknown):
		return color.RedString(state)
	case string(models.PipelineStateActive):
		return color.GreenString(state)
	case string(models.PipelineStateDisabled):
		return color.YellowString(state)
	default:
		return state
	}
}

func ColorizeRunState(state string) string {
	switch strings.ToUpper(state) {
	case string(models.RunStateUnknown):
		return color.RedString(state)
	case string(models.RunStatePending):
		return color.YellowString(state)
	case string(models.RunStateRunning):
		return color.YellowString(state)
	case string(models.RunStateComplete):
		return color.GreenString(state)
	default:
		return state
	}
}

func ColorizeRunStatus(status string) string {
	switch strings.ToUpper(status) {
	case string(models.RunStatusUnknown):
		return color.RedString(status)
	case string(models.RunStatusSuccessful):
		return color.GreenString(status)
	case string(models.RunStatusFailed):
		return color.RedString(status)
	case string(models.RunStatusCancelled):
		return status
	default:
		return status
	}
}

func ColorizeTaskRunState(state string) string {
	switch strings.ToUpper(state) {
	case string(models.TaskRunStateUnknown):
		return color.RedString(state)
	case string(models.TaskRunStateProcessing):
		return color.YellowString(state)
	case string(models.TaskRunStateWaiting):
		return color.YellowString(state)
	case string(models.TaskRunStateRunning):
		return color.YellowString(state)
	case string(models.TaskRunStateComplete):
		return color.GreenString(state)
	default:
		return state
	}
}

func ColorizeTaskRunStatus(status string) string {
	switch strings.ToUpper(status) {
	case string(models.TaskRunStatusUnknown):
		return color.RedString(status)
	case string(models.TaskRunStatusSuccessful):
		return color.GreenString(status)
	case string(models.TaskRunStatusFailed):
		return color.RedString(status)
	case string(models.TaskRunStatusCancelled):
		return status
	case string(models.TaskRunStatusSkipped):
		return status
	default:
		return status
	}
}

func ColorizeTriggerState(state string) string {
	switch strings.ToUpper(state) {
	case string(models.TriggerStateUnknown):
		return color.RedString(state)
	case string(models.TriggerStateProcessing):
		return color.YellowString(state)
	case string(models.TriggerStateRunning):
		return color.GreenString(state)
	case string(models.TriggerStateExited):
		return color.RedString(state)
	default:
		return state
	}
}

func ColorizeTriggerStatus(status string) string {
	switch strings.ToUpper(status) {
	case string(models.TriggerStatusUnknown):
		return color.RedString(status)
	case string(models.TriggerStatusEnabled):
		return color.GreenString(status)
	case string(models.TriggerStatusDisabled):
		return color.YellowString(status)
	default:
		return status
	}
}

func ColorizeCommonTaskStatus(status string) string {
	switch strings.ToUpper(status) {
	case string(models.CommonTaskStatusUnknown):
		return color.RedString(status)
	case string(models.CommonTaskStatusEnabled):
		return color.GreenString(status)
	case string(models.CommonTaskStatusDisabled):
		return color.YellowString(status)
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

func Health(states []models.RunStatus, emoji bool) string {
	failed := 0
	passed := 0
	for _, state := range states {
		switch state {
		case models.RunStatusFailed:
			failed++
		case models.RunStatusUnknown:
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

func Dependencies(dependencies map[string]models.RequiredParentStatus) []string {
	result := []string{}
	any := []string{}
	successful := []string{}
	failure := []string{}

	for name, state := range dependencies {
		switch state {
		case models.RequiredParentStatusAny:
			any = append(any, name)
		case models.RequiredParentStatusSuccess:
			successful = append(successful, name)
		case models.RequiredParentStatusFailure:
			failure = append(failure, name)
		case models.RequiredParentStatusUnknown:
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
