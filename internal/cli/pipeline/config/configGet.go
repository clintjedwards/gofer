package config

import (
	"bytes"
	"context"
	"fmt"
	"html/template"
	"sort"
	"strconv"

	"github.com/clintjedwards/gofer/internal/cli/cl"
	"github.com/clintjedwards/gofer/internal/cli/format"
	proto "github.com/clintjedwards/gofer/proto/go"
	"github.com/fatih/color"

	"github.com/spf13/cobra"
	"google.golang.org/grpc/metadata"
)

var cmdPipelineConfigGet = &cobra.Command{
	Use:     "get <pipeline_id> <version>",
	Short:   "Get pipeline config",
	Long:    `Get pipeline configuration details.`,
	Example: `$ gofer pipeline config get simple 1`,
	RunE:    pipelineConfigGet,
	Args:    cobra.ExactArgs(2),
}

func init() {
	CmdPipelineConfig.AddCommand(cmdPipelineConfigGet)
}

func pipelineConfigGet(_ *cobra.Command, args []string) error {
	pipelineID := args[0]
	versionStr := args[1]

	version, err := strconv.Atoi(versionStr)
	if err != nil {
		cl.State.Fmt.PrintErr("Could not parse verison into number; " + err.Error())
		cl.State.Fmt.Finish()
		return err
	}

	cl.State.Fmt.Print("Retrieving pipeline config")

	conn, err := cl.State.Connect()
	if err != nil {
		cl.State.Fmt.PrintErr(err)
		cl.State.Fmt.Finish()
		return err
	}

	client := proto.NewGoferClient(conn)

	md := metadata.Pairs("Authorization", "Bearer "+cl.State.Config.Token)
	ctx := metadata.NewOutgoingContext(context.Background(), md)
	resp, err := client.GetPipelineConfig(ctx, &proto.GetPipelineConfigRequest{
		NamespaceId: cl.State.Config.Namespace,
		PipelineId:  pipelineID,
		Version:     int64(version),
	})
	if err != nil {
		cl.State.Fmt.PrintErr(fmt.Sprintf("could not get pipeline config: %v", err))
		cl.State.Fmt.Finish()
		return err
	}

	output, err := formatPipelineConfig(resp.Config, cl.State.Config.Detail)
	if err != nil {
		cl.State.Fmt.PrintErr(fmt.Sprintf("could not render pipeline: %v", err))
		cl.State.Fmt.Finish()
		return err
	}

	cl.State.Fmt.Println(output)
	cl.State.Fmt.Finish()

	return nil
}

type data struct {
	Pipeline    string
	Version     string
	Parallelism string
	Name        string
	Description string
	Tasks       []taskData
	State       string
	Registered  string
	Deprecated  string
}

type taskData struct {
	Name      string
	DependsOn []string
	NumItems  int
}

func formatPipelineConfig(config *proto.PipelineConfig, detail bool) (string, error) {
	tasks := []taskData{}
	for _, task := range config.Tasks {
		tasks = append(tasks, taskData{
			Name:      color.BlueString(task.Id),
			DependsOn: format.Dependencies(task.DependsOn),
			NumItems:  len(task.DependsOn), // This is purely for sorting purposes
		})
	}

	sort.Slice(tasks, func(i, j int) bool { return tasks[i].NumItems < tasks[j].NumItems })

	parallelism := "Unlimited"
	if config.Parallelism != 0 {
		parallelism = fmt.Sprint(config.Parallelism)
	}

	data := data{
		Pipeline:    color.BlueString(config.Pipeline),
		Version:     color.MagentaString(fmt.Sprint(config.Version)),
		Parallelism: parallelism,
		Name:        config.Name,
		Description: config.Description,
		Tasks:       tasks,
		State:       format.ColorizePipelineConfigState(format.NormalizeEnumValue(config.State.String(), "Unknown")),
		Registered:  format.UnixMilli(config.Registered, "Never", detail),
		Deprecated:  format.UnixMilli(config.Deprecated, "Never", detail),
	}

	const formatTmpl = `[{{.Pipeline}}] {{.Name}} :: {{.State}}

  Version: {{.Version}}
  Parallelism: {{.Parallelism}}
  {{.Description}}
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

Registered {{.Registered}} | Deprecated {{.Deprecated}}`

	var tpl bytes.Buffer
	t := template.Must(template.New("tmp").Parse(formatTmpl))
	_ = t.Execute(&tpl, data)
	return tpl.String(), nil
}
