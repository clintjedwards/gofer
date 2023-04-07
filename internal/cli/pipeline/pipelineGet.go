package pipeline

import (
	"bytes"
	"context"
	"fmt"
	"html/template"
	"sort"
	"strconv"

	"github.com/clintjedwards/gofer/internal/cli/cl"
	"github.com/clintjedwards/gofer/internal/cli/format"
	"github.com/clintjedwards/gofer/internal/models"
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
	pipelineResp, err := client.GetPipeline(ctx, &proto.GetPipelineRequest{
		NamespaceId: cl.State.Config.Namespace,
		Id:          id,
	})
	if err != nil {
		cl.State.Fmt.PrintErr(fmt.Sprintf("could not get pipeline: %v", err))
		cl.State.Fmt.Finish()
		return err
	}

	subscriptionsResp, err := client.ListPipelineExtensionSubscriptions(context.Background(), &proto.ListPipelineExtensionSubscriptionsRequest{
		NamespaceId: cl.State.Config.Namespace,
		PipelineId:  id,
	})
	if err != nil {
		cl.State.Fmt.PrintErr(fmt.Sprintf("could not get extension subscriptions: %v", err))
		cl.State.Fmt.Finish()
		return err
	}

	output, err := formatPipeline(ctx, client, pipelineResp.Pipeline, subscriptionsResp.Subscriptions, cl.State.Config.Detail)
	if err != nil {
		cl.State.Fmt.PrintErr(fmt.Sprintf("could not render pipeline: %v", err))
		cl.State.Fmt.Finish()
		return err
	}

	cl.State.Fmt.Println(output)
	cl.State.Fmt.Finish()

	return nil
}

func recentEvents(client proto.GoferClient, namespace, pipeline, extensionLabel string, limit int) ([]models.Event, error) {
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	_, err := client.ListEvents(ctx, &proto.ListEventsRequest{
		Reverse: true,
	})
	if err != nil {
		return nil, err
	}

	events := []models.Event{}

	// count := 0
	// for count < limit {
	// 	response, err := resp.Recv()
	// 	if err != nil {
	// 		if err == io.EOF {
	// 			break
	// 		}
	// 		return nil, err
	// 	}

	// 	if !strings.EqualFold(response.Event.Type, string(models.EventTypeExtensionResolvedExtensionEvent)) {
	// 		continue
	// 	}

	// 	details := models.EventResolvedExtensionEvent{}
	// 	err = json.Unmarshal([]byte(response.Event.Details), &details)
	// 	if err != nil {
	// 		return nil, err
	// 	}

	// 	if details.NamespaceID != namespace ||
	// 		details.PipelineID != pipeline ||
	// 		details.Label != extensionLabel {
	// 		continue
	// 	}

	// 	concreteEvent := models.Event{
	// 		ID:      response.Event.Id,
	// 		Type:    models.EventType(response.Event.Type),
	// 		Details: details,
	// 		Emitted: response.Event.Emitted,
	// 	}

	// 	events = append(events, concreteEvent)
	// 	count++
	// }

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
	Extensions  []extensionData
	Created     string
	LastRun     string
}

type taskData struct {
	Name      string
	DependsOn []string
	NumItems  int
}

type extensionData struct {
	Label    string
	Name     string
	Events   string
	Settings map[string]string
	State    string
}

func formatStatePrefix(state proto.Run_RunState) string {
	if state == proto.Run_RUNNING {
		return "Running for"
	}

	return "Lasted"
}

