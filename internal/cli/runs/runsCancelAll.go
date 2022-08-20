package runs

import (
	"context"
	"fmt"

	"github.com/clintjedwards/gofer/internal/cli/cl"
	proto "github.com/clintjedwards/gofer/proto/go"

	"github.com/spf13/cobra"
	"google.golang.org/grpc/metadata"
)

var cmdRunsCancelAll = &cobra.Command{
	Use:     "cancel-all <pipeline_id>",
	Short:   "CancelAll cancels all run for a given pipeline",
	Example: `$ gofer runs cancel-all simple_test_pipeline`,
	RunE:    runsCancelAll,
	Args:    cobra.ExactArgs(1),
}

func init() {
	cmdRunsCancelAll.Flags().BoolP("force", "f", false, "Stop run and child taskrun containers immediately (SIGKILL)")
	CmdRuns.AddCommand(cmdRunsCancelAll)
}

func runsCancelAll(cmd *cobra.Command, args []string) error {
	pipelineID := args[0]

	force, err := cmd.Flags().GetBool("force")
	if err != nil {
		fmt.Println(err)
		return err
	}

	cl.State.Fmt.Print("Cancelling all runs")

	conn, err := cl.State.Connect()
	if err != nil {
		cl.State.Fmt.PrintErr(err)
		cl.State.Fmt.Finish()
		return err
	}

	client := proto.NewGoferClient(conn)

	md := metadata.Pairs("Authorization", "Bearer "+cl.State.Config.Token)
	ctx := metadata.NewOutgoingContext(context.Background(), md)
	resp, err := client.CancelAllRuns(ctx, &proto.CancelAllRunsRequest{
		NamespaceId: cl.State.Config.Namespace,
		PipelineId:  pipelineID,
		Force:       force,
	})
	if err != nil {
		cl.State.Fmt.PrintErr(fmt.Sprintf("could not cancel runs: %v", err))
		cl.State.Fmt.Finish()
		return err
	}

	cl.State.Fmt.PrintSuccess(fmt.Sprintf("cancelled all in-progress runs %v", resp.Runs))
	cl.State.Fmt.Finish()

	return nil
}
