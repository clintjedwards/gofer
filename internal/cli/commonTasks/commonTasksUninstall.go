package commonTasks

import (
	"context"
	"fmt"

	"github.com/clintjedwards/gofer/internal/cli/cl"
	proto "github.com/clintjedwards/gofer/proto/go"

	"github.com/spf13/cobra"
	"google.golang.org/grpc/metadata"
)

var cmdCommonTasksUninstall = &cobra.Command{
	Use:     "uninstall <name>",
	Short:   "Uninstall a specific common task by name.",
	Long:    `Uninstall a specific common task by name.`,
	Example: `$ gofer common-tasks uninstall cron`,
	RunE:    commontasksUninstall,
	Args:    cobra.ExactArgs(1),
}

func init() {
	CmdCommonTasks.AddCommand(cmdCommonTasksUninstall)
}

func commontasksUninstall(_ *cobra.Command, args []string) error {
	name := args[0]

	cl.State.Fmt.Print("Uninstalling common task")

	conn, err := cl.State.Connect()
	if err != nil {
		cl.State.Fmt.PrintErr(err)
		cl.State.Fmt.Finish()
		return err
	}

	client := proto.NewGoferClient(conn)
	md := metadata.Pairs("Authorization", "Bearer "+cl.State.Config.Token)
	ctx := metadata.NewOutgoingContext(context.Background(), md)
	_, err = client.UninstallCommonTask(ctx, &proto.UninstallCommonTaskRequest{
		Name: name,
	})
	if err != nil {
		cl.State.Fmt.PrintErr(fmt.Sprintf("could not uninstall common task: %v", err))
		cl.State.Fmt.Finish()
		return err
	}

	cl.State.Fmt.PrintSuccess("CommonTask Uninstalled!")
	cl.State.Fmt.Finish()

	return nil
}
