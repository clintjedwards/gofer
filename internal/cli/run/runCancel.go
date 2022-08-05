package run

// import (
// 	"context"
// 	"fmt"
// 	"strconv"

// 	"github.com/clintjedwards/gofer/internal/cli/cl"
// 	proto "github.com/clintjedwards/gofer/proto/go"

// 	"github.com/spf13/cobra"
// 	"google.golang.org/grpc/metadata"
// )

// var cmdRunCancel = &cobra.Command{
// 	Use:     "cancel <pipeline> <id>",
// 	Short:   "Cancel a run in progress",
// 	Example: `$ gofer run cancel simple_test_pipeline 3`,
// 	RunE:    runCancel,
// 	Args:    cobra.ExactArgs(2),
// }

// func init() {
// 	cmdRunCancel.Flags().BoolP("force", "f", false, "Stop run and child taskrun containers immediately (SIGKILL)")
// 	CmdRun.AddCommand(cmdRunCancel)
// }

// func runCancel(cmd *cobra.Command, args []string) error {
// 	pipelineID := args[0]
// 	idRaw := args[1]
// 	id, err := strconv.Atoi(idRaw)
// 	if err != nil {
// 		return err
// 	}

// 	force, err := cmd.Flags().GetBool("force")
// 	if err != nil {
// 		fmt.Println(err)
// 		return err
// 	}

// 	cl.State.Fmt.Print("Cancelling run")

// 	conn, err := cl.State.Connect()
// 	if err != nil {
// 		cl.State.Fmt.PrintErr(err)
// 		cl.State.Fmt.Finish()
// 		return err
// 	}

// 	client := proto.NewGoferClient(conn)

// 	md := metadata.Pairs("Authorization", "Bearer "+cl.State.Config.Token)
// 	ctx := metadata.NewOutgoingContext(context.Background(), md)
// 	_, err = client.CancelRun(ctx, &proto.CancelRunRequest{
// 		NamespaceId: cl.State.Config.Namespace,
// 		Id:          int64(id),
// 		PipelineId:  pipelineID,
// 		Force:       force,
// 	})
// 	if err != nil {
// 		cl.State.Fmt.PrintErr(fmt.Sprintf("could not cancel run: %v", err))
// 		cl.State.Fmt.Finish()
// 		return err
// 	}

// 	cl.State.Fmt.PrintSuccess("canceled run")
// 	cl.State.Fmt.Finish()

// 	return nil
// }
