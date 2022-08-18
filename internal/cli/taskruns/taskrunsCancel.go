package taskruns

import (
	"context"
	"fmt"
	"strconv"

	"github.com/clintjedwards/gofer/internal/cli/cl"
	proto "github.com/clintjedwards/gofer/proto/go"

	"github.com/spf13/cobra"
	"google.golang.org/grpc/metadata"
)

var cmdTaskRunsCancel = &cobra.Command{
	Use:   "cancel <pipeline> <run> <id>",
	Short: "Cancel a specific task run",
	Long: `Cancels a task run by requesting that the scheduler gracefully stops it. Usually this means the scheduler will
pass a SIGTERM to the container. If the container does not shut down within the API defined timeout or the user has passed
the force flag the scheduler will then kill the container immediately.

Cancelling a task run might mean that downstream/dependent task runs are skipped.`,
	Example: `$ gofer taskruns cancel simple_test_pipeline 23 example_task`,
	RunE:    taskrunsCancel,
	Args:    cobra.ExactArgs(3),
}

func init() {
	cmdTaskRunsCancel.Flags().BoolP("force", "f", false, "Stop job immediately(sigkill/ungraceful shutdown)")
	CmdTaskRuns.AddCommand(cmdTaskRunsCancel)
}

func taskrunsCancel(cmd *cobra.Command, args []string) error {
	pipeline := args[0]

	runIDRaw := args[1]
	runID, err := strconv.Atoi(runIDRaw)
	if err != nil {
		return err
	}

	id := args[2]

	force, err := cmd.Flags().GetBool("force")
	if err != nil {
		fmt.Println(err)
		return err
	}

	cl.State.Fmt.Print("Cancelling taskrun")

	conn, err := cl.State.Connect()
	if err != nil {
		cl.State.Fmt.PrintErr(err)
		cl.State.Fmt.Finish()
		return err
	}

	client := proto.NewGoferClient(conn)

	md := metadata.Pairs("Authorization", "Bearer "+cl.State.Config.Token)
	ctx := metadata.NewOutgoingContext(context.Background(), md)
	_, err = client.CancelTaskRun(ctx, &proto.CancelTaskRunRequest{
		NamespaceId: cl.State.Config.Namespace,
		RunId:       int64(runID),
		PipelineId:  pipeline,
		Id:          id,
		Force:       force,
	})
	if err != nil {
		cl.State.Fmt.PrintErr(fmt.Sprintf("could not cancel taskrun: %v", err))
		cl.State.Fmt.Finish()
		return err
	}

	cl.State.Fmt.PrintSuccess("Cancelled Task Run")
	cl.State.Fmt.Finish()

	return nil
}
