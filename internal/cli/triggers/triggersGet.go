package triggers

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

var cmdTriggersGet = &cobra.Command{
	Use:     "get <name>",
	Short:   "Get a specific trigger by name.",
	Example: `$ gofer triggers get cron`,
	RunE:    triggersGet,
	Args:    cobra.ExactArgs(1),
}

func init() {
	CmdTriggers.AddCommand(cmdTriggersGet)
}

func triggersGet(_ *cobra.Command, args []string) error {
	name := args[0]

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
		Name: name,
	})
	if err != nil {
		cl.State.Fmt.PrintErr(fmt.Sprintf("could not get trigger: %v", err))
		cl.State.Fmt.Finish()
		return err
	}

	cl.State.Fmt.Println(formatTriggerInfo(triggerInfo{
		Name:          color.YellowString(resp.Trigger.Name),
		State:         cliformat.ColorizeTriggerState(cliformat.NormalizeEnumValue(resp.Trigger.State.String(), "Unknown")),
		Status:        cliformat.ColorizeTriggerStatus(cliformat.NormalizeEnumValue(resp.Trigger.Status.String(), "Unknown")),
		Started:       cliformat.UnixMilli(resp.Trigger.Started, "Not Started", cl.State.Config.Detail),
		URL:           resp.Trigger.Url,
		Documentation: resp.Trigger.Documentation,
	}))
	cl.State.Fmt.Finish()

	return nil
}

type triggerInfo struct {
	Name          string
	URL           string
	Started       string
	State         string
	Status        string
	Documentation string
}

func formatTriggerInfo(ti triggerInfo) string {
	const formatTmpl = `Trigger "{{.Name}}" :: {{.Status}} :: {{.State}}

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
