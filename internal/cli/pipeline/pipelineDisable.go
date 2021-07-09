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

var cmdPipelineDisable = &cobra.Command{
	Use:   "disable <id>",
	Short: "Disable pipeline",
	Long: `Disable pipeline.

This will prevent the pipeline from running any more jobs and events passed to the pipeline
will be discarded.
`,
	Example: `$ gofer pipeline disable simple_test_pipeline`,
	RunE:    pipelineDisable,
	Args:    cobra.ExactArgs(1),
}

func init() {
	CmdPipeline.AddCommand(cmdPipelineDisable)
}

func pipelineDisable(_ *cobra.Command, args []string) error {
	id := args[0]

	cl.State.Fmt.Print("Disabling pipeline")
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
		return err
	}

	client := proto.NewGoferClient(conn)

	md := metadata.Pairs("Authorization", "Bearer "+cl.State.Config.Token)
	ctx := metadata.NewOutgoingContext(context.Background(), md)
	_, err = client.DisablePipeline(ctx, &proto.DisablePipelineRequest{
		NamespaceId: cl.State.Config.Namespace,
		Id:          id,
	})
	if err != nil {
		cl.State.Fmt.PrintErr(fmt.Sprintf("could not disable pipeline: %v", err))
		cl.State.Fmt.Finish()
		return err
	}

	cl.State.Fmt.Finish()

	return nil
}
