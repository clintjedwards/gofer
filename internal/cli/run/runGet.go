package run

// import (
// 	"bytes"
// 	"context"
// 	"fmt"
// 	"strconv"
// 	"text/template"

// 	"github.com/clintjedwards/gofer/internal/cli/cl"
// 	"github.com/clintjedwards/gofer/internal/cli/format"
// 	proto "github.com/clintjedwards/gofer/proto/go"

// 	"github.com/fatih/color"
// 	"github.com/spf13/cobra"
// 	"google.golang.org/grpc/metadata"
// )

// var cmdRunGet = &cobra.Command{
// 	Use:     "get <pipeline> <id>",
// 	Short:   "Get details on a specific run",
// 	Example: `$ gofer run get simple_test_pipeline 23`,
// 	RunE:    runGet,
// 	Args:    cobra.ExactArgs(2),
// }

// func init() {
// 	CmdRun.AddCommand(cmdRunGet)
// }

// func runGet(_ *cobra.Command, args []string) error {
// 	pipelineID := args[0]

// 	idRaw := args[1]
// 	id, err := strconv.Atoi(idRaw)
// 	if err != nil {
// 		return err
// 	}

// 	cl.State.Fmt.Print("Retrieving run")

// 	conn, err := cl.State.Connect()
// 	if err != nil {
// 		cl.State.Fmt.PrintErr(err)
// 		cl.State.Fmt.Finish()
// 		return err
// 	}

// 	client := proto.NewGoferClient(conn)
// 	ctx, cancel := context.WithCancel(context.Background())
// 	defer cancel()

// 	md := metadata.Pairs("Authorization", "Bearer "+cl.State.Config.Token)
// 	ctx = metadata.NewOutgoingContext(ctx, md)

// 	resp, err := client.GetRun(ctx, &proto.GetRunRequest{
// 		NamespaceId: cl.State.Config.Namespace,
// 		PipelineId:  pipelineID,
// 		Id:          int64(id),
// 	})
// 	if err != nil {
// 		cl.State.Fmt.PrintErr(fmt.Sprintf("could not get run: %v", err))
// 		cl.State.Fmt.Finish()
// 		return err
// 	}

// 	taskRuns, err := client.ListTaskRuns(ctx, &proto.ListTaskRunsRequest{
// 		PipelineId: resp.Run.PipelineId,
// 		RunId:      resp.Run.Id,
// 	})
// 	if err != nil {
// 		cl.State.Fmt.PrintErr(fmt.Sprintf("could not get task run data: %v", err))
// 		cl.State.Fmt.Finish()
// 		return err
// 	}

// 	cl.State.Fmt.Println(formatRunInfo(resp.Run, taskRuns.TaskRuns, cl.State.Config.Detail))
// 	cl.State.Fmt.Finish()

// 	return nil
// }

// type data struct {
// 	ID             string
// 	State          string
// 	Started        string
// 	Duration       string
// 	PipelineID     string
// 	TriggerKind    string
// 	TriggerName    string
// 	Objects        string
// 	ObjectsExpired bool
// 	Only           bool
// 	TaskRuns       []taskRunData
// }

// type taskRunData struct {
// 	Duration    string
// 	Started     string
// 	ID          string
// 	State       string
// 	StatePrefix string
// 	DependsOn   []string
// }

// func formatStatePrefix(state string) string {
// 	if state == proto.TaskRun_RUNNING.String() {
// 		return "Running for"
// 	}

// 	if state == proto.TaskRun_PROCESSING.String() || state == proto.TaskRun_WAITING.String() {
// 		return "Waiting for"
// 	}

// 	return "Lasted"
// }

// func formatRunInfo(run *proto.Run, taskRuns []*proto.TaskRun, detail bool) string {
// 	taskRunList := []taskRunData{}
// 	for _, task := range taskRuns {
// 		taskRunList = append(taskRunList, taskRunData{
// 			Duration:    format.Duration(task.Started, task.Ended),
// 			Started:     format.UnixMilli(task.Started, "Not yet", detail),
// 			ID:          color.BlueString(task.Id),
// 			State:       format.TaskRunState(task.State.String()),
// 			StatePrefix: formatStatePrefix(task.State.String()),
// 			DependsOn:   format.Dependencies(task.Task.DependsOn),
// 		})
// 	}

// 	data := data{
// 		ID:             color.BlueString("#" + strconv.Itoa(int(run.Id))),
// 		State:          format.RunState(run.State.String()),
// 		Started:        format.UnixMilli(run.Started, "Not yet", detail),
// 		Duration:       format.Duration(run.Started, run.Ended),
// 		PipelineID:     color.BlueString(run.PipelineId),
// 		TriggerName:    color.CyanString(run.TriggerName),
// 		TriggerKind:    color.YellowString(run.TriggerKind),
// 		Objects:        format.SliceJoin(run.Objects, "None"),
// 		ObjectsExpired: false,
// 		Only:           len(run.Only) > 0,
// 		TaskRuns:       taskRunList,
// 	}

// 	const formatTmpl = `Run {{.ID}} for Pipeline {{.PipelineID}} :: {{.State}}

//   Triggered via {{.TriggerName}} ({{.TriggerKind}}) {{.Started}} and ran for {{.Duration}}

//   {{- if .TaskRuns}}

//   🗒 Task Runs {{- if .Only}} (Only a subset of task runs executed due to "only" parameter) {{- end -}}
//     {{- range $run := .TaskRuns}}
//     • {{$run.ID}} :: Started {{ $run.Started }} :: {{ $run.StatePrefix }} {{ $run.Duration }} :: {{ $run.State }}
// 	{{- if $run.DependsOn -}}
// 	  {{- range $dependant := $run.DependsOn }}
//         - {{ $dependant }}
// 	  {{- end -}}
// 	{{- end -}}
// 	{{- end}}

//   {{- end}}

//   {{- if .Objects}}

//   ☁︎ Objects: [{{ .Objects }}]
//   {{ if .ObjectsExpired }}
//   * Objects above have expired and may no longer be available due to run object limit.
//   {{- end -}}
//   {{- end}}`

// 	var tpl bytes.Buffer
// 	t := template.Must(template.New("tmp").Parse(formatTmpl))
// 	_ = t.Execute(&tpl, data)
// 	return tpl.String()
// }
