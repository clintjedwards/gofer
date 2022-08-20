package commonTasks

import (
	"context"
	"fmt"

	"github.com/clintjedwards/gofer/internal/cli/cl"
	proto "github.com/clintjedwards/gofer/proto/go"

	"github.com/spf13/cobra"
	"google.golang.org/grpc/metadata"
)

var cmdCommonTasksEnable = &cobra.Command{
	Use:     "enable <name>",
	Short:   "Enable a specific common task by name.",
	Example: `$ gofer common-tasks enable cron`,
	RunE:    commonTasksEnable,
	Args:    cobra.ExactArgs(1),
}

func init() {
	CmdCommonTasks.AddCommand(cmdCommonTasksEnable)
}

func commonTasksEnable(_ *cobra.Command, args []string) error {
	name := args[0]

	cl.State.Fmt.Print("Enabling common task")

	conn, err := cl.State.Connect()
	if err != nil {
		cl.State.Fmt.PrintErr(err)
		cl.State.Fmt.Finish()
		return err
	}

	client := proto.NewGoferClient(conn)

	md := metadata.Pairs("Authorization", "Bearer "+cl.State.Config.Token)
	ctx := metadata.NewOutgoingContext(context.Background(), md)
	_, err = client.EnableCommonTask(ctx, &proto.EnableCommonTaskRequest{
		Name: name,
	})
	if err != nil {
		cl.State.Fmt.PrintErr(fmt.Sprintf("could not enable common task: %v", err))
		cl.State.Fmt.Finish()
		return err
	}

	cl.State.Fmt.PrintSuccess("common task enabled")
	cl.State.Fmt.Finish()

	return nil
}
