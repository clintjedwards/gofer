package registry

import (
	"context"
	"fmt"

	"github.com/clintjedwards/gofer/internal/cli/cl"
	"github.com/clintjedwards/gofer/proto"
	"github.com/spf13/cobra"
	"google.golang.org/grpc/metadata"
)

var cmdRegistryList = &cobra.Command{
	Use:   "list",
	Short: "List all Docker registry auths",
	RunE:  registryList,
}

func init() {
	CmdRegistry.AddCommand(cmdRegistryList)
}

func registryList(_ *cobra.Command, _ []string) error {
	cl.State.Fmt.Print("Listing Docker registry auths")

	conn, err := cl.State.Connect()
	if err != nil {
		cl.State.Fmt.PrintErr(err)
		cl.State.Fmt.Finish()
		return err
	}

	client := proto.NewGoferClient(conn)

	md := metadata.Pairs("Authorization", "Bearer "+cl.State.Config.Token)
	ctx := metadata.NewOutgoingContext(context.Background(), md)
	resp, err := client.ListRegistryAuths(ctx, &proto.ListRegistryAuthsRequest{})
	if err != nil {
		cl.State.Fmt.PrintErr(fmt.Sprintf("could not get token: %v", err))
		cl.State.Fmt.Finish()
		return err
	}

	cl.State.Fmt.Println(resp.Auths)
	cl.State.Fmt.Finish()

	return nil
}
