package event

import (
	"context"
	"fmt"
	"io"

	"github.com/clintjedwards/gofer/internal/cli/cl"
	"github.com/clintjedwards/gofer/proto"
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

		cl.State.Fmt.Println(resp)
	}

	cl.State.Fmt.Finish()

	return nil
}
