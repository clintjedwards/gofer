package notifier

import (
	"context"
	"fmt"
	"strings"

	"github.com/clintjedwards/gofer/internal/cli/cl"
	"github.com/clintjedwards/gofer/proto"
	"github.com/spf13/cobra"
	"google.golang.org/grpc/metadata"
)

var cmdNotifierUninstall = &cobra.Command{
	Use:   "uninstall <kind>",
	Short: "Uninstall a specific notifier.",
	Long: "Uninstall a specific notifier.\n\n" +
		"Warning: uninstalling a specific notifier in use by pipelines will cause those pipelines to stop notifiering",
	Example: `$ gofer notifier uninstall cron`,
	RunE:    notifierUninstall,
	Args:    cobra.ExactArgs(1),
}

func init() {
	CmdNotifier.AddCommand(cmdNotifierUninstall)
}

func notifierUninstall(_ *cobra.Command, args []string) error {
	kind := args[0]

	cl.State.Fmt.Print("Uninstalling notifier")
	cl.State.Fmt.Finish()

	var input string

	for {
		fmt.Printf("Please type notifier name %q to confirm: \n", kind)
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
	_, err = client.UninstallNotifier(ctx, &proto.UninstallNotifierRequest{
		Kind: kind,
	})
	if err != nil {
		cl.State.Fmt.PrintErr(fmt.Sprintf("could not uninstall notifier: %v", err))
		cl.State.Fmt.Finish()
		return err
	}

	cl.State.Fmt.PrintSuccess(fmt.Sprintf("successfully uninstalled notifier %q", kind))
	cl.State.Fmt.Finish()

	return nil
}
