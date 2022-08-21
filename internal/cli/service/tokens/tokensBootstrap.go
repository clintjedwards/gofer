package tokens

import (
	"context"
	"fmt"

	"github.com/clintjedwards/gofer/internal/cli/cl"
	proto "github.com/clintjedwards/gofer/proto/go"

	"github.com/spf13/cobra"
)

var cmdTokensBootstrap = &cobra.Command{
	Use:   "bootstrap",
	Short: "Bootstrap creates the initial management token",
	RunE:  tokensBootstrap,
}

func init() {
	CmdTokens.AddCommand(cmdTokensBootstrap)
}

func tokensBootstrap(_ *cobra.Command, _ []string) error {
	cl.State.Fmt.Print("Creating Token")

	conn, err := cl.State.Connect()
	if err != nil {
		cl.State.Fmt.PrintErr(err)
		cl.State.Fmt.Finish()
		return err
	}

	client := proto.NewGoferClient(conn)

	resp, err := client.BootstrapToken(context.Background(), &proto.BootstrapTokenRequest{})
	if err != nil {
		cl.State.Fmt.PrintErr(fmt.Sprintf("could not get token: %v", err))
		cl.State.Fmt.Finish()
		return err
	}

	cl.State.Fmt.PrintSuccess(fmt.Sprintf("Token: %s", resp.Token))
	cl.State.Fmt.Finish()

	return nil
}
