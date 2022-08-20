package triggers

import (
	"context"
	"fmt"

	"github.com/clintjedwards/gofer/internal/cli/cl"
	proto "github.com/clintjedwards/gofer/proto/go"

	"github.com/spf13/cobra"
	"google.golang.org/grpc/metadata"
)

var cmdTriggersEnable = &cobra.Command{
	Use:     "enable <name>",
	Short:   "Enable a specific trigger by name.",
	Example: `$ gofer triggers enable cron`,
	RunE:    triggersEnable,
	Args:    cobra.ExactArgs(1),
}

func init() {
	CmdTriggers.AddCommand(cmdTriggersEnable)
}

func triggersEnable(_ *cobra.Command, args []string) error {
	name := args[0]

	cl.State.Fmt.Print("Enabling trigger")

	conn, err := cl.State.Connect()
	if err != nil {
		cl.State.Fmt.PrintErr(err)
		cl.State.Fmt.Finish()
		return err
	}

	client := proto.NewGoferClient(conn)

	md := metadata.Pairs("Authorization", "Bearer "+cl.State.Config.Token)
	ctx := metadata.NewOutgoingContext(context.Background(), md)
	_, err = client.EnableTrigger(ctx, &proto.EnableTriggerRequest{
		Name: name,
	})
	if err != nil {
		cl.State.Fmt.PrintErr(fmt.Sprintf("could not enable trigger: %v", err))
		cl.State.Fmt.Finish()
		return err
	}

	cl.State.Fmt.PrintSuccess("trigger enabled")
	cl.State.Fmt.Finish()

	return nil
}
