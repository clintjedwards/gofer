package extension

import (
	"context"
	"fmt"

	"github.com/clintjedwards/gofer/internal/cli/cl"
	proto "github.com/clintjedwards/gofer/proto/go"
	"github.com/spf13/cobra"
	"google.golang.org/grpc/metadata"
)

var cmdPipelineExtensionUnsub = &cobra.Command{
	Use:     "unsub <pipeline_id> <name> <label>",
	Short:   "Unsubscribe a pipeline from a extension",
	Long:    `Unsubscribe a pipeline from a extension.`,
	Example: `$ gofer pipeline extension unsub simple cron every_5_seconds`,
	RunE:    pipelineExtensionUnsub,
	Args:    cobra.ExactArgs(3),
}

func init() {
	CmdPipelineExtension.AddCommand(cmdPipelineExtensionUnsub)
}

func pipelineExtensionUnsub(_ *cobra.Command, args []string) error {
	id := args[0]
	name := args[1]
	label := args[2]

	cl.State.Fmt.Print("Unsubscribing pipeline from extension")

	conn, err := cl.State.Connect()
	if err != nil {
		cl.State.Fmt.PrintErr(err)
		cl.State.Fmt.Finish()
		return err
	}

	client := proto.NewGoferClient(conn)

	md := metadata.Pairs("Authorization", "Bearer "+cl.State.Config.Token)
	ctx := metadata.NewOutgoingContext(context.Background(), md)
	_, err = client.DeletePipelineExtensionSubscription(ctx, &proto.DeletePipelineExtensionSubscriptionRequest{
		NamespaceId:    cl.State.Config.Namespace,
		PipelineId:     id,
		ExtensionName:  name,
		ExtensionLabel: label,
	})
	if err != nil {
		cl.State.Fmt.PrintErr(fmt.Sprintf("could not unsubscribe extension from pipeline: %v", err))
		cl.State.Fmt.Finish()
		return err
	}

	cl.State.Fmt.PrintSuccess(fmt.Sprintf("pipeline %q unsubscribed from extension %q with label %q", id, name, label))
	cl.State.Fmt.Finish()

	return nil
}
