package token

import (
	"context"
	"fmt"

	"github.com/clintjedwards/gofer/internal/cli/cl"
	proto "github.com/clintjedwards/gofer/proto/go"

	"github.com/spf13/cobra"
	"google.golang.org/grpc/metadata"
)

var cmdTokenWhoami = &cobra.Command{
	Use:   "whoami",
	Short: "Get details about the token currently being used",
	RunE:  tokenWhoami,
}

func init() {
	CmdToken.AddCommand(cmdTokenWhoami)
}

func tokenWhoami(_ *cobra.Command, _ []string) error {
	cl.State.Fmt.Print("Retrieving token details")

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
		Token: cl.State.Config.Token,
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
