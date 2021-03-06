package pipeline

import (
	"bytes"
	"context"
	"fmt"
	"io"
	"sort"
	"strconv"
	"text/template"

	"github.com/clintjedwards/gofer/internal/cli/cl"
	"github.com/clintjedwards/gofer/internal/cli/format"
	"github.com/clintjedwards/gofer/internal/models"
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

func recentEvents(client proto.GoferClient, namespace, pipeline, trigger string, limit int) ([]models.EventResolvedTrigger, error) {
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	resp, err := client.ListEvents(ctx, &proto.ListEventsRequest{
		Reverse: true,
	})
	if err != nil {
		return nil, err
	}

	events := []models.EventResolvedTrigger{}

	count := 0
	for count < limit {
		response, err := resp.Recv()
		if err != nil {
			if err == io.EOF {
				break
			}
			return nil, err
		}

		event, ok := response.Event.(*proto.ListEventsResponse_ResolvedTriggerEvent)
		if !ok {
			continue
		}

		if event.ResolvedTriggerEvent.Namespace != namespace ||
			event.ResolvedTriggerEvent.Pipeline != pipeline ||
			event.ResolvedTriggerEvent.Label != trigger {
			continue
		}

		concreteEvent := &models.EventResolvedTrigger{}
		concreteEvent.FromProto(event.ResolvedTriggerEvent)

		events = append(events, *concreteEvent)
		count++
	}

	return events, nil
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
	Notifiers   []notifierData
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
	NumItems  int
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
	State  string
}

type notifierData struct {
	Label  string
	Kind   string
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
		recentEvents, err := recentEvents(client, pipeline.Namespace, pipeline.Id, trigger.Label, 5)
		if err != nil {
			return "", fmt.Errorf("could not get event data: %v", err)
		}

		eventDataList := []eventData{}
		for _, event := range recentEvents {
			eventDataList = append(eventDataList, eventData{
				Processed: format.UnixMilli(event.Emitted, "Never", detail),
				Details:   event.Result.Details,
			})
		}

		triggerDataList = append(triggerDataList, triggerData{
			Label:  color.BlueString(trigger.Label),
			Kind:   color.YellowString(trigger.Kind),
			Events: eventDataList,
			Config: trigger.Config,
			State:  format.PipelineTriggerConfigState(trigger.State.String()),
		})
	}

	sort.Slice(triggerDataList, func(i, j int) bool { return triggerDataList[i].Label < triggerDataList[j].Label })

	notifierDataList := []notifierData{}
	for _, notifier := range pipeline.Notifiers {
		notifierDataList = append(notifierDataList, notifierData{
			Label:  color.BlueString(notifier.Label),
			Kind:   color.YellowString(notifier.Kind),
			Config: notifier.Config,
		})
	}

	sort.Slice(notifierDataList, func(i, j int) bool { return notifierDataList[i].Label < notifierDataList[j].Label })

	tasks := []taskData{}
	for _, task := range pipeline.Tasks {
		tasks = append(tasks, taskData{
			Name:      color.BlueString(task.Id),
			DependsOn: format.Dependencies(task.DependsOn),
			NumItems:  len(task.DependsOn), // This is purely for sorting purposes
		})
	}

	sort.Slice(tasks, func(i, j int) bool { return tasks[i].NumItems < tasks[j].NumItems })

	data := data{
		ID:          color.BlueString(pipeline.Id),
		Name:        pipeline.Name,
		State:       format.PipelineState(pipeline.State.String()),
		Description: pipeline.Description,
		RecentRuns:  recentRunList,
		Triggers:    triggerDataList,
		Notifiers:   notifierDataList,
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

  ???? Recent Runs
    {{- range $run := .RecentRuns}}
    ??? {{ $run.ID }} :: {{ $run.Started }} by trigger {{$run.TriggerName}} ({{$run.TriggerKind}}) :: {{ $run.StatePrefix }} {{ $run.Lasted }} :: {{ $run.State }}
    {{- end}}
  {{- end}}
  {{- if .Tasks }}

  ???? Tasks:
    {{- range $task := .Tasks}}
    ??? {{ $task.Name }}
	{{- if $task.DependsOn -}}
	  {{- range $dependant := $task.DependsOn }}
        - {{ $dependant }}
	  {{- end -}}
	{{- end -}}
    {{- end -}}
  {{- end}}

  {{- if .Objects}}

  ?????? Objects: [{{ .Objects }}]
  {{- end}}

  {{- if .Triggers }}

  ???? Attached Triggers:
    {{- range $trigger := .Triggers}}
    ??? [{{ $trigger.State }}] {{ $trigger.Label }} ({{ $trigger.Kind }}) {{- if ne (len $trigger.Events) 0 }} recent events:{{- end }}
      {{- range $event := $trigger.Events }}
      + {{$event.Processed}} | {{$event.Details}}
	  {{- end}}
    {{- end}}
  {{- end}}

  {{- if .Notifiers }}

  ???? Attached Notifiers:
    {{- range $notifier := .Notifiers}}
    ???? {{ $notifier.Label }} ({{ $notifier.Kind }})
    {{- end}}
  {{- end}}

{{- if .Location }}

  ??? Config Location: {{.Location}}
{{- end}}

Created {{.Created}} | Last Run {{.LastRun}} | Health {{.Health}}`

	var tpl bytes.Buffer
	t := template.Must(template.New("tmp").Parse(formatTmpl))
	_ = t.Execute(&tpl, data)
	return tpl.String(), nil
}
