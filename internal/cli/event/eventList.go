package event

import (
	"context"
	"encoding/json"
	"fmt"
	"io"

	"github.com/clintjedwards/gofer/internal/cli/cl"
	proto "github.com/clintjedwards/gofer/proto/go"

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

		cl.State.Fmt.Println(printEvent(resp))
	}

	cl.State.Fmt.Finish()

	return nil
}

func printEvent(event *proto.ListEventsResponse) string {
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

		other[key] = fmt.Sprint(value)
	}

	otherRaw, _ := json.Marshal(other)
	id := metadata["EventId"].(int64)

	return fmt.Sprintf("[%s] %s: %v", color.YellowString(fmt.Sprint(id)), color.BlueString(kind), string(otherRaw))
}
