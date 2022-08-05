package run

// import (
// 	"context"
// 	"fmt"

// 	"github.com/clintjedwards/gofer/internal/cli/cl"
// 	proto "github.com/clintjedwards/gofer/proto/go"

// 	"github.com/fatih/color"
// 	"github.com/spf13/cobra"
// 	"google.golang.org/grpc/metadata"
// )

// var cmdRunStart = &cobra.Command{
// 	Use:     "start <pipeline_id>",
// 	Short:   "Start a new run",
// 	Example: `$ gofer run start simple_test_pipeline`,
// 	RunE:    runStart,
// 	Args:    cobra.ExactArgs(1),
// }

// func init() {
// 	cmdRunStart.Flags().StringSliceP("only", "o", []string{}, "Run only theses tasks")
// 	CmdRun.AddCommand(cmdRunStart)
// }

// func runStart(cmd *cobra.Command, args []string) error {
// 	only, _ := cmd.Flags().GetStringSlice("only")

// 	pipelineID := args[0]

// 	cl.State.Fmt.Print("Creating run")

// 	conn, err := cl.State.Connect()
// 	if err != nil {
// 		cl.State.Fmt.PrintErr(err)
// 		cl.State.Fmt.Finish()
// 		return err
// 	}

// 	client := proto.NewGoferClient(conn)

// 	md := metadata.Pairs("Authorization", "Bearer "+cl.State.Config.Token)
// 	ctx := metadata.NewOutgoingContext(context.Background(), md)
// 	resp, err := client.StartRun(ctx, &proto.StartRunRequest{
// 		NamespaceId: cl.State.Config.Namespace,
// 		PipelineId:  pipelineID,
// 		Only:        only,
// 	})
// 	if err != nil {
// 		cl.State.Fmt.PrintErr(fmt.Sprintf("could not start run: %v", err))
// 		cl.State.Fmt.Finish()
// 		return err
// 	}

// 	cl.State.Fmt.PrintSuccess(fmt.Sprintf("Started new run (%d) for pipeline %s", resp.Run.Id, pipelineID))
// 	cl.State.Fmt.Println(fmt.Sprintf("\n  View details of your new run: %s", color.YellowString("gofer run get %s %d", resp.Run.PipelineId, resp.Run.Id)))
// 	cl.State.Fmt.Println(fmt.Sprintf("  List all task runs: %s", color.YellowString("gofer taskrun list %s %d", resp.Run.PipelineId, resp.Run.Id)))
// 	cl.State.Fmt.Finish()

// 	return nil
// }
