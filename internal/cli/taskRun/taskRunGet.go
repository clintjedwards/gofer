package taskrun

import (
	"bytes"
	"context"
	"fmt"
	"strconv"
	"text/template"

	"github.com/clintjedwards/gofer/internal/cli/cl"
	"github.com/clintjedwards/gofer/internal/cli/format"
	"github.com/clintjedwards/gofer/proto"
	"github.com/fatih/color"
	"github.com/spf13/cobra"
	"google.golang.org/grpc/metadata"
)

var cmdTaskRunGet = &cobra.Command{
	Use:     "get <pipeline> <run> <id>",
	Short:   "Get details on a specific task run",
	Example: `$ gofer taskrun get simple_test_pipeline 23 example_run`,
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
	ID         string
	State      string
	Started    string
	Ended      string
	Failure    *proto.TaskRunFailure
	ExitCode   int64
	Duration   string
	Logs       []string
	EnvVars    map[string]string
	PipelineID string
	RunID      string
	TaskRunCmd string
	ImageName  string
}

func formatTaskRunInfo(taskRun *proto.TaskRun, detail bool) string {
	data := data{
		ID:         color.BlueString(taskRun.Id),
		State:      format.TaskRunState(taskRun.State.String()),
		Started:    format.UnixMilli(taskRun.Started, "Not yet", detail),
		Duration:   format.Duration(taskRun.Started, taskRun.Ended),
		PipelineID: color.BlueString(taskRun.PipelineId),
		EnvVars:    taskRun.Task.EnvVars,
		ExitCode:   taskRun.ExitCode,
		RunID:      color.BlueString("#" + strconv.Itoa(int(taskRun.RunId))),
		Failure:    taskRun.Failure,
		TaskRunCmd: color.CyanString(fmt.Sprintf("taskrun logs %s %d %s", taskRun.PipelineId, taskRun.RunId, taskRun.Id)),
		ImageName:  taskRun.Task.Image,
	}

	const formatTmpl = `TaskRun {{.ID}} :: {{.State}}

  ✏ Parent Pipeline {{.PipelineID}} | Parent Run {{.RunID}}
  ✏ Started {{.Started}} and ran for {{.Duration}}
  ✏ {{.ImageName}}

{{- if .Failure.Kind}}

  Failure Details:
    | Exit code: {{.ExitCode}}
    | Kind: {{.Failure.Kind}}
    | Reason: {{.Failure.Description}}
{{- end}}
{{- if .EnvVars}}

  $ Environment Variables:
  {{- range $key, $value := .EnvVars}}
    | {{$key}}={{$value}}
  {{- end}}
{{- end}}

* Use '{{.TaskRunCmd}}' to view logs.
`

	var tpl bytes.Buffer
	t := template.Must(template.New("tmp").Parse(formatTmpl))
	_ = t.Execute(&tpl, data)
	return tpl.String()
}
