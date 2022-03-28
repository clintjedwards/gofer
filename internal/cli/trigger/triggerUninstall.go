package trigger

import (
	"context"
	"fmt"
	"strings"

	"github.com/clintjedwards/gofer/internal/cli/cl"
	"github.com/clintjedwards/gofer/proto"
	"github.com/spf13/cobra"
	"google.golang.org/grpc/metadata"
)

var cmdTriggerUninstall = &cobra.Command{
	Use:   "uninstall <kind>",
	Short: "Uninstall a specific trigger.",
	Long: "Uninstall a specific trigger.\n\n" +
		"Warning: uninstalling a specific trigger in use by pipelines will cause those pipelines to stop triggering",
	Example: `$ gofer trigger uninstall cron`,
	RunE:    triggerUninstall,
	Args:    cobra.ExactArgs(1),
}

func init() {
	CmdTrigger.AddCommand(cmdTriggerUninstall)
}

func triggerUninstall(_ *cobra.Command, args []string) error {
	kind := args[0]

	cl.State.Fmt.Print("Uninstalling trigger")
	cl.State.Fmt.Finish()

	var input string

	for {
		fmt.Printf("Please type trigger name %q to confirm: \n", kind)
		fmt.Scanln(&input)
		if strings.EqualFold(input, kind) {
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
	_, err = client.UninstallTrigger(ctx, &proto.UninstallTriggerRequest{
		Kind: kind,
	})
	if err != nil {
		cl.State.Fmt.PrintErr(fmt.Sprintf("could not uninstall trigger: %v", err))
		cl.State.Fmt.Finish()
		return err
	}

	cl.State.Fmt.PrintSuccess(fmt.Sprintf("successfully uninstalled trigger %q", kind))
	cl.State.Fmt.Finish()

	return nil
}
