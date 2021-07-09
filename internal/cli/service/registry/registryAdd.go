package registry

import (
	"context"
	"fmt"

	"github.com/clintjedwards/gofer/internal/cli/cl"
	"github.com/clintjedwards/gofer/proto"
	"github.com/spf13/cobra"
	"google.golang.org/grpc/metadata"
)

var cmdRegistryAdd = &cobra.Command{
	Use:     "add <registry> <user>",
	Short:   "create a new docker registry auth",
	Example: "$ gofer service ghcr.io/clintjedwards/gofer my-user",
	RunE:    registryAdd,
	Args:    cobra.ExactArgs(2),
}

func init() {
	CmdRegistry.AddCommand(cmdRegistryAdd)
}

func registryAdd(_ *cobra.Command, args []string) error {
	cl.State.Fmt.Print("Creating Registry Auth")
	cl.State.Fmt.Finish()

	registry := args[0]
	user := args[1]

	var input string

	fmt.Print("Please input registry pass: ")
	fmt.Scanln(&input)

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
	_, err = client.AddRegistryAuth(ctx, &proto.AddRegistryAuthRequest{
		Registry: registry,
		User:     user,
		Pass:     input,
	})
	if err != nil {
		cl.State.Fmt.PrintErr(fmt.Sprintf("could not add registry auth: %v", err))
		cl.State.Fmt.Finish()
		return err
	}

	cl.State.Fmt.PrintSuccess("Docker registry auth added")
	cl.State.Fmt.Finish()

	return nil
}
