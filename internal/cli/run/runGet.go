package run

import (
	"bytes"
	"context"
	"fmt"
	"strconv"
	"text/template"

	"github.com/clintjedwards/gofer/internal/cli/cl"
	"github.com/clintjedwards/gofer/internal/cli/format"
	proto "github.com/clintjedwards/gofer/proto/go"

	"github.com/fatih/color"
	"github.com/spf13/cobra"
	"google.golang.org/grpc/metadata"
)

var cmdRunGet = &cobra.Command{
	Use:     "get <pipeline> <id>",
	Short:   "Get details on a specific run",
	Example: `$ gofer run get simple_test_pipeline 23`,
	RunE:    runGet,
	Args:    cobra.ExactArgs(2),
}

func init() {
	CmdRun.AddCommand(cmdRunGet)
}

func runGet(_ *cobra.Command, args []string) error {
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

	run, err := client.GetRun(ctx, &proto.GetRunRequest{
		NamespaceId: cl.State.Config.Namespace,
		PipelineId:  pipelineID,
		Id:          int64(id),
	})
	if err != nil {
		cl.State.Fmt.PrintErr(fmt.Sprintf("could not get run: %v", err))
		cl.State.Fmt.Finish()
		return err
	}

	taskRuns, err := client.ListTaskRuns(ctx, &proto.ListTaskRunsRequest{
		NamespaceId: cl.State.Config.Namespace,
		PipelineId:  run.Run.Pipeline,
		RunId:       run.Run.Id,
	})
	if err != nil {
		cl.State.Fmt.PrintErr(fmt.Sprintf("could not get task run data: %v", err))
		cl.State.Fmt.Finish()
		return err
	}

	cl.State.Fmt.Println(formatRunInfo(run.Run, taskRuns.TaskRuns, cl.State.Config.Detail))
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
	ExtensionLabel string
	ExtensionName  string
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

func formatTaskRunStatePrefix(state proto.TaskRun_TaskRunState) string {
	if state == proto.TaskRun_RUNNING {
		return "Running for"
	}

	if state == proto.TaskRun_WAITING || state == proto.TaskRun_PROCESSING {
		return "Waiting for"
	}

	return "Lasted"
}

func formatRunInfo(run *proto.Run, taskRuns []*proto.TaskRun, detail bool) string {
	taskRunList := []taskRunData{}
	for _, task := range taskRuns {
		data := taskRunData{
			Duration:    format.Duration(task.Started, task.Ended),
			Started:     format.UnixMilli(task.Started, "Not yet", detail),
			ID:          color.BlueString(task.Id),
			Status:      format.ColorizeTaskRunStatus(format.NormalizeEnumValue(task.Status.String(), "Unknown")),
			State:       format.ColorizeTaskRunState(format.NormalizeEnumValue(task.State.String(), "Unknown")),
			StatePrefix: formatTaskRunStatePrefix(task.State),
		}

		switch concreteTask := task.Task.(type) {
		case *proto.TaskRun_CommonTask:
			data.DependsOn = format.Dependencies(concreteTask.CommonTask.Settings.GetDependsOn())
		case *proto.TaskRun_CustomTask:
			data.DependsOn = format.Dependencies(concreteTask.CustomTask.GetDependsOn())
		}

		taskRunList = append(taskRunList, data)
	}

	data := data{
		ID:             color.BlueString("#" + strconv.Itoa(int(run.Id))),
		Status:         format.ColorizeRunStatus(format.NormalizeEnumValue(run.Status.String(), "Unknown")),
		State:          format.ColorizeRunState(format.NormalizeEnumValue(run.State.String(), "Unknown")),
		Started:        format.UnixMilli(run.Started, "Not yet", detail),
		Duration:       format.Duration(run.Started, run.Ended),
		PipelineID:     color.BlueString(run.Pipeline),
		ExtensionName:  color.CyanString(run.Extension.Name),
		ExtensionLabel: color.YellowString(run.Extension.Label),
		ObjectsExpired: run.StoreObjectsExpired,
		TaskRuns:       taskRunList,
	}

	const formatTmpl = `Run {{.ID}} for Pipeline {{.PipelineID}} :: {{.State}} :: {{.Status}}

  Triggered via {{.ExtensionName}} ({{.ExtensionLabel}}) {{.Started}} and ran for {{.Duration}}
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
