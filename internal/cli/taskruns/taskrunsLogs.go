package taskruns

import (
	"context"
	"fmt"
	"io"
	"strconv"

	"github.com/clintjedwards/gofer/internal/cli/cl"
	proto "github.com/clintjedwards/gofer/proto/go"

	"github.com/spf13/cobra"
)

var cmdTaskRunsLogs = &cobra.Command{
	Use:     "logs <pipeline> <run> <id>",
	Short:   "Examine logs for a particular taskrun/container",
	Example: `$ gofer taskruns logs simple_test_pipeline 23 example_task`,
	RunE:    taskrunLogs,
	Args:    cobra.ExactArgs(3),
}

func init() {
	CmdTaskRuns.AddCommand(cmdTaskRunsLogs)
}

func taskrunLogs(_ *cobra.Command, args []string) error {
	// We don't use the formatter here because we may want to redirect logs we get into
	// a file or such.
	cl.State.Fmt.Finish()

	pipeline := args[0]

	runIDRaw := args[1]
	runID, err := strconv.Atoi(runIDRaw)
	if err != nil {
		return err
	}

	id := args[2]

	conn, err := cl.State.Connect()
	if err != nil {
		fmt.Println(err)
		return err
	}
	defer conn.Close()

	client := proto.NewGoferClient(conn)

	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	stream, err := client.GetTaskRunLogs(ctx, &proto.GetTaskRunLogsRequest{
		NamespaceId: cl.State.Config.Namespace,
		RunId:       int64(runID),
		PipelineId:  pipeline,
		Id:          id,
	})
	if err != nil {
		fmt.Printf("could not get logs for task run: %v\n", err)
		return err
	}

	for {
		resp, err := stream.Recv()
		if err != nil {
			if err == io.EOF {
				break
			}
			fmt.Printf("could not get logs: %v\n", err)
			return nil
		}

		fmt.Println(resp.LogLine)
	}

	return nil
}
