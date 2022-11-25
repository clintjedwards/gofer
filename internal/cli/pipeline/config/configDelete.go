package config

import (
	"context"
	"fmt"
	"strconv"
	"strings"

	"github.com/clintjedwards/gofer/internal/cli/cl"
	proto "github.com/clintjedwards/gofer/proto/go"
	"github.com/fatih/color"

	"github.com/spf13/cobra"
	"google.golang.org/grpc/metadata"
)

var cmdPipelineConfigDelete = &cobra.Command{
	Use:   "delete <pipeline_id> <version>",
	Short: "Delete pipeline config",
	Long: `Delete pipeline configuration.

You cannot remove currently live versions or the last pipeline configuration.
	`,
	Example: `$ gofer pipeline config delete simple 1`,
	RunE:    pipelineConfigDelete,
	Args:    cobra.ExactArgs(2),
}

func init() {
	CmdPipelineConfig.AddCommand(cmdPipelineConfigDelete)
}

func pipelineConfigDelete(_ *cobra.Command, args []string) error {
	pipelineID := args[0]
	versionStr := args[1]

	version, err := strconv.Atoi(versionStr)
	if err != nil {
		cl.State.Fmt.PrintErr("Could not parse verison into number; " + err.Error())
		cl.State.Fmt.Finish()
		return err
	}

	cl.State.Fmt.Print("Deleting pipeline config")
	cl.State.Fmt.Finish()

	var input string

	for {
		fmt.Printf("Please type the ID of the pipeline to confirm you would like to %s: ", color.YellowString("delete configuration version "+versionStr))
		fmt.Scanln(&input)
		if strings.EqualFold(input, pipelineID) {
			break
		}
	}

	cl.State.NewFormatter()

	cl.State.Fmt.Print("Deleting pipeline config")

	conn, err := cl.State.Connect()
	if err != nil {
		cl.State.Fmt.PrintErr(err)
		cl.State.Fmt.Finish()
		return err
	}

	client := proto.NewGoferClient(conn)

	md := metadata.Pairs("Authorization", "Bearer "+cl.State.Config.Token)
	ctx := metadata.NewOutgoingContext(context.Background(), md)
	_, err = client.DeletePipelineConfig(ctx, &proto.DeletePipelineConfigRequest{
		NamespaceId: cl.State.Config.Namespace,
		PipelineId:  pipelineID,
		Version:     int64(version),
	})
	if err != nil {
		cl.State.Fmt.PrintErr(fmt.Sprintf("could not delete pipeline config: %v", err))
		cl.State.Fmt.Finish()
		return err
	}

	cl.State.Fmt.PrintSuccess(fmt.Sprintf("pipeline config version %q deleted", versionStr))
	cl.State.Fmt.Finish()

	return nil
}
