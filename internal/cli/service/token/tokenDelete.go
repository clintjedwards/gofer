package token

import (
	"context"
	"fmt"

	"github.com/clintjedwards/gofer/internal/cli/cl"
	"github.com/clintjedwards/gofer/proto"
	"github.com/spf13/cobra"
	"google.golang.org/grpc/metadata"
)

var cmdTokenDelete = &cobra.Command{
	Use:   "delete",
	Short: "Delete specific token",
	RunE:  tokenDelete,
}

func init() {
	CmdToken.AddCommand(cmdTokenDelete)
}

func tokenDelete(_ *cobra.Command, _ []string) error {
	cl.State.Fmt.Print("Deleting token")
	cl.State.Fmt.Finish()

	var input string

	fmt.Print("Please paste the token to delete: ")
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
	_, err = client.DeleteToken(ctx, &proto.DeleteTokenRequest{
		Token: input,
	})
	if err != nil {
		cl.State.Fmt.PrintErr(fmt.Sprintf("could not delete token: %v", err))
		cl.State.Fmt.Finish()
		return err
	}

	cl.State.Fmt.PrintSuccess("Token Deleted")
	cl.State.Fmt.Finish()

	return nil
}
