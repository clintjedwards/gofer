package event

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"strconv"
	"text/template"

	"github.com/clintjedwards/gofer/internal/cli/cl"
	"github.com/clintjedwards/gofer/internal/cli/format"
	proto "github.com/clintjedwards/gofer/proto/go"
	"github.com/fatih/color"

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
	Details map[string]interface{}
}

func formatEvent(event *proto.GetEventResponse, detail bool) (string, error) {
	evnt := Event{
		Type:    event.Event.Type,
		ID:      event.Event.Id,
		Emitted: event.Event.Emitted,
		Details: event.Event.Details,
	}

	details := map[string]interface{}{}
	err := json.Unmarshal([]byte(evnt.Details), &details)
	if err != nil {
		return "", err
	}

	fmttedDetails := map[string]interface{}{}
	for key, value := range details {
		fmttedDetails[color.CyanString(formatEventKind(key))] = value
	}

	data := data{
		Kind:    color.BlueString(formatEventKind(evnt.Type)),
		Emitted: format.UnixMilli(evnt.Emitted, "Unknown", detail),
		ID:      color.YellowString(fmt.Sprintf("%d", evnt.ID)),
		Details: fmttedDetails,
	}

	const formatTmpl = `[{{.ID}}] {{.Kind}}

  🗒 Details:
    {{- range $key, $value := .Details }}
    • {{ $key }}: {{ $value }}
    {{- end }}

  Emitted {{.Emitted}}`

	var tpl bytes.Buffer
	t := template.Must(template.New("tmp").Parse(formatTmpl))
	_ = t.Execute(&tpl, data)
	return tpl.String(), nil
}
