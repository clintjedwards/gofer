package pipeline

import (
	"context"
	"fmt"

	"github.com/clintjedwards/gofer/internal/cli/cl"
	"github.com/clintjedwards/gofer/proto"
	"github.com/spf13/cobra"
	"google.golang.org/grpc/metadata"
)

var cmdPipelineEnable = &cobra.Command{
	Use:   "enable <id>",
	Short: "Enable pipeline",
	Long: `Enable pipeline.

This restores a previously disabled pipeline.`,
	Example: `$ gofer pipeline enable simple_test_pipeline`,
	RunE:    pipelineEnable,
	Args:    cobra.ExactArgs(1),
}

func init() {
	CmdPipeline.AddCommand(cmdPipelineEnable)
}

func pipelineEnable(_ *cobra.Command, args []string) error {
	id := args[0]

	cl.State.Fmt.Print("Enabling pipeline")

	conn, err := cl.State.Connect()
	if err != nil {
		cl.State.Fmt.PrintErr(err)
		cl.State.Fmt.Finish()
		return err
	}

	client := proto.NewGoferClient(conn)

	md := metadata.Pairs("Authorization", "Bearer "+cl.State.Config.Token)
	ctx := metadata.NewOutgoingContext(context.Background(), md)
	_, err = client.EnablePipeline(ctx, &proto.EnablePipelineRequest{
		NamespaceId: cl.State.Config.Namespace,
		Id:          id,
	})
	if err != nil {
		cl.State.Fmt.PrintErr(fmt.Sprintf("could not enable pipeline: %v", err))
		cl.State.Fmt.Finish()
		return err
	}

	cl.State.Fmt.Finish()

	return nil
}