func formatPipeline(ctx context.Context, client proto.GoferClient, pipeline *proto.Pipeline, extensions []*proto.PipelineExtensionSubscription, detail bool) (string, error) {
	recentRuns := recentRuns(ctx, client, pipeline.Metadata.Namespace, pipeline.Metadata.Id, 5)
	recentRunList := [][]string{}
	recentRunHealth := []proto.Run_RunStatus{}
	for _, run := range recentRuns {
		recentRunList = append(recentRunList, []string{
			color.BlueString("• Run #" + strconv.Itoa(int(run.Id))),
			fmt.Sprintf("%s by %s %s", format.UnixMilli(run.Started, "Not yet", detail), color.CyanString(run.Extension.Label), color.YellowString(run.Extension.Name)),
			fmt.Sprintf("%s %s", formatStatePrefix(run.State), format.Duration(run.Started, run.Ended)),
			format.ColorizeRunState(format.NormalizeEnumValue(run.State.String(), "Unknown")),
			format.ColorizeRunStatus(format.NormalizeEnumValue(run.Status.String(), "Unknown")),
		})

		recentRunHealth = append(recentRunHealth, run.Status)
	}

	recentRunsTable := format.GenerateGenericTable(recentRunList, "", 4)

	extensionDataList := []extensionData{}
	for _, extension := range extensions {
		recentEvents, err := recentEvents(client, extension.Namespace, extension.Pipeline, extension.Label, 5)
		if err != nil {
			return "", fmt.Errorf("could not get event data: %v", err)
		}

		eventDataList := [][]string{}
		for _, event := range recentEvents {
			details := ""
			// evtDetail, ok := event.Details.(models.EventResolvedExtensionEvent)
			// if ok {
			// 	details = evtDetail.Result.Details
			// }

			eventDataList = append(eventDataList, []string{
				format.UnixMilli(event.Emitted, "Never", detail), details,
			})
		}

		eventDataTable := format.GenerateGenericTable(eventDataList, "|", 7)

		extensionDataList = append(extensionDataList, extensionData{
			Label:    color.BlueString(extension.Label),
			Name:     color.YellowString(extension.Name),
			Events:   eventDataTable,
			Settings: extension.Settings,
		})
	}

	sort.Slice(extensionDataList, func(i, j int) bool { return extensionDataList[i].Label < extensionDataList[j].Label })

	tasks := []taskData{}
	for _, task := range pipeline.Config.CustomTasks {
		tasks = append(tasks, taskData{
			Name:      color.BlueString(task.Id),
			DependsOn: format.Dependencies(task.DependsOn),
			NumItems:  len(task.DependsOn), // This is purely for sorting purposes
		})
	}

	sort.Slice(tasks, func(i, j int) bool { return tasks[i].NumItems < tasks[j].NumItems })

	var lastRunTime int64
	if len(recentRuns) != 0 {
		lastRun := recentRuns[len(recentRuns)-1]
		lastRunTime = lastRun.Ended
	}

	data := data{
		ID:          color.BlueString(pipeline.Metadata.Id),
		Name:        pipeline.Config.Name,
		State:       format.ColorizePipelineMetadataState(format.NormalizeEnumValue(pipeline.Metadata.State.String(), "Unknown")),
		Description: pipeline.Config.Description,
		RecentRuns:  recentRunsTable,
		Extensions:  extensionDataList,
		Health:      format.Health(recentRunHealth, true),
		Tasks:       tasks,
		Created:     format.UnixMilli(pipeline.Metadata.Created, "Never", detail),
		LastRun:     format.UnixMilli(lastRunTime, "Never", detail),
	}

	const formatTmpl = `[{{.ID}}] {{.Name}} :: {{.State}}

  {{.Description}}
  {{- if .RecentRuns}}
  📦 Recent Runs
{{.RecentRuns}}
  {{- end }}
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

  {{- if .Extensions }}

  🗘 Attached Extensions:
    {{- range $extension := .Extensions}}
    ⟳ {{ $extension.Label }} ({{ $extension.Name }}) {{if $extension.Events }}recent events:{{- end }}
{{ $extension.Events }}
    {{- end -}}
  {{- end}}

Created {{.Created}} | Last Run {{.LastRun}} | Health {{.Health}}`

	var tpl bytes.Buffer
	t := template.Must(template.New("tmp").Parse(formatTmpl))
	_ = t.Execute(&tpl, data)
	return tpl.String(), nil
}
