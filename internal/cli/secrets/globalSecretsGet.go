package secrets

import (
	"context"
	"fmt"

	"github.com/clintjedwards/gofer/internal/cli/cl"
	proto "github.com/clintjedwards/gofer/proto/go"

	"github.com/spf13/cobra"
	"google.golang.org/grpc/metadata"
)

var cmdGlobalSecretsGet = &cobra.Command{
	Use:     "get <key>",
	Short:   "Read a secret from the global secret store",
	Example: `$ gofer global secrets get simple_test_global my_key`,
	RunE:    globalSecretsGet,
	Args:    cobra.ExactArgs(1),
}

func init() {
	CmdGlobalSecrets.AddCommand(cmdGlobalSecretsGet)
}

func globalSecretsGet(_ *cobra.Command, args []string) error {
	// We don't use the formatter here because we may want to redirect the object we get into
	// a file or similar situation.
	cl.State.Fmt.Finish()
	key := args[0]

	conn, err := cl.State.Connect()
	if err != nil {
		fmt.Println(err)
		return err
	}

	client := proto.NewGoferClient(conn)

	md := metadata.Pairs("Authorization", "Bearer "+cl.State.Config.Token)
	ctx := metadata.NewOutgoingContext(context.Background(), md)
	resp, err := client.GetGlobalSecret(ctx, &proto.GetGlobalSecretRequest{
		Key:           key,
		IncludeSecret: true,
	})
	if err != nil {
		fmt.Printf("could not read object: %v\n", err)
		return err
	}

	fmt.Printf(resp.Secret)

	return nil
}
