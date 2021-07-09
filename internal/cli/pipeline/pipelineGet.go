package pipeline

import (
	"bytes"
	"context"
	"fmt"
	"html/template"
	"strconv"

	"github.com/clintjedwards/gofer/internal/cli/cl"
	"github.com/clintjedwards/gofer/internal/cli/format"
	"github.com/clintjedwards/gofer/proto"
	"github.com/fatih/color"
	"github.com/spf13/cobra"
	"google.golang.org/grpc/metadata"
)

var cmdPipelineGet = &cobra.Command{
	Use:     "get <id>",
	Short:   "Get details on a specific pipeline",
	Example: `$ gofer pipeline get simple_test_pipeline`,
	RunE:    pipelineGet,
	Args:    cobra.ExactArgs(1),
}

func init() {
	CmdPipeline.AddCommand(cmdPipelineGet)
}

func pipelineGet(_ *cobra.Command, args []string) error {
	id := args[0]

	cl.State.Fmt.Print("Retrieving pipeline")

	conn, err := cl.State.Connect()
	if err != nil {
		cl.State.Fmt.PrintErr(err)
		cl.State.Fmt.Finish()
		return err
	}

	client := proto.NewGoferClient(conn)

	md := metadata.Pairs("Authorization", "Bearer "+cl.State.Config.Token)
	ctx := metadata.NewOutgoingContext(context.Background(), md)
	resp, err := client.GetPipeline(ctx, &proto.GetPipelineRequest{
		NamespaceId: cl.State.Config.Namespace,
		Id:          id,
	})
	if err != nil {
		cl.State.Fmt.PrintErr(fmt.Sprintf("could not get pipeline: %v", err))
		cl.State.Fmt.Finish()
		return err
	}

	output, err := formatPipeline(client, resp.Pipeline, cl.State.Config.Detail)
	if err != nil {
		cl.State.Fmt.PrintErr(fmt.Sprintf("could not render pipeline: %v", err))
		cl.State.Fmt.Finish()
		return err
	}

	cl.State.Fmt.Println(output)
	cl.State.Fmt.Finish()

	return nil
}

// gets last N IDs for sequential id'd resources
func getLastNIDs(n int, last int64) []int64 {
	lastIDs := []int64{}

	count := 0
	for i := last; i > 0; i-- {
		if count == n {
			break
		}

		lastIDs = append(lastIDs, i)
		count++
	}

	return lastIDs
}

func recentRuns(client proto.GoferClient, pipeline string, runs []int64) ([]*proto.Run, error) {
	if len(runs) == 0 {
		return []*proto.Run{}, nil
	}

	resp, err := client.BatchGetRuns(context.Background(), &proto.BatchGetRunsRequest{
		PipelineId: pipeline,
		Ids:        runs,
	})
	if err != nil {
		return nil, err
	}

	return resp.Runs, err
}

func recentEvents(client proto.GoferClient, namespace, pipeline, trigger string, limit int64) ([]*proto.TriggerEvent, error) {
	resp, err := client.ListTriggerEvents(context.Background(), &proto.ListTriggerEventsRequest{
		Limit:                limit,
		PipelineId:           pipeline,
		NamespaceId:          namespace,
		PipelineTriggerLabel: trigger,
	})
	if err != nil {
		return nil, err
	}

	return resp.Events, err
}

type data struct {
	ID          string
	Name        string
	State       string
	Description string
	RecentRuns  []runData
	Tasks       []taskData
	Health      string
	Triggers    []triggerData
	Objects     string
	Created     string
	LastRun     string
	Location    string
}

type runData struct {
	ID          string
	Started     string
	Lasted      string
	StatePrefix string
	State       string
	TriggerName string
	TriggerKind string
}

type taskData struct {
	Name      string
	DependsOn []string
}

type eventData struct {
	Processed string
	Details   string
}

type triggerData struct {
	Label  string
	Kind   string
	Events []eventData
	Config map[string]string
}

func formatStatePrefix(state string) string {
	if state == proto.Run_RUNNING.String() {
		return "Running for"
	}

	return "Lasted"
}

