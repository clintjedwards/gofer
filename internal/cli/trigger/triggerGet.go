package trigger

import (
	"bytes"
	"context"
	"fmt"
	"text/template"

	"github.com/clintjedwards/gofer/internal/cli/cl"
	cliformat "github.com/clintjedwards/gofer/internal/cli/format"
	"github.com/clintjedwards/gofer/proto"
	"github.com/fatih/color"
	"github.com/spf13/cobra"
	"google.golang.org/grpc/metadata"
)

var cmdTriggerGet = &cobra.Command{
	Use:     "get <kind>",
	Short:   "Get a specific trigger by kind.",
	Example: `$ gofer trigger get cron`,
	RunE:    triggerGet,
	Args:    cobra.ExactArgs(1),
}

func init() {
	CmdTrigger.AddCommand(cmdTriggerGet)
}

func triggerGet(_ *cobra.Command, args []string) error {
	kind := args[0]

	cl.State.Fmt.Print("Retrieving trigger")

	conn, err := cl.State.Connect()
	if err != nil {
		cl.State.Fmt.PrintErr(err)
		cl.State.Fmt.Finish()
		return err
	}

	client := proto.NewGoferClient(conn)

	md := metadata.Pairs("Authorization", "Bearer "+cl.State.Config.Token)
	ctx := metadata.NewOutgoingContext(context.Background(), md)
	resp, err := client.GetTrigger(ctx, &proto.GetTriggerRequest{
		Kind: kind,
	})
	if err != nil {
		cl.State.Fmt.PrintErr(fmt.Sprintf("could not get trigger: %v", err))
		cl.State.Fmt.Finish()
		return err
	}

	cl.State.Fmt.Println(formatTriggerInfo(triggerInfo{
		Kind:          color.YellowString(resp.Trigger.Kind),
		State:         cliformat.TriggerState(resp.Trigger.State.String()),
		Started:       cliformat.UnixMilli(resp.Trigger.Started, "Not yet", cl.State.Config.Detail),
		URL:           resp.Trigger.Url,
		Documentation: resp.Trigger.Documentation,
		Image:         resp.Trigger.Image,
	}))
	cl.State.Fmt.Finish()

	return nil
}

type triggerInfo struct {
	Kind          string
	URL           string
	Started       string
	State         string
	Documentation string
	Image         string
}

func formatTriggerInfo(ti triggerInfo) string {
	const formatTmpl = `Trigger "{{.Kind}}" :: {{.State}} :: {{.Image}}

Started {{.Started}}

Endpoint: {{.URL}}

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
