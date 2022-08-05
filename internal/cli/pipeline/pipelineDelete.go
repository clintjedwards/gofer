package pipeline

import (
	"context"
	"fmt"
	"strings"

	"github.com/clintjedwards/gofer/internal/cli/cl"
	proto "github.com/clintjedwards/gofer/proto/go"

	"github.com/spf13/cobra"
	"google.golang.org/grpc/metadata"
)

var cmdPipelineDelete = &cobra.Command{
	Use:     "delete <id>",
	Short:   "Delete pipeline",
	Long:    `Delete a pipeline.`,
	Example: `$ gofer pipeline delete simple_test_pipeline`,
	RunE:    pipelineDelete,
	Args:    cobra.ExactArgs(1),
}

func init() {
	CmdPipeline.AddCommand(cmdPipelineDelete)
}

func pipelineDelete(_ *cobra.Command, args []string) error {
	id := args[0]

	cl.State.Fmt.Print("Deleting pipeline")
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
	_, err = client.DeletePipeline(ctx, &proto.DeletePipelineRequest{
		NamespaceId: cl.State.Config.Namespace,
		Id:          id,
	})
	if err != nil {
		cl.State.Fmt.PrintErr(fmt.Sprintf("could not delete pipeline: %v", err))
		cl.State.Fmt.Finish()
		return err
	}

	cl.State.Fmt.PrintSuccess(fmt.Sprintf("pipeline %q deleted", id))
	cl.State.Fmt.Finish()

	return nil
}
