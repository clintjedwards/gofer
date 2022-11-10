package commonTask

import (
	"bytes"
	"context"
	"fmt"
	"text/template"

	"github.com/clintjedwards/gofer/internal/cli/cl"
	cliformat "github.com/clintjedwards/gofer/internal/cli/format"
	proto "github.com/clintjedwards/gofer/proto/go"

	"github.com/fatih/color"
	"github.com/spf13/cobra"
	"google.golang.org/grpc/metadata"
)

var cmdCommonTaskGet = &cobra.Command{
	Use:     "get <name>",
	Short:   "Get a specific common task by name.",
	Example: `$ gofer common-task get cron`,
	RunE:    commonTaskGet,
	Args:    cobra.ExactArgs(1),
}

func init() {
	CmdCommonTask.AddCommand(cmdCommonTaskGet)
}

func commonTaskGet(_ *cobra.Command, args []string) error {
	name := args[0]

	cl.State.Fmt.Print("Retrieving common task")

	conn, err := cl.State.Connect()
	if err != nil {
		cl.State.Fmt.PrintErr(err)
		cl.State.Fmt.Finish()
		return err
	}

	client := proto.NewGoferClient(conn)

	md := metadata.Pairs("Authorization", "Bearer "+cl.State.Config.Token)
	ctx := metadata.NewOutgoingContext(context.Background(), md)
	resp, err := client.GetCommonTask(ctx, &proto.GetCommonTaskRequest{
		Name: name,
	})
	if err != nil {
		cl.State.Fmt.PrintErr(fmt.Sprintf("could not get common task: %v", err))
		cl.State.Fmt.Finish()
		return err
	}

	cl.State.Fmt.Println(formatCommonTaskInfo(commontaskInfo{
		Name:          color.YellowString(resp.CommonTask.Name),
		Image:         resp.CommonTask.Image,
		Status:        cliformat.ColorizeCommonTaskStatus(cliformat.NormalizeEnumValue(resp.CommonTask.Status.String(), "Unknown")),
		Documentation: resp.CommonTask.Documentation,
	}))
	cl.State.Fmt.Finish()

	return nil
}

type commontaskInfo struct {
	Name          string
	Image         string
	Status        string
	Documentation string
}

func formatCommonTaskInfo(ti commontaskInfo) string {
	const formatTmpl = `CommonTask "{{.Name}}" :: {{.Status}}

  Image: {{.Image}}

{{- if .Documentation }}

  Documentation: {{.Documentation}}
{{- else}}

  No Documentation found
{{- end}}`

	var tpl bytes.Buffer
	t := template.Must(template.New("tmp").Parse(formatTmpl))
	_ = t.Execute(&tpl, ti)
	return tpl.String()
}
