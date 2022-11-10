package commonTask

import (
	"context"
	"fmt"
	"strings"

	"github.com/clintjedwards/gofer/internal/cli/cl"
	proto "github.com/clintjedwards/gofer/proto/go"

	"github.com/spf13/cobra"
	"google.golang.org/grpc/metadata"
)

var cmdCommonTaskDisable = &cobra.Command{
	Use:     "disable <name>",
	Short:   "Disable a specific common task by name.",
	Example: `$ gofer common-task disable cron`,
	RunE:    commonTaskDisable,
	Args:    cobra.ExactArgs(1),
}

func init() {
	CmdCommonTask.AddCommand(cmdCommonTaskDisable)
}

func commonTaskDisable(_ *cobra.Command, args []string) error {
	name := args[0]

	cl.State.Fmt.Print("Disabling common task")
	cl.State.Fmt.Finish()

	var input string

	for {
		fmt.Println("It is important to note that disabling a common task will cause all pipelines using the common task to fail")
		fmt.Print("Please type the ID of the common task to confirm: ")
		fmt.Scanln(&input)
		if strings.EqualFold(input, name) {
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
	_, err = client.DisableCommonTask(ctx, &proto.DisableCommonTaskRequest{
		Name: name,
	})
	if err != nil {
		cl.State.Fmt.PrintErr(fmt.Sprintf("could not disable common task: %v", err))
		cl.State.Fmt.Finish()
		return err
	}

	cl.State.Fmt.PrintSuccess("Common Task disabled")
	cl.State.Fmt.Finish()

	return nil
}
