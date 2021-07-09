package pipeline

import (
	"context"
	"fmt"
	"os"
	"strings"
	"time"

	"github.com/clintjedwards/gofer/internal/cli/cl"
	"github.com/clintjedwards/gofer/proto"
	"github.com/spf13/cobra"
	"google.golang.org/grpc/metadata"
)

var cmdPipelineUpdate = &cobra.Command{
	Use:   "update <id> <url|file>",
	Short: "Update pipeline",
	Long: `Update pipeline via pipeline configuration file.

Warning! Updating a pipeline requires disabling that pipeline and pausing all trigger events.
This may cause those events while the pipeline is being upgraded to be discarded.

Updating a Pipeline requires a "pipeline config file". You can find documentation on how create a pipeline configuration
file here: https://clintjedwards.com/gofer/docs/pipeline-configuration/overview

Gofer can accept a configuration file from your local machine or checked into a repository.

Pipeline configuration files can be a single file or broken up into multiple files. Pointing the create command
at a single file or folder will both work.

Remote configuration file syntax is based off hashicorp's go-getter syntax(https://github.com/hashicorp/go-getter#protocol-specific-options).
Allowing the user to use many remote protocols and pass in options.
`,
	Example: `$ gofer pipeline update aup3gq github.com/clintjedwards/gofer.git//gofer
$ gofer pipeline update simple_test_pipeline somefile.hcl
$ gofer pipeline update simple_test_pipeline ./gofer/test.hcl`,
	RunE: pipelineUpdate,
	Args: cobra.ExactArgs(2),
}

func init() {
	cmdPipelineUpdate.Flags().BoolP("force", "f", false, "Stop all runs and update pipeline immediately")
	cmdPipelineUpdate.Flags().BoolP("graceful-stop", "g", false,
		"Stop all runs gracefully; sends a SIGTERM to all task runs for all in-progress runs and then waits for them to stop.")
	CmdPipeline.AddCommand(cmdPipelineUpdate)
}

func pipelineUpdate(cmd *cobra.Command, args []string) error {
	id := args[0]
	input := args[1]
	force, err := cmd.Flags().GetBool("force")
	if err != nil {
		fmt.Println(err)
		return err
	}

	gracefully, err := cmd.Flags().GetBool("graceful-stop")
	if err != nil {
		fmt.Println(err)
		return err
	}

	cl.State.Fmt.Print("Updating pipeline")

	conn, err := cl.State.Connect()
	if err != nil {
		cl.State.Fmt.PrintErr(err)
		cl.State.Fmt.Finish()
		return err
	}

	client := proto.NewGoferClient(conn)

	cl.State.Fmt.Print("Disabling pipeline")

	md := metadata.Pairs("Authorization", "Bearer "+cl.State.Config.Token)
	ctx := metadata.NewOutgoingContext(context.Background(), md)
	_, err = client.DisablePipeline(ctx, &proto.DisablePipelineRequest{
		NamespaceId: cl.State.Config.Namespace,
		Id:          id,
	})
	if err != nil {
		cl.State.Fmt.PrintErr(fmt.Sprintf("could not disable pipeline: %v", err))
		cl.State.Fmt.Finish()
		return err
	}

	cl.State.Fmt.PrintSuccess("Disabled pipeline")

	if force {
		cl.State.Fmt.Print("Force cancelling pipeline runs")

		_, err = client.CancelAllRuns(ctx, &proto.CancelAllRunsRequest{PipelineId: id, Force: true})
		if err != nil {
			cl.State.Fmt.PrintErr(fmt.Sprintf("could not force cancel pipeline: %v", err))
			cl.State.Fmt.Finish()
			return err
		}
	}

	if gracefully {
		cl.State.Fmt.Print("Cancelling pipeline runs")

		_, err = client.CancelAllRuns(ctx, &proto.CancelAllRunsRequest{PipelineId: id, Force: false})
		if err != nil {
			cl.State.Fmt.PrintErr(fmt.Sprintf("could not gracefully cancel pipeline: %v", err))
			cl.State.Fmt.Finish()
			return err
		}
	}

	cl.State.Fmt.Print("Waiting for in-progress runs to stop")
	for {
		resp, err := client.ListRuns(ctx, &proto.ListRunsRequest{PipelineId: id, Offset: 0, Limit: 10})
		if err != nil {
			cl.State.Fmt.PrintErr(fmt.Sprintf("could not get run list for pipeline: %v", err))
			time.Sleep(time.Second * 3)
			continue
		}

		hasRunningJob := false
		for _, run := range resp.Runs {
			if run.State != proto.Run_FAILED && run.State != proto.Run_CANCELLED && run.State != proto.Run_SUCCESS {
				hasRunningJob = true
			}
		}

		if hasRunningJob {
			time.Sleep(time.Second * 5)
			continue
		}

		break
	}

	cl.State.Fmt.PrintSuccess("Checked for runs in-progress")

	cl.State.Fmt.Print("Updating pipeline")
	if strings.HasSuffix(strings.ToLower(input), ".hcl") {
		file, err := os.ReadFile(input)
		if err != nil {
			cl.State.Fmt.PrintErr(fmt.Sprintf("could not read file: %v", err))
			cl.State.Fmt.Finish()
			return err
		}

		resp, err := client.UpdatePipelineRaw(ctx, &proto.UpdatePipelineRawRequest{
			Id:      id,
			Content: file,
			Path:    input,
		})
		if err != nil {
			cl.State.Fmt.PrintErr(fmt.Sprintf("could not update pipeline: %v", err))
			cl.State.Fmt.Finish()
			return err
		}

		cl.State.Fmt.PrintSuccess(fmt.Sprintf("Updated pipeline: [%s] %q", resp.Pipeline.Id, resp.Pipeline.Name))

		cl.State.Fmt.Print("Enabling pipeline")

		_, err = client.EnablePipeline(ctx, &proto.EnablePipelineRequest{
			Id: id,
		})
		if err != nil {
			cl.State.Fmt.PrintErr(fmt.Sprintf("could not enable pipeline: %v", err))
			cl.State.Fmt.Finish()
			return err
		}

		cl.State.Fmt.PrintSuccess("Enabled pipeline")
		cl.State.Fmt.Finish()
		return nil
	}

	resp, err := client.UpdatePipelineByURL(ctx, &proto.UpdatePipelineByURLRequest{
		Id:  id,
		Url: input,
	})
	if err != nil {
		cl.State.Fmt.PrintErr(fmt.Sprintf("could not update pipeline: %v", err))
		cl.State.Fmt.Finish()
		return err
	}

	cl.State.Fmt.PrintSuccess(fmt.Sprintf("Updated pipeline: [%s] %q", resp.Pipeline.Id, resp.Pipeline.Name))

	cl.State.Fmt.Print("Enabling pipeline")

	_, err = client.EnablePipeline(ctx, &proto.EnablePipelineRequest{
		Id: id,
	})
	if err != nil {
		cl.State.Fmt.PrintErr(fmt.Sprintf("could not enable pipeline: %v", err))
		cl.State.Fmt.Finish()
		return err
	}

	cl.State.Fmt.PrintSuccess("Enabled pipeline")
	cl.State.Fmt.Finish()

	return nil
}
