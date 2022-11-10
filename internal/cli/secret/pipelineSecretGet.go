package secret

import (
	"context"
	"fmt"

	"github.com/clintjedwards/gofer/internal/cli/cl"
	proto "github.com/clintjedwards/gofer/proto/go"

	"github.com/spf13/cobra"
	"google.golang.org/grpc/metadata"
)

var cmdPipelineSecretGet = &cobra.Command{
	Use:     "get <pipeline_id> <key>",
	Short:   "Read a secret from the pipeline secret store",
	Example: `$ gofer secret pipeline get simple_test_pipeline my_key`,
	RunE:    pipelineSecretGet,
	Args:    cobra.ExactArgs(2),
}

func init() {
	CmdPipelineSecret.AddCommand(cmdPipelineSecretGet)
}

func pipelineSecretGet(_ *cobra.Command, args []string) error {
	// We don't use the formatter here because we may want to redirect the object we get into
	// a file or similar situation.
	cl.State.Fmt.Finish()
	pipelineID := args[0]
	key := args[1]

	conn, err := cl.State.Connect()
	if err != nil {
		fmt.Println(err)
		return err
	}

	client := proto.NewGoferClient(conn)

	md := metadata.Pairs("Authorization", "Bearer "+cl.State.Config.Token)
	ctx := metadata.NewOutgoingContext(context.Background(), md)
	resp, err := client.GetPipelineSecret(ctx, &proto.GetPipelineSecretRequest{
		NamespaceId:   cl.State.Config.Namespace,
		PipelineId:    pipelineID,
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
