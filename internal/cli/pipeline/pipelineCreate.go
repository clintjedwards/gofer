package pipeline

import (
	"context"
	"fmt"
	"os"
	"strings"

	"github.com/clintjedwards/gofer/internal/cli/cl"
	"github.com/clintjedwards/gofer/proto"
	"github.com/fatih/color"
	"github.com/spf13/cobra"
	"google.golang.org/grpc/metadata"
)

var cmdPipelineCreate = &cobra.Command{
	Use:   "create <url|path>",
	Short: "Create a new pipeline",
	Long: `Create a new pipeline.

Creating a Pipeline requires a "pipeline config file". You can find documentation on how create a pipeline configuration
file [here](https://clintjedwards.com/gofer/docs/getting-started/first-steps/generate-pipeline-config).

Gofer can accept a configuration file from your local machine or checked into a repository.

Pipeline configuration files can be a single file or broken up into multiple files. Pointing the create command
at a single file or folder will both work.

Remote configuration file syntax is based off hashicorp's go-getter syntax(https://github.com/hashicorp/go-getter#protocol-specific-options).
Allowing the user to use many remote protocols, authentication schemes, and pass in options.
`,
	Example: `$ gofer pipeline create github.com/clintjedwards/gofer.git//gofer
$ gofer pipeline create somefile.hcl
$ gofer pipeline create ./gofer/test.hcl`,
	RunE: pipelineCreate,
	Args: cobra.ExactArgs(1),
}

func init() {
	CmdPipeline.AddCommand(cmdPipelineCreate)
}

func pipelineCreate(_ *cobra.Command, args []string) error {
	input := args[0]

	cl.State.Fmt.Print("Creating pipeline")

	conn, err := cl.State.Connect()
	if err != nil {
		cl.State.Fmt.PrintErr(err)
		cl.State.Fmt.Finish()
		return err
	}

	client := proto.NewGoferClient(conn)

	md := metadata.Pairs("Authorization", "Bearer "+cl.State.Config.Token)
	ctx := metadata.NewOutgoingContext(context.Background(), md)

	if strings.HasSuffix(strings.ToLower(input), ".hcl") {
		file, err := os.ReadFile(input)
		if err != nil {
			cl.State.Fmt.PrintErr(fmt.Sprintf("could not read file: %v", err))
			cl.State.Fmt.Finish()
			return nil
		}

		resp, err := client.CreatePipelineRaw(ctx, &proto.CreatePipelineRawRequest{
			Content: file,
			Path:    input,
		})
		if err != nil {
			cl.State.Fmt.PrintErr(fmt.Sprintf("could not create pipeline: %v", err))
			cl.State.Fmt.Finish()
			return err
		}

		printCreateSuccess(resp.Pipeline)
		return err
	}

	resp, err := client.CreatePipelineByURL(ctx, &proto.CreatePipelineByURLRequest{
		Url: input,
	})
	if err != nil {
		cl.State.Fmt.PrintErr(fmt.Sprintf("could not create pipeline: %v", err))
		cl.State.Fmt.Finish()
		return err
	}

	printCreateSuccess(resp.Pipeline)
	return nil
}

func printCreateSuccess(pipeline *proto.Pipeline) {
	cl.State.Fmt.PrintSuccess(fmt.Sprintf("Created pipeline: [%s] %q", color.BlueString(pipeline.Id), pipeline.Name))
	cl.State.Fmt.Println(fmt.Sprintf("\n  View details of your new pipeline: %s", color.YellowString("gofer pipeline get %s", pipeline.Id)))
	cl.State.Fmt.Println(fmt.Sprintf("  Start a new run: %s", color.YellowString("gofer run start %s", pipeline.Id)))
	cl.State.Fmt.Finish()
}
