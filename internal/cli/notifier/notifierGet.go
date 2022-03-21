package notifier

import (
	"bytes"
	"context"
	"fmt"
	"text/template"

	"github.com/clintjedwards/gofer/internal/cli/cl"
	"github.com/clintjedwards/gofer/proto"
	"github.com/fatih/color"
	"github.com/spf13/cobra"
	"google.golang.org/grpc/metadata"
)

var cmdNotifierGet = &cobra.Command{
	Use:     "get <name>",
	Short:   "Get a specific notifier by name.",
	Example: `$ gofer notifier get log`,
	RunE:    notifierGet,
	Args:    cobra.ExactArgs(1),
}

func init() {
	CmdNotifier.AddCommand(cmdNotifierGet)
}

func notifierGet(_ *cobra.Command, args []string) error {
	name := args[0]

	cl.State.Fmt.Print("Retrieving notifier")

	conn, err := cl.State.Connect()
	if err != nil {
		cl.State.Fmt.PrintErr(err)
		cl.State.Fmt.Finish()
		return err
	}

	client := proto.NewGoferClient(conn)

	md := metadata.Pairs("Authorization", "Bearer "+cl.State.Config.Token)
	ctx := metadata.NewOutgoingContext(context.Background(), md)
	resp, err := client.GetNotifier(ctx, &proto.GetNotifierRequest{
		Name: name,
	})
	if err != nil {
		cl.State.Fmt.PrintErr(fmt.Sprintf("could not get notifier: %v", err))
		cl.State.Fmt.Finish()
		return err
	}

	cl.State.Fmt.Println(formatNotifierInfo(notifierInfo{
		Kind:          color.YellowString(resp.Notifier.Kind),
		Image:         resp.Notifier.Image,
		Documentation: resp.Notifier.Documentation,
	}))
	cl.State.Fmt.Finish()

	return nil
}

type notifierInfo struct {
	Kind          string
	Image         string
	Documentation string
}

func formatNotifierInfo(ti notifierInfo) string {
	const formatTmpl = `Notifier "{{.Kind}}"

Image {{.Image}}

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
