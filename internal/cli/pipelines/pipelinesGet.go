package pipelines

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

var cmdPipelinesGet = &cobra.Command{
	Use:     "get <id>",
	Short:   "Get details on a specific pipeline",
	Example: `$ gofer pipelines get simple_test_pipeline`,
	RunE:    pipelinesGet,
	Args:    cobra.ExactArgs(1),
}

func init() {
	CmdPipelines.AddCommand(cmdPipelinesGet)
}

func pipelinesGet(_ *cobra.Command, args []string) error {
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
	RecentRuns  string
	Tasks       []taskData
	Health      string
	Triggers    []triggerData
	Created     string
	LastRun     string
}

type taskData struct {
	Name      string
	DependsOn []string
	NumItems  int
}

type triggerData struct {
	Label    string
	Name     string
	Events   string
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
	recentRunList := [][]string{}
	recentRunHealth := []models.RunStatus{}
	for _, run := range recentRuns {
		recentRunList = append(recentRunList, []string{
			color.BlueString("• Run #" + strconv.Itoa(int(run.ID))),
			fmt.Sprintf("%s by %s %s", format.UnixMilli(run.Started, "Not yet", detail), color.CyanString(run.Trigger.Label), color.YellowString(run.Trigger.Name)),
			fmt.Sprintf("%s %s", formatStatePrefix(run.State), format.Duration(run.Started, run.Ended)),
			format.ColorizeRunStatus(format.NormalizeEnumValue(run.Status, "Unknown")),
			format.ColorizeRunState(format.NormalizeEnumValue(run.State, "Unknown")),
		})

		recentRunHealth = append(recentRunHealth, run.Status)
	}

	recentRunsTable := format.GenerateGenericTable(recentRunList, "", 4)

	triggerDataList := []triggerData{}
	for _, trigger := range pipeline.Triggers {
		recentEvents, err := recentEvents(client, pipeline.Namespace, pipeline.ID, trigger.Label, 5)
		if err != nil {
			return "", fmt.Errorf("could not get event data: %v", err)
		}

		eventDataList := [][]string{}
		for _, event := range recentEvents {
			details := ""
			evtDetail, ok := event.Details.(models.EventResolvedTriggerEvent)
			if ok {
				details = evtDetail.Result.Details
			}

			eventDataList = append(eventDataList, []string{
				format.UnixMilli(event.Emitted, "Never", detail), details,
			})
		}

		eventDataTable := format.GenerateGenericTable(eventDataList, "|", 7)

		triggerDataList = append(triggerDataList, triggerData{
			Label:    color.BlueString(trigger.Label),
			Name:     color.YellowString(trigger.Name),
			Events:   eventDataTable,
			Settings: trigger.Settings,
		})
	}

	sort.Slice(triggerDataList, func(i, j int) bool { return triggerDataList[i].Label < triggerDataList[j].Label })

	tasks := []taskData{}
	for _, task := range pipeline.CustomTasks {
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
		RecentRuns:  recentRunsTable,
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
{{.RecentRuns}}
  {{- end -}}
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
    ⟳ {{ $trigger.Label }} ({{ $trigger.Name }}) {{if $trigger.Events }}recent events:{{- end }}
{{ $trigger.Events }}
    {{- end -}}
  {{- end}}
Created {{.Created}} | Last Run {{.LastRun}} | Health {{.Health}}`

	var tpl bytes.Buffer
	t := template.Must(template.New("tmp").Parse(formatTmpl))
	_ = t.Execute(&tpl, data)
	return tpl.String(), nil
}
