package token

import (
	"context"
	"fmt"

	"github.com/clintjedwards/gofer/internal/cli/cl"
	proto "github.com/clintjedwards/gofer/proto/go"

	"github.com/spf13/cobra"
	"google.golang.org/grpc/metadata"
)

var cmdTokenGet = &cobra.Command{
	Use:   "get",
	Short: "Get details on specific token",
	RunE:  tokenGet,
}

func init() {
	CmdToken.AddCommand(cmdTokenGet)
}

func tokenGet(_ *cobra.Command, _ []string) error {
	cl.State.Fmt.Print("Retrieving token details")
	cl.State.Fmt.Finish()

	var input string

	fmt.Print("Please paste the token to retrieve: ")
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
	resp, err := client.GetToken(ctx, &proto.GetTokenRequest{
		Token: input,
	})
	if err != nil {
		cl.State.Fmt.PrintErr(fmt.Sprintf("could not get token: %v", err))
		cl.State.Fmt.Finish()
		return err
	}

	cl.State.Fmt.Println(resp.Details)
	cl.State.Fmt.Finish()

	return nil
}
