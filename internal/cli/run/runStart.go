package run

import (
	"context"
	"fmt"
	"strings"

	"github.com/clintjedwards/gofer/internal/cli/cl"
	proto "github.com/clintjedwards/gofer/proto/go"

	"github.com/fatih/color"
	"github.com/spf13/cobra"
	"google.golang.org/grpc/metadata"
)

var cmdRunStart = &cobra.Command{
	Use:     "start <pipeline_id>",
	Short:   "Start a new run",
	Example: `$ gofer run start simple_test_pipeline`,
	RunE:    runStart,
	Args:    cobra.ExactArgs(1),
}

func init() {
	cmdRunStart.Flags().StringSliceP("variable", "v", []string{}, "optional environment variables to pass to your run. Format: Key=Value")
	CmdRun.AddCommand(cmdRunStart)
}

func runStart(cmd *cobra.Command, args []string) error {
	pipelineID := args[0]

	variableList, err := cmd.Flags().GetStringSlice("variable")
	if err != nil {
		cl.State.Fmt.PrintErr(err)
		cl.State.Fmt.Finish()
		return err
	}

	variables := map[string]string{}
	for _, variable := range variableList {
		key, value, found := strings.Cut(variable, "=")
		if !found {
			cl.State.Fmt.PrintErr(fmt.Sprintf("malformed variable %q; must be in format <KEY>=<VALUE>", variable))
			cl.State.Fmt.Finish()
			return fmt.Errorf("malformed variable")
		}

		variables[key] = value
	}

	cl.State.Fmt.Print("Starting run")

	conn, err := cl.State.Connect()
	if err != nil {
		cl.State.Fmt.PrintErr(err)
		cl.State.Fmt.Finish()
		return err
	}

	client := proto.NewGoferClient(conn)

	md := metadata.Pairs("Authorization", "Bearer "+cl.State.Config.Token)
	ctx := metadata.NewOutgoingContext(context.Background(), md)
	resp, err := client.StartRun(ctx, &proto.StartRunRequest{
		NamespaceId: cl.State.Config.Namespace,
		PipelineId:  pipelineID,
	})
	if err != nil {
		cl.State.Fmt.PrintErr(fmt.Sprintf("could not start run: %v", err))
		cl.State.Fmt.Finish()
		return err
	}

	cl.State.Fmt.PrintSuccess(fmt.Sprintf("Started new run (%d) for pipeline %s", resp.Run.Id, pipelineID))
	cl.State.Fmt.Println(fmt.Sprintf("\n  View details of your new run: %s", color.YellowString("gofer run get %s %d", resp.Run.Pipeline, resp.Run.Id)))
	cl.State.Fmt.Println(fmt.Sprintf("  List all task runs: %s", color.YellowString("gofer taskruns list %s %d", resp.Run.Pipeline, resp.Run.Id)))
	cl.State.Fmt.Finish()

	return nil
}
