package pipeline

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"io"
	"sort"
	"strconv"
	"strings"
	"text/template"

	"github.com/clintjedwards/gofer/internal/cli/cl"
	"github.com/clintjedwards/gofer/internal/cli/format"
	"github.com/clintjedwards/gofer/models"
	proto "github.com/clintjedwards/gofer/proto/go"

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

	pipeline := models.Pipeline{}
	pipeline.FromProto(resp.Pipeline)

	output, err := formatPipeline(ctx, client, &pipeline, cl.State.Config.Detail)
	if err != nil {
		cl.State.Fmt.PrintErr(fmt.Sprintf("could not render pipeline: %v", err))
		cl.State.Fmt.Finish()
		return err
	}

	cl.State.Fmt.Println(output)
	cl.State.Fmt.Finish()

	return nil
}

func recentEvents(client proto.GoferClient, namespace, pipeline, triggerLabel string, limit int) ([]models.Event, error) {
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	resp, err := client.ListEvents(ctx, &proto.ListEventsRequest{
		Reverse: true,
	})
	if err != nil {
		return nil, err
	}

	events := []models.Event{}

	count := 0
	for count < limit {
		response, err := resp.Recv()
		if err != nil {
			if err == io.EOF {
				break
			}
			return nil, err
		}

		if !strings.EqualFold(response.Event.Kind, string(models.EventKindResolvedTriggerEvent)) {
			continue
		}

		details := models.EventResolvedTriggerEvent{}
		err = json.Unmarshal([]byte(response.Event.Details), &details)
		if err != nil {
			return nil, err
		}

		if details.NamespaceID != namespace ||
			details.PipelineID != pipeline ||
			details.Label != triggerLabel {
			continue
		}

		concreteEvent := models.Event{
			ID:      response.Event.Id,
			Kind:    models.EventKind(response.Event.Kind),
			Details: details,
			Emitted: response.Event.Emitted,
		}

		events = append(events, concreteEvent)
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
	Created     string
	LastRun     string
}

type runData struct {
	ID           string
	Started      string
	Lasted       string
	Status       string
	StatePrefix  string
	State        string
	TriggerLabel string
	TriggerName  string
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
	Label    string
	Name     string
	Events   []eventData
	Settings map[string]string
	State    string
}

func formatStatePrefix(state models.RunState) string {
	if state == models.RunStateRunning {
		return "Running for"
	}

	return "Lasted"
}

func formatPipeline(ctx context.Context, client proto.GoferClient, pipeline *models.Pipeline, detail bool) (string, error) {
	recentRuns := recentRuns(ctx, client, pipeline.Namespace, pipeline.ID, 5)
	recentRunList := []runData{}
	recentRunHealth := []models.RunStatus{}
	for _, run := range recentRuns {
		recentRunList = append(recentRunList, runData{
			ID:           color.BlueString("Run #" + strconv.Itoa(int(run.ID))),
			Started:      format.UnixMilli(run.Started, "Not yet", detail),
			Lasted:       format.Duration(run.Started, run.Ended),
			Status:       format.ColorizeRunStatus(format.NormalizeEnumValue(run.Status, "Unknown")),
			State:        format.ColorizeRunState(format.NormalizeEnumValue(run.State, "Unknown")),
			StatePrefix:  formatStatePrefix(run.State),
			TriggerLabel: color.CyanString(run.Trigger.Label),
			TriggerName:  color.YellowString(run.Trigger.Name),
		})

		recentRunHealth = append(recentRunHealth, run.Status)
	}

	triggerDataList := []triggerData{}
	for _, trigger := range pipeline.Triggers {
		recentEvents, err := recentEvents(client, pipeline.Namespace, pipeline.ID, trigger.Label, 5)
		if err != nil {
			return "", fmt.Errorf("could not get event data: %v", err)
		}

		eventDataList := []eventData{}
		for _, event := range recentEvents {
			details := ""
			evtDetail, ok := event.Details.(models.EventResolvedTriggerEvent)
			if ok {
				details = evtDetail.Result.Details
			}

			eventDataList = append(eventDataList, eventData{
				Processed: format.UnixMilli(event.Emitted, "Never", detail),
				Details:   details,
			})
		}

		triggerDataList = append(triggerDataList, triggerData{
			Label:    color.BlueString(trigger.Label),
			Name:     color.YellowString(trigger.Name),
			Events:   eventDataList,
			Settings: trigger.Settings,
		})
	}

	sort.Slice(triggerDataList, func(i, j int) bool { return triggerDataList[i].Label < triggerDataList[j].Label })

	tasks := []taskData{}
	for _, task := range pipeline.Tasks {
		tasks = append(tasks, taskData{
			Name:      color.BlueString(task.ID),
			DependsOn: format.Dependencies(task.DependsOn),
			NumItems:  len(task.DependsOn), // This is purely for sorting purposes
		})
	}

	sort.Slice(tasks, func(i, j int) bool { return tasks[i].NumItems < tasks[j].NumItems })

	var lastRunTime int64 = 0
	if len(recentRuns) != 0 {
		lastRun := recentRuns[len(recentRuns)-1]
		lastRunTime = lastRun.Ended
	}

	data := data{
		ID:          color.BlueString(pipeline.ID),
		Name:        pipeline.Name,
		State:       format.ColorizePipelineState(format.NormalizeEnumValue(pipeline.State, "Unknown")),
		Description: pipeline.Description,
		RecentRuns:  recentRunList,
		Triggers:    triggerDataList,
		Health:      format.Health(recentRunHealth, true),
		Tasks:       tasks,
		Created:     format.UnixMilli(pipeline.Created, "Never", detail),
		LastRun:     format.UnixMilli(lastRunTime, "Never", detail),
	}

	const formatTmpl = `[{{.ID}}] {{.Name}} :: {{.State}}

  {{.Description}}
  {{- if .RecentRuns}}

  📦 Recent Runs
    {{- range $run := .RecentRuns}}
    • {{ $run.ID }} :: {{ $run.Started }} by trigger {{$run.TriggerLabel}} ({{$run.TriggerName}}) :: {{ $run.StatePrefix }} {{ $run.Lasted }} :: {{ $run.State }}
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

  {{- if .Triggers }}

  🗘 Attached Triggers:
    {{- range $trigger := .Triggers}}
    ⟳ [{{ $trigger.State }}] {{ $trigger.Label }} ({{ $trigger.Name }}) {{- if ne (len $trigger.Events) 0 }} recent events:{{- end }}
      {{- range $event := $trigger.Events }}
      + {{$event.Processed}} | {{$event.Details}}
	  {{- end}}
    {{- end}}
  {{- end}}

Created {{.Created}} | Last Run {{.LastRun}} | Health {{.Health}}`

	var tpl bytes.Buffer
	t := template.Must(template.New("tmp").Parse(formatTmpl))
	_ = t.Execute(&tpl, data)
	return tpl.String(), nil
}
