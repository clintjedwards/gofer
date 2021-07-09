package registry

import (
	"context"
	"fmt"

	"github.com/clintjedwards/gofer/internal/cli/cl"
	"github.com/clintjedwards/gofer/proto"
	"github.com/spf13/cobra"
	"google.golang.org/grpc/metadata"
)

var cmdRegistryRemove = &cobra.Command{
	Use:   "remove <registry>",
	Short: "Remove Docker registry auth",
	RunE:  registryRemove,
}

func init() {
	CmdRegistry.AddCommand(cmdRegistryRemove)
}

func registryRemove(_ *cobra.Command, args []string) error {
	cl.State.Fmt.Print("Remove Docker registry auth")

	registry := args[0]

	conn, err := cl.State.Connect()
	if err != nil {
		cl.State.Fmt.PrintErr(err)
		cl.State.Fmt.Finish()
		return err
	}

	client := proto.NewGoferClient(conn)

	md := metadata.Pairs("Authorization", "Bearer "+cl.State.Config.Token)
	ctx := metadata.NewOutgoingContext(context.Background(), md)
	_, err = client.RemoveRegistryAuth(ctx, &proto.RemoveRegistryAuthRequest{
		Registry: registry,
	})
	if err != nil {
		cl.State.Fmt.PrintErr(fmt.Sprintf("could not remove docker registry: %v", err))
		cl.State.Fmt.Finish()
		return err
	}

	cl.State.Fmt.PrintSuccess("Docker registry deleted")
	cl.State.Fmt.Finish()

	return nil
}
