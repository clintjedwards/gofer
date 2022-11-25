package extension

import (
	"context"
	"fmt"

	"github.com/clintjedwards/gofer/internal/cli/cl"
	proto "github.com/clintjedwards/gofer/proto/go"
	"github.com/spf13/cobra"
	"google.golang.org/grpc/metadata"
)

var cmdPipelineExtensionEnable = &cobra.Command{
	Use:     "enable <pipeline_id> <name> <label>",
	Short:   "Enable a pipeline extension subscription",
	Long:    `Enable a pipeline extension subscription.`,
	Example: `$ gofer pipeline extension enable simple cron every_5_seconds`,
	RunE:    pipelineExtensionEnable,
	Args:    cobra.ExactArgs(3),
}

func init() {
	CmdPipelineExtension.AddCommand(cmdPipelineExtensionEnable)
}

func pipelineExtensionEnable(_ *cobra.Command, args []string) error {
	id := args[0]
	name := args[1]
	label := args[2]

	cl.State.Fmt.Print("Enabling subscription")

	conn, err := cl.State.Connect()
	if err != nil {
		cl.State.Fmt.PrintErr(err)
		cl.State.Fmt.Finish()
		return err
	}

	client := proto.NewGoferClient(conn)

	md := metadata.Pairs("Authorization", "Bearer "+cl.State.Config.Token)
	ctx := metadata.NewOutgoingContext(context.Background(), md)
	_, err = client.EnablePipelineExtensionSubscription(ctx, &proto.EnablePipelineExtensionSubscriptionRequest{
		NamespaceId:    cl.State.Config.Namespace,
		PipelineId:     id,
		ExtensionName:  name,
		ExtensionLabel: label,
	})
	if err != nil {
		cl.State.Fmt.PrintErr(fmt.Sprintf("could not enable subscription: %v", err))
		cl.State.Fmt.Finish()
		return err
	}

	cl.State.Fmt.PrintSuccess(fmt.Sprintf("Extension %q (%q) enabled for pipeline %q", label, name, id))
	cl.State.Fmt.Finish()

	return nil
}
