package event

import (
	"bytes"
	"context"
	"fmt"
	"strconv"
	"text/template"

	"github.com/clintjedwards/gofer/internal/cli/cl"
	"github.com/clintjedwards/gofer/internal/cli/format"
	"github.com/clintjedwards/gofer/proto"
	"github.com/fatih/color"
	"github.com/fatih/structs"
	"github.com/spf13/cobra"
	"google.golang.org/grpc/metadata"
)

var cmdEventGet = &cobra.Command{
	Use:     "get <id>",
	Short:   "Get details on a specific event",
	Example: `$ gofer event get Abdedow8953`,
	RunE:    eventGet,
	Args:    cobra.ExactArgs(1),
}

func init() {
	CmdEvent.AddCommand(cmdEventGet)
}

func eventGet(_ *cobra.Command, args []string) error {
	id, err := strconv.Atoi(args[0])
	if err != nil {
		cl.State.Fmt.PrintErr(err)
		cl.State.Fmt.Finish()
	}

	cl.State.Fmt.Print("Retrieving event")

	conn, err := cl.State.Connect()
	if err != nil {
		cl.State.Fmt.PrintErr(err)
		cl.State.Fmt.Finish()
		return err
	}

	client := proto.NewGoferClient(conn)

	md := metadata.Pairs("Authorization", "Bearer "+cl.State.Config.Token)
	ctx := metadata.NewOutgoingContext(context.Background(), md)
	resp, err := client.GetEvent(ctx, &proto.GetEventRequest{
		Id: int64(id),
	})
	if err != nil {
		cl.State.Fmt.PrintErr(fmt.Sprintf("could not get taskrun: %v", err))
		cl.State.Fmt.Finish()
		return err
	}

	output, err := formatEvent(resp, cl.State.Config.Detail)
	if err != nil {
		cl.State.Fmt.PrintErr(fmt.Sprintf("could not render event: %v", err))
		cl.State.Fmt.Finish()
		return err
	}

	cl.State.Fmt.Println(output)
	cl.State.Fmt.Finish()

	return nil
}

type data struct {
	Kind    string
	Emitted string
	ID      string
	Event   string
	Other   map[string]string
}

func formatEvent(event *proto.GetEventResponse, detail bool) (string, error) {
	// We're doing hacks here because I refuse to build long switch chains
	convertedMap := structs.Map(event.Event)
	kind := ""
	metadata := map[string]interface{}{}
	rawMap := map[string]interface{}{}
	for key, value := range convertedMap {
		rawMap = value.(map[string]interface{})
		metadata = rawMap["Metadata"].(map[string]interface{})
		kind = key
		break // We only interate through this map so we can take the first (and only) key
	}

	other := map[string]string{}
	for key, value := range rawMap {
		if key == "Metadata" {
			continue
		}

		other[color.BlueString("%s:", key)] = fmt.Sprint(value)
	}

	data := data{
		Kind:    color.BlueString(kind),
		Emitted: format.UnixMilli(metadata["Emitted"].(int64), "Unknown", detail),
		ID:      fmt.Sprintf("%d", metadata["EventId"].(int64)),
		Other:   other,
	}

	const formatTmpl = `[{{.ID}}] {{.Kind}}

🗒 Details:
  {{- range $key, $value := .Other }}
  • {{ $key }} {{ $value }}
  {{- end }}

Emitted {{.Emitted}}`

	var tpl bytes.Buffer
	t := template.Must(template.New("tmp").Parse(formatTmpl))
	_ = t.Execute(&tpl, data)
	return tpl.String(), nil
}
