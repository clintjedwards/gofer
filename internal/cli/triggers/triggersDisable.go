package triggers

import (
	"context"
	"fmt"
	"strings"

	"github.com/clintjedwards/gofer/internal/cli/cl"
	proto "github.com/clintjedwards/gofer/proto/go"

	"github.com/spf13/cobra"
	"google.golang.org/grpc/metadata"
)

var cmdTriggersDisable = &cobra.Command{
	Use:     "disable <name>",
	Short:   "Disable a specific trigger by name.",
	Example: `$ gofer triggers disable cron`,
	RunE:    triggersDisable,
	Args:    cobra.ExactArgs(1),
}

func init() {
	CmdTriggers.AddCommand(cmdTriggersDisable)
}

func triggersDisable(_ *cobra.Command, args []string) error {
	name := args[0]

	cl.State.Fmt.Print("Disabling trigger")
	cl.State.Fmt.Finish()

	var input string

	for {
		fmt.Println("It is important to note that disabling a trigger will stop all trigger events and pipelines that " +
			"depend on this trigger will no longer run.")
		fmt.Print("Please type the ID of the trigger to confirm: ")
		fmt.Scanln(&input)
		if strings.EqualFold(input, name) {
			break
		}
	}

	cl.State.NewFormatter()

	conn, err := cl.State.Connect()
	if err != nil {
		cl.State.Fmt.PrintErr(err)
		cl.State.Fmt.Finish()
		return err
	}

	client := proto.NewGoferClient(conn)

	md := metadata.Pairs("Authorization", "Bearer "+cl.State.Config.Token)
	ctx := metadata.NewOutgoingContext(context.Background(), md)
	_, err = client.DisableTrigger(ctx, &proto.DisableTriggerRequest{
		Name: name,
	})
	if err != nil {
		cl.State.Fmt.PrintErr(fmt.Sprintf("could not disable trigger: %v", err))
		cl.State.Fmt.Finish()
		return err
	}

	cl.State.Fmt.PrintSuccess("Trigger disabled")
	cl.State.Fmt.Finish()

	return nil
}
