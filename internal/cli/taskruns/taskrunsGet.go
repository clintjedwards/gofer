package taskruns

import (
	"bytes"
	"context"
	"fmt"
	"strconv"
	"strings"
	"text/template"

	"github.com/clintjedwards/gofer/internal/cli/cl"
	"github.com/clintjedwards/gofer/internal/cli/format"
	"github.com/clintjedwards/gofer/models"
	proto "github.com/clintjedwards/gofer/proto/go"
	"golang.org/x/text/cases"
	"golang.org/x/text/language"

	"github.com/fatih/color"
	"github.com/spf13/cobra"
	"google.golang.org/grpc/metadata"
)

var cmdTaskRunsGet = &cobra.Command{
	Use:     "get <pipeline> <run> <id>",
	Short:   "Get details on a specific task run",
	Example: `$ gofer taskruns get simple_test_pipeline 23 example_run`,
	RunE:    taskrunsGet,
	Args:    cobra.ExactArgs(3),
}

func init() {
	CmdTaskRuns.AddCommand(cmdTaskRunsGet)
}

func taskrunsGet(_ *cobra.Command, args []string) error {
	pipeline := args[0]

	runIDRaw := args[1]
	runID, err := strconv.Atoi(runIDRaw)
	if err != nil {
		return err
	}

	id := args[2]

	cl.State.Fmt.Print("Retrieving taskrun")

	conn, err := cl.State.Connect()
	if err != nil {
		cl.State.Fmt.PrintErr(err)
		cl.State.Fmt.Finish()
		return err
	}

	client := proto.NewGoferClient(conn)

	md := metadata.Pairs("Authorization", "Bearer "+cl.State.Config.Token)
	ctx := metadata.NewOutgoingContext(context.Background(), md)
	resp, err := client.GetTaskRun(ctx, &proto.GetTaskRunRequest{
		NamespaceId: cl.State.Config.Namespace,
		RunId:       int64(runID),
		PipelineId:  pipeline,
		Id:          id,
	})
	if err != nil {
		cl.State.Fmt.PrintErr(fmt.Sprintf("could not get taskrun: %v", err))
		cl.State.Fmt.Finish()
		return err
	}

	taskrun := models.TaskRun{}
	taskrun.FromProto(resp.TaskRun)

	cl.State.Fmt.Println(formatTaskRunInfo(&taskrun, cl.State.Config.Detail))
	cl.State.Fmt.Finish()

	return nil
}

type variable struct {
	Key    string
	Value  string
	Source string
}

type data struct {
	ID           string
	State        string
	Status       string
	Started      string
	Ended        string
	StatusReason *models.TaskRunStatusReason
	ExitCode     int64
	Duration     string
	Logs         []string
	EnvVars      []variable
	PipelineID   string
	RunID        string
	TaskRunCmd   string
	ImageName    string
}

func formatTaskRunInfo(taskRun *models.TaskRun, detail bool) string {
	var exitCode int64 = 0
	if taskRun.ExitCode != nil {
		exitCode = *taskRun.ExitCode
	}

	data := data{
		ID:           color.BlueString(taskRun.ID),
		State:        format.ColorizeTaskRunState(format.NormalizeEnumValue(taskRun.State, "Unknown")),
		Status:       format.ColorizeTaskRunStatus(format.NormalizeEnumValue(taskRun.Status, "Unknown")),
		Started:      format.UnixMilli(taskRun.Started, "Not yet", detail),
		Duration:     format.Duration(taskRun.Started, taskRun.Ended),
		PipelineID:   color.BlueString(taskRun.Pipeline),
		EnvVars:      convertVariables(taskRun.Task.Variables),
		ExitCode:     exitCode,
		RunID:        color.BlueString("#" + strconv.Itoa(int(taskRun.Run))),
		StatusReason: taskRun.StatusReason,
		TaskRunCmd:   color.CyanString(fmt.Sprintf("taskrun logs %s %d %s", taskRun.Pipeline, taskRun.Run, taskRun.ID)),
		ImageName:    taskRun.Task.Image,
	}

	const formatTmpl = `TaskRun {{.ID}} :: {{.Status}} :: {{.State}}

  ✏ Parent Pipeline {{.PipelineID}} | Parent Run {{.RunID}}
  ✏ Started {{.Started}} and ran for {{.Duration}}
  ✏ {{.ImageName}}
 {{if .ExitCode}} ✏ Exit Code: {{.ExitCode}} {{- end}}
{{- if .StatusReason}}

  Status Details:
    | Reason: {{.StatusReason.Reason}}
	| Description: {{.StatusReason.Description}}
{{- end}}
{{- if .EnvVars}}

  $ Environment Variables:
  {{- range $v := .EnvVars}}
    | {{$v.Key}}={{$v.Value}} from {{$v.Source}}
  {{- end}}
{{- end}}

* Use '{{.TaskRunCmd}}' to view logs.
`

	var tpl bytes.Buffer
	t := template.Must(template.New("tmp").Parse(formatTmpl))
	_ = t.Execute(&tpl, data)
	return tpl.String()
}

func convertVariables(vars []models.Variable) []variable {
	convertedVariables := []variable{}

	for _, rawVar := range vars {
		newVar := variable{
			Key:    rawVar.Key,
			Value:  rawVar.Value,
			Source: formatSource(string(rawVar.Source)),
		}

		convertedVariables = append(convertedVariables, newVar)
	}

	return convertedVariables
}

func formatSource(source string) string {
	toTitle := cases.Title(language.AmericanEnglish)
	toLower := cases.Lower(language.AmericanEnglish)

	source = strings.ReplaceAll(source, "_", " ")
	source = toTitle.String(toLower.String(source))

	return source
}
