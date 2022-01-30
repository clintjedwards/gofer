package event

import (
	"context"
	"fmt"
	"strconv"

	"github.com/clintjedwards/gofer/internal/cli/cl"
	"github.com/clintjedwards/gofer/proto"
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

	cl.State.Fmt.Println(resp)
	cl.State.Fmt.Finish()

	return nil
}
