package tokens

import (
	"context"
	"fmt"

	"github.com/clintjedwards/gofer/internal/cli/cl"
	proto "github.com/clintjedwards/gofer/proto/go"

	"github.com/spf13/cobra"
	"google.golang.org/grpc/metadata"
)

var cmdTokensDisable = &cobra.Command{
	Use:   "disable",
	Short: "Disable specific token",
	RunE:  tokensDisable,
}

func init() {
	CmdTokens.AddCommand(cmdTokensDisable)
}

func tokensDisable(_ *cobra.Command, _ []string) error {
	cl.State.Fmt.Print("Disabling token")
	cl.State.Fmt.Finish()

	var input string

	fmt.Print("Please paste the token to disable: ")
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
	_, err = client.DisableToken(ctx, &proto.DisableTokenRequest{
		Token: input,
	})
	if err != nil {
		cl.State.Fmt.PrintErr(fmt.Sprintf("could not disable token: %v", err))
		cl.State.Fmt.Finish()
		return err
	}

	cl.State.Fmt.PrintSuccess("Token Disabled")
	cl.State.Fmt.Finish()

	return nil
}
