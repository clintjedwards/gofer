package event

import (
	"context"
	"fmt"
	"io"
	"strings"

	"github.com/clintjedwards/gofer/internal/cli/cl"
	"github.com/clintjedwards/gofer/internal/cli/format"
	proto "github.com/clintjedwards/gofer/proto/go"
	"golang.org/x/text/cases"
	"golang.org/x/text/language"

	"github.com/fatih/color"
	"github.com/fatih/structs"
	"github.com/spf13/cobra"
	"google.golang.org/grpc/metadata"
)

var cmdEventList = &cobra.Command{
	Use:   "list",
	Short: "List all events",
	Long: `List all events.

Returns events from oldest to newest.`,
	Example: `$ gofer events list`,
	RunE:    eventList,
	Args:    cobra.ExactArgs(0),
}

func init() {
	cmdEventList.Flags().BoolP("reverse", "r", false, "Sort events from newest to oldest")
	cmdEventList.Flags().BoolP("follow", "f", false, "Continuously wait for more events; does not work with reverse")
	CmdEvent.AddCommand(cmdEventList)
}

type Event struct {
	Kind    string
	ID      int64
	Emitted int64
	Details string
}

func eventList(cmd *cobra.Command, _ []string) error {
	cl.State.Fmt.Print("Retrieving events")

	reverse, err := cmd.Flags().GetBool("reverse")
	if err != nil {
		return err
	}

	follow, err := cmd.Flags().GetBool("follow")
	if err != nil {
		return err
	}

	conn, err := cl.State.Connect()
	if err != nil {
		cl.State.Fmt.PrintErr(err)
		cl.State.Fmt.Finish()
		return err
	}

	client := proto.NewGoferClient(conn)

	md := metadata.Pairs("Authorization", "Bearer "+cl.State.Config.Token)
	ctx := metadata.NewOutgoingContext(context.Background(), md)
	stream, err := client.ListEvents(ctx, &proto.ListEventsRequest{
		Reverse: reverse,
		Follow:  follow,
	})
	if err != nil {
		cl.State.Fmt.PrintErr(fmt.Sprintf("could not list events: %v", err))
		cl.State.Fmt.Finish()
		return err
	}

	for {
		resp, err := stream.Recv()
		if err != nil {
			if err == io.EOF {
				break
			}
			fmt.Printf("could not get events: %v\n", err)
			cl.State.Fmt.PrintErr(err)
			break
		}

		event := constructEvent(resp.Event)

		cl.State.Fmt.Println(fmt.Sprintf("[%s] %s %s: %v",
			color.YellowString(fmt.Sprint(event.ID)),
			format.UnixMilli(event.Emitted, "Unknown", cl.State.Config.Detail),
			color.BlueString(formatEventKind(string(event.Kind))), event.Details))
	}

	cl.State.Fmt.Finish()

	return nil
}

func constructEvent(event *proto.Event) Event {
	// We're doing hacks here because I refuse to build long switch chains
	convertedMap := structs.Map(event)

	kind := convertedMap["Kind"].(string)
	id := convertedMap["Id"].(int64)
	emitted := convertedMap["Emitted"].(int64)
	details := convertedMap["Details"].(string)

	return Event{
		Kind:    kind,
		ID:      id,
		Emitted: emitted,
		Details: details,
	}
}

func formatEventKind(kind string) string {
	toTitle := cases.Title(language.AmericanEnglish)
	toLower := cases.Lower(language.AmericanEnglish)

	kind = strings.ReplaceAll(kind, "_", " ")
	kind = toTitle.String(toLower.String(kind))

	return kind
}
