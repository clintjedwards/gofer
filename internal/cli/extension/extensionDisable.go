package extension

import (
	"context"
	"fmt"
	"strings"

	"github.com/clintjedwards/gofer/internal/cli/cl"
	proto "github.com/clintjedwards/gofer/proto/go"

	"github.com/spf13/cobra"
	"google.golang.org/grpc/metadata"
)

var cmdExtensionDisable = &cobra.Command{
	Use:     "disable <name>",
	Short:   "Disable a specific extension by name.",
	Example: `$ gofer extension disable cron`,
	RunE:    extensionDisable,
	Args:    cobra.ExactArgs(1),
}

func init() {
	CmdExtension.AddCommand(cmdExtensionDisable)
}

func extensionDisable(_ *cobra.Command, args []string) error {
	name := args[0]

	cl.State.Fmt.Print("Disabling extension")
	cl.State.Fmt.Finish()

	var input string

	for {
		fmt.Println("It is important to note that disabling a extension will stop all extension events and pipelines that " +
			"depend on this extension will no longer run.")
		fmt.Print("Please type the ID of the extension to confirm: ")
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
	_, err = client.DisableExtension(ctx, &proto.DisableExtensionRequest{
		Name: name,
	})
	if err != nil {
		cl.State.Fmt.PrintErr(fmt.Sprintf("could not disable extension: %v", err))
		cl.State.Fmt.Finish()
		return err
	}

	cl.State.Fmt.PrintSuccess("Extension disabled")
	cl.State.Fmt.Finish()

	return nil
}
