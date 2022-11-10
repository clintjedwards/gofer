package trigger

import (
	"context"
	"fmt"

	"github.com/clintjedwards/gofer/internal/cli/cl"
	proto "github.com/clintjedwards/gofer/proto/go"

	"github.com/spf13/cobra"
	"google.golang.org/grpc/metadata"
)

var cmdTriggerUninstall = &cobra.Command{
	Use:     "uninstall <name>",
	Short:   "Uninstall a specific trigger by name.",
	Long:    `Uninstall a specific trigger by name.`,
	Example: `$ gofer trigger uninstall cron`,
	RunE:    triggerUninstall,
	Args:    cobra.ExactArgs(1),
}

func init() {
	CmdTrigger.AddCommand(cmdTriggerUninstall)
}

func triggerUninstall(_ *cobra.Command, args []string) error {
	name := args[0]

	cl.State.Fmt.Print("Uninstalling trigger")

	conn, err := cl.State.Connect()
	if err != nil {
		cl.State.Fmt.PrintErr(err)
		cl.State.Fmt.Finish()
		return err
	}

	client := proto.NewGoferClient(conn)
	md := metadata.Pairs("Authorization", "Bearer "+cl.State.Config.Token)
	ctx := metadata.NewOutgoingContext(context.Background(), md)
	_, err = client.UninstallTrigger(ctx, &proto.UninstallTriggerRequest{
		Name: name,
	})
	if err != nil {
		cl.State.Fmt.PrintErr(fmt.Sprintf("could not uninstall trigger: %v", err))
		cl.State.Fmt.Finish()
		return err
	}

	cl.State.Fmt.PrintSuccess("Trigger Uninstalled!")
	cl.State.Fmt.Finish()

	return nil
}
