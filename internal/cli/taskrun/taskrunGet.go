package taskrun

import (
	"bytes"
	"context"
	"fmt"
	"strconv"
	"strings"
	"text/template"

	"github.com/clintjedwards/gofer/internal/cli/cl"
	"github.com/clintjedwards/gofer/internal/cli/format"
	proto "github.com/clintjedwards/gofer/proto/go"
	"golang.org/x/text/cases"
	"golang.org/x/text/language"

	"github.com/fatih/color"
	"github.com/spf13/cobra"
	"google.golang.org/grpc/metadata"
)

var cmdTaskRunGet = &cobra.Command{
	Use:     "get <pipeline> <run> <task_id>",
	Short:   "Get details on a specific task run",
	Example: `$ gofer taskrun get simple 23 example_task`,
	RunE:    taskrunGet,
	Args:    cobra.ExactArgs(3),
}

func init() {
	CmdTaskRun.AddCommand(cmdTaskRunGet)
}

func taskrunGet(_ *cobra.Command, args []string) error {
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

	cl.State.Fmt.Println(formatTaskRunInfo(resp.TaskRun, cl.State.Config.Detail))
	cl.State.Fmt.Finish()

	return nil
}

type data struct {
	ID           string
	State        string
	Status       string
	Started      string
	Ended        string
	StatusReason *proto.TaskRunStatusReason
	ExitCode     int64
	Duration     string
	Logs         []string
	EnvVars      string
	PipelineID   string
	RunID        string
	TaskRunCmd   string
	ImageName    string
}

func formatTaskRunInfo(taskRun *proto.TaskRun, detail bool) string {
	exitCode := taskRun.ExitCode

	faint := color.New(color.Faint).SprintfFunc()

	// First we create a FuncMap with which to register the function.
	funcMap := template.FuncMap{
		"magenta": color.MagentaString,
		"faint":   faint,
	}

	variableMap := map[string][]string{}

	for _, variable := range taskRun.Variables {
		variableMap[variable.Key] = []string{
			color.MagentaString("│"),
			variable.Key,
			color.BlueString(variable.Value),
			faint("%s", formatSource(string(variable.Source))),
		}
	}

	var imageName string

	switch concreteTask := taskRun.Task.(type) {
	case *proto.TaskRun_CommonTask:
		imageName = concreteTask.CommonTask.Registration.Image
	case *proto.TaskRun_CustomTask:
		imageName = concreteTask.CustomTask.Image
	}

	variableList := [][]string{}
	for _, variable := range variableMap {
		variableList = append(variableList, variable)
	}

	variablesTable := format.GenerateGenericTable(variableList, "", 4)

	data := data{
		ID:         color.BlueString(taskRun.Id),
		State:      format.ColorizeTaskRunState(format.NormalizeEnumValue(taskRun.State.String(), "Unknown")),
		Status:     format.ColorizeTaskRunStatus(format.NormalizeEnumValue(taskRun.Status.String(), "Unknown")),
		Started:    format.UnixMilli(taskRun.Started, "Not yet", detail),
		Duration:   format.Duration(taskRun.Started, taskRun.Ended),
		PipelineID: color.BlueString(taskRun.Pipeline),
		EnvVars:    variablesTable,
		ExitCode:   exitCode,
		RunID:      color.BlueString("#" + strconv.Itoa(int(taskRun.Run))),
		TaskRunCmd: color.CyanString(fmt.Sprintf("gofer taskrun logs %s %d %s", taskRun.Pipeline, taskRun.Run, taskRun.Id)),
		ImageName:  color.BlueString(imageName),
	}

	if taskRun.StatusReason.Description != "" {
		data.StatusReason = taskRun.StatusReason
	}

	const formatTmpl = `TaskRun {{.ID}} :: {{.State}} :: {{.Status}}

   {{magenta "│"}} Parent Pipeline: {{.PipelineID}}
   {{magenta "├─"}} Parent Run: {{.RunID}}
   {{magenta "├──"}} Task ID: {{.ID}}
   {{magenta "│"}} Started {{.Started}} and ran for {{.Duration}}
  {{if .ImageName}} {{magenta "│"}} Image {{.ImageName}} {{- end}}
  {{if .ExitCode}} {{magenta "│"}} Exit Code: {{.ExitCode}} {{- end}}
{{- if .StatusReason}}

 Status Details:
   {{magenta "│"}} Reason: {{.StatusReason.Reason}}
   {{magenta "│"}} Description: {{.StatusReason.Description}}
{{ end }}

{{- if .EnvVars}}
 $ Environment Variables:
{{.EnvVars}}
{{- end}}
* Use '{{.TaskRunCmd}}' to view logs.`

	var tpl bytes.Buffer
	t := template.Must(template.New("tmp").Funcs(funcMap).Parse(formatTmpl))
	_ = t.Execute(&tpl, data)
	return tpl.String()
}

func formatSource(source string) string {
	toTitle := cases.Title(language.AmericanEnglish)
	toLower := cases.Lower(language.AmericanEnglish)

	source = strings.ReplaceAll(source, "_", " ")
	source = toTitle.String(toLower.String(source))

	return source
}