func formatPipeline(client proto.GoferClient, pipeline *proto.Pipeline, detail bool) (string, error) {
	recentRunIDs := getLastNIDs(5, pipeline.LastRunId)
	recentRuns, err := recentRuns(client, pipeline.Id, recentRunIDs)
	if err != nil {
		return "", fmt.Errorf("could not get run data: %v", err)
	}

	recentRunList := []runData{}
	recentRunHealth := []string{}
	for _, run := range recentRuns {
		recentRunList = append(recentRunList, runData{
			ID:          color.BlueString("Run #" + strconv.Itoa(int(run.Id))),
			Started:     format.UnixMilli(run.Started, "Not yet", detail),
			Lasted:      format.Duration(run.Started, run.Ended),
			State:       format.RunState(run.State.String()),
			StatePrefix: formatStatePrefix(run.State.String()),
			TriggerName: color.CyanString(run.TriggerName),
			TriggerKind: color.YellowString(run.TriggerKind),
		})

		recentRunHealth = append(recentRunHealth, run.State.String())
	}

	triggerDataList := []triggerData{}
	for _, trigger := range pipeline.Triggers {
		recentEvents, err := recentEvents(client, pipeline.Namespace, pipeline.Id, trigger.Label, 3)
		if err != nil {
			return "", fmt.Errorf("could not get event data: %v", err)
		}

		eventDataList := []eventData{}
		for _, event := range recentEvents {
			eventDataList = append(eventDataList, eventData{
				Processed: format.UnixMilli(event.Processed, "Never", detail),
				Details:   event.Result.Details,
			})
		}

		triggerDataList = append(triggerDataList, triggerData{
			Label:  color.BlueString(trigger.Label),
			Kind:   color.YellowString(trigger.Kind),
			Events: eventDataList,
			Config: trigger.Config,
		})
	}

	tasks := []taskData{}
	for _, task := range pipeline.Tasks {
		tasks = append(tasks, taskData{
			Name:      color.BlueString(task.Id),
			DependsOn: format.Dependencies(task.DependsOn),
		})
	}

	data := data{
		ID:          color.BlueString(pipeline.Id),
		Name:        pipeline.Name,
		State:       format.PipelineState(pipeline.State.String()),
		Description: pipeline.Description,
		RecentRuns:  recentRunList,
		Triggers:    triggerDataList,
		Health:      format.Health(recentRunHealth, true),
		Objects:     format.SliceJoin(pipeline.Objects, "None"),
		Tasks:       tasks,
		Created:     format.UnixMilli(pipeline.Created, "Never", detail),
		LastRun:     format.UnixMilli(pipeline.LastRunTime, "Never", detail),
		Location:    pipeline.Location,
	}

	const formatTmpl = `[{{.ID}}] {{.Name}} :: {{.State}}

  {{.Description}}
  {{- if .RecentRuns}}

  📦 Recent Runs
    {{- range $run := .RecentRuns}}
    • {{ $run.ID }} :: Started {{ $run.Started }} by trigger {{$run.TriggerName}} ({{$run.TriggerKind}}) :: {{ $run.StatePrefix }} {{ $run.Lasted }} :: {{ $run.State }}
    {{- end}}
  {{- end}}
  {{- if .Tasks }}

  🗒 Tasks:
    {{- range $task := .Tasks}}
    • {{ $task.Name }}
	{{- if $task.DependsOn -}}
	  {{- range $dependant := $task.DependsOn }}
        - {{ $dependant }}
	  {{- end -}}
	{{- end -}}
    {{- end -}}
  {{- end}}

  {{- if .Objects}}

  ☁︎ Objects: [{{ .Objects }}]
  {{- end}}

  {{- if .Triggers }}

  🗘 Attached Triggers:
    {{- range $trigger := .Triggers}}
    ⟳ {{ $trigger.Label }} ({{ $trigger.Kind }}) recent events:
      {{- range $event := $trigger.Events }}
      + {{$event.Processed}} | {{$event.Details}}
	  {{- end}}
    {{- end}}
  {{- end}}

{{- if .Location }}

  ☍ Config Location: {{.Location}}
{{- end}}

Created {{.Created}} | Last Run {{.LastRun}} | Health {{.Health}}`

	var tpl bytes.Buffer
	t := template.Must(template.New("tmp").Parse(formatTmpl))
	_ = t.Execute(&tpl, data)
	return tpl.String(), nil
}
