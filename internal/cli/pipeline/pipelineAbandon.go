package pipeline

import (
	"context"
	"fmt"
	"strings"

	"github.com/clintjedwards/gofer/internal/cli/cl"
	"github.com/clintjedwards/gofer/proto"
	"github.com/spf13/cobra"
	"google.golang.org/grpc/metadata"
)

var cmdPipelineAbandon = &cobra.Command{
	Use:   "abandon <id>",
	Short: "Abandon pipeline",
	Long: `Abandon a pipeline.

Abandoning a pipeline marks it for deletion and removes it from all lists. The pipeline may still be readable for a
short time.
`,
	Example: `$ gofer pipeline abandon simple_test_pipeline`,
	RunE:    pipelineAbandon,
	Args:    cobra.ExactArgs(1),
}

func init() {
	CmdPipeline.AddCommand(cmdPipelineAbandon)
}

func pipelineAbandon(_ *cobra.Command, args []string) error {
	id := args[0]

	cl.State.Fmt.Print("Abandoning pipeline")
	cl.State.Fmt.Finish()

	var input string

	for {
		fmt.Print("Please type the ID of the pipeline to confirm: ")
		fmt.Scanln(&input)
		if strings.EqualFold(input, id) {
			break
		}
	}

	cl.State.NewFormatter()

	conn, err := cl.State.Connect()
	if err != nil {
		cl.State.Fmt.PrintErr(err)
		cl.State.Fmt.Finish()
	}

	client := proto.NewGoferClient(conn)

	md := metadata.Pairs("Authorization", "Bearer "+cl.State.Config.Token)
	ctx := metadata.NewOutgoingContext(context.Background(), md)
	_, err = client.AbandonPipeline(ctx, &proto.AbandonPipelineRequest{
		NamespaceId: cl.State.Config.Namespace,
		Id:          id,
	})
	if err != nil {
		cl.State.Fmt.PrintErr(fmt.Sprintf("could not abandon pipeline: %v", err))
		cl.State.Fmt.Finish()
		return err
	}

	cl.State.Fmt.PrintSuccess(fmt.Sprintf("pipeline %s abandoned", id))
	cl.State.Fmt.Finish()

	return nil
}
