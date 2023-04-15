package extension

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

var cmdExtensionGet = &cobra.Command{
	Use:     "get <name>",
	Short:   "Get a specific extension by name.",
	Example: `$ gofer extension get cron`,
	RunE:    extensionGet,
	Args:    cobra.ExactArgs(1),
}

func init() {
	CmdExtension.AddCommand(cmdExtensionGet)
}

func extensionGet(_ *cobra.Command, args []string) error {
	name := args[0]

	cl.State.Fmt.Print("Retrieving extension")

	conn, err := cl.State.Connect()
	if err != nil {
		cl.State.Fmt.PrintErr(err)
		cl.State.Fmt.Finish()
		return err
	}

	client := proto.NewGoferClient(conn)

	md := metadata.Pairs("Authorization", "Bearer "+cl.State.Config.Token)
	ctx := metadata.NewOutgoingContext(context.Background(), md)
	resp, err := client.GetExtension(ctx, &proto.GetExtensionRequest{
		Name: name,
	})
	if err != nil {
		cl.State.Fmt.PrintErr(fmt.Sprintf("could not get extension: %v", err))
		cl.State.Fmt.Finish()
		return err
	}

	cl.State.Fmt.Println(formatExtensionInfo(extensionInfo{
		Name:          color.YellowString(resp.Extension.Name),
		State:         cliformat.ColorizeExtensionState(cliformat.NormalizeEnumValue(resp.Extension.State.String(), "Unknown")),
		Status:        cliformat.ColorizeExtensionStatus(cliformat.NormalizeEnumValue(resp.Extension.Status.String(), "Unknown")),
		Started:       cliformat.UnixMilli(resp.Extension.Started, "Not Started", cl.State.Config.Detail),
		URL:           resp.Extension.Url,
		Documentation: resp.Extension.Documentation,
	}))
	cl.State.Fmt.Finish()

	return nil
}

type extensionInfo struct {
	Name          string
	URL           string
	Started       string
	State         string
	Status        string
	Documentation string
}

func formatExtensionInfo(ti extensionInfo) string {
	const formatTmpl = `Extension "{{.Name}}" :: {{.Status}} :: {{.State}}

Started {{.Started}}

Endpoint: {{.URL}}

{{- if .Documentation }}

Documentation:

{{.Documentation}}
{{- else}}

No Documentation found
{{- end}}`

	var tpl bytes.Buffer
	t := template.Must(template.New("tmp").Parse(formatTmpl))
	_ = t.Execute(&tpl, ti)
	return tpl.String()
}
