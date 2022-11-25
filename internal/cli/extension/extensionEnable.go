package extension

import (
	"context"
	"fmt"

	"github.com/clintjedwards/gofer/internal/cli/cl"
	proto "github.com/clintjedwards/gofer/proto/go"

	"github.com/spf13/cobra"
	"google.golang.org/grpc/metadata"
)

var cmdExtensionEnable = &cobra.Command{
	Use:     "enable <name>",
	Short:   "Enable a specific extension by name.",
	Example: `$ gofer extension enable cron`,
	RunE:    extensionEnable,
	Args:    cobra.ExactArgs(1),
}

func init() {
	CmdExtension.AddCommand(cmdExtensionEnable)
}

func extensionEnable(_ *cobra.Command, args []string) error {
	name := args[0]

	cl.State.Fmt.Print("Enabling extension")

	conn, err := cl.State.Connect()
	if err != nil {
		cl.State.Fmt.PrintErr(err)
		cl.State.Fmt.Finish()
		return err
	}

	client := proto.NewGoferClient(conn)

	md := metadata.Pairs("Authorization", "Bearer "+cl.State.Config.Token)
	ctx := metadata.NewOutgoingContext(context.Background(), md)
	_, err = client.EnableExtension(ctx, &proto.EnableExtensionRequest{
		Name: name,
	})
	if err != nil {
		cl.State.Fmt.PrintErr(fmt.Sprintf("could not enable extension: %v", err))
		cl.State.Fmt.Finish()
		return err
	}

	cl.State.Fmt.PrintSuccess("extension enabled")
	cl.State.Fmt.Finish()

	return nil
}
