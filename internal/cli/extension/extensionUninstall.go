package extension

import (
	"context"
	"fmt"

	"github.com/clintjedwards/gofer/internal/cli/cl"
	proto "github.com/clintjedwards/gofer/proto/go"

	"github.com/spf13/cobra"
	"google.golang.org/grpc/metadata"
)

var cmdExtensionUninstall = &cobra.Command{
	Use:     "uninstall <name>",
	Short:   "Uninstall a specific extension by name.",
	Long:    `Uninstall a specific extension by name.`,
	Example: `$ gofer extension uninstall cron`,
	RunE:    extensionUninstall,
	Args:    cobra.ExactArgs(1),
}

func init() {
	CmdExtension.AddCommand(cmdExtensionUninstall)
}

func extensionUninstall(_ *cobra.Command, args []string) error {
	name := args[0]

	cl.State.Fmt.Print("Uninstalling extension")

	conn, err := cl.State.Connect()
	if err != nil {
		cl.State.Fmt.PrintErr(err)
		cl.State.Fmt.Finish()
		return err
	}

	client := proto.NewGoferClient(conn)
	md := metadata.Pairs("Authorization", "Bearer "+cl.State.Config.Token)
	ctx := metadata.NewOutgoingContext(context.Background(), md)
	_, err = client.UninstallExtension(ctx, &proto.UninstallExtensionRequest{
		Name: name,
	})
	if err != nil {
		cl.State.Fmt.PrintErr(fmt.Sprintf("could not uninstall extension: %v", err))
		cl.State.Fmt.Finish()
		return err
	}

	cl.State.Fmt.PrintSuccess("Extension Uninstalled!")
	cl.State.Fmt.Finish()

	return nil
}
