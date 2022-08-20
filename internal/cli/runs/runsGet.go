package runs

import (
	"bytes"
	"context"
	"fmt"
	"strconv"
	"text/template"

	"github.com/clintjedwards/gofer/internal/cli/cl"
	"github.com/clintjedwards/gofer/internal/cli/format"
	"github.com/clintjedwards/gofer/models"
	proto "github.com/clintjedwards/gofer/proto/go"

	"github.com/fatih/color"
	"github.com/spf13/cobra"
	"google.golang.org/grpc/metadata"
)

var cmdRunsGet = &cobra.Command{
	Use:     "get <pipeline> <id>",
	Short:   "Get details on a specific run",
	Example: `$ gofer runs get simple_test_pipeline 23`,
	RunE:    runsGet,
	Args:    cobra.ExactArgs(2),
}

func init() {
	CmdRuns.AddCommand(cmdRunsGet)
}

func runsGet(_ *cobra.Command, args []string) error {
	pipelineID := args[0]

	idRaw := args[1]
	id, err := strconv.Atoi(idRaw)
	if err != nil {
		return err
	}

	cl.State.Fmt.Print("Retrieving run")

	conn, err := cl.State.Connect()
	if err != nil {
		cl.State.Fmt.PrintErr(err)
		cl.State.Fmt.Finish()
		return err
	}

	client := proto.NewGoferClient(conn)
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	md := metadata.Pairs("Authorization", "Bearer "+cl.State.Config.Token)
	ctx = metadata.NewOutgoingContext(ctx, md)

	resp, err := client.GetRun(ctx, &proto.GetRunRequest{
		NamespaceId: cl.State.Config.Namespace,
		PipelineId:  pipelineID,
		Id:          int64(id),
	})
	if err != nil {
		cl.State.Fmt.PrintErr(fmt.Sprintf("could not get run: %v", err))
		cl.State.Fmt.Finish()
		return err
	}

	protoTaskRuns, err := client.ListTaskRuns(ctx, &proto.ListTaskRunsRequest{
		NamespaceId: cl.State.Config.Namespace,
		PipelineId:  resp.Run.Pipeline,
		RunId:       resp.Run.Id,
	})
	if err != nil {
		cl.State.Fmt.PrintErr(fmt.Sprintf("could not get task run data: %v", err))
		cl.State.Fmt.Finish()
		return err
	}

	run := models.Run{}
	run.FromProto(resp.Run)

	taskRuns := []models.TaskRun{}
	for _, protoTaskRun := range protoTaskRuns.TaskRuns {
		taskrun := models.TaskRun{}
		taskrun.FromProto(protoTaskRun)
		taskRuns = append(taskRuns, taskrun)
	}

	cl.State.Fmt.Println(formatRunInfo(run, taskRuns, cl.State.Config.Detail))
	cl.State.Fmt.Finish()

	return nil
}

type data struct {
	ID             string
	State          string
	Status         string
	Started        string
	Duration       string
	PipelineID     string
	TriggerLabel   string
	TriggerName    string
	ObjectsExpired bool
	TaskRuns       []taskRunData
}

type taskRunData struct {
	Duration    string
	Started     string
	ID          string
	Status      string
	State       string
	StatePrefix string
	DependsOn   []string
}

func formatTaskRunStatePrefix(state models.TaskRunState) string {
	if state == models.TaskRunStateRunning {
		return "Running for"
	}

	if state == models.TaskRunStateWaiting || state == models.TaskRunStateProcessing {
		return "Waiting for"
	}

	return "Lasted"
}

func formatRunInfo(run models.Run, taskRuns []models.TaskRun, detail bool) string {
	taskRunList := []taskRunData{}
	for _, task := range taskRuns {
		taskRunList = append(taskRunList, taskRunData{
			Duration:    format.Duration(task.Started, task.Ended),
			Started:     format.UnixMilli(task.Started, "Not yet", detail),
			ID:          color.BlueString(task.ID),
			Status:      format.ColorizeTaskRunStatus(format.NormalizeEnumValue(task.Status, "Unknown")),
			State:       format.ColorizeTaskRunState(format.NormalizeEnumValue(task.State, "Unknown")),
			StatePrefix: formatTaskRunStatePrefix(task.State),
			DependsOn:   format.Dependencies(task.Task.DependsOn),
		})
	}

	data := data{
		ID:             color.BlueString("#" + strconv.Itoa(int(run.ID))),
		Status:         format.ColorizeRunStatus(format.NormalizeEnumValue(run.Status, "Unknown")),
		State:          format.ColorizeRunState(format.NormalizeEnumValue(run.State, "Unknown")),
		Started:        format.UnixMilli(run.Started, "Not yet", detail),
		Duration:       format.Duration(run.Started, run.Ended),
		PipelineID:     color.BlueString(run.Pipeline),
		TriggerName:    color.CyanString(run.Trigger.Name),
		TriggerLabel:   color.YellowString(run.Trigger.Label),
		ObjectsExpired: run.StoreObjectsExpired,
		TaskRuns:       taskRunList,
	}

	const formatTmpl = `Run {{.ID}} for Pipeline {{.PipelineID}} :: {{.State}} :: {{.Status}}

  Triggered via {{.TriggerName}} ({{.TriggerLabel}}) {{.Started}} and ran for {{.Duration}}
  {{- if .TaskRuns}}

  🗒 Task Runs
    {{- range $run := .TaskRuns}}
    • {{$run.ID}} :: Started {{ $run.Started }} :: {{ $run.StatePrefix }} {{ $run.Duration }} :: {{ $run.State }} :: {{ $run.Status }}
	{{- if $run.DependsOn -}}
	  {{- range $dependant := $run.DependsOn }}
        - {{ $dependant }}
	  {{- end -}}
	{{- end -}}
	{{- end}}

  {{- end}}

  Objects Expired: {{.ObjectsExpired}}`

	var tpl bytes.Buffer
	t := template.Must(template.New("tmp").Parse(formatTmpl))
	_ = t.Execute(&tpl, data)
	return tpl.String()
}
