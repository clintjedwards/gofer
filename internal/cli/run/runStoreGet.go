package run

import (
	"context"
	"encoding/binary"
	"fmt"
	"os"
	"strconv"

	"github.com/clintjedwards/gofer/internal/cli/cl"
	"github.com/clintjedwards/gofer/proto"
	"github.com/spf13/cobra"
	"google.golang.org/grpc/metadata"
)

var cmdRunStoreGet = &cobra.Command{
	Use:     "get <pipeline_id> <run_id> <key>",
	Short:   "Read an object from the run specific store",
	Example: `$ gofer run store get simple_test_pipeline 5 my_key`,
	RunE:    storeGet,
	Args:    cobra.ExactArgs(3),
}

func init() {
	cmdRunStoreGet.Flags().BoolP("stringify", "s", false, "Attempt to print the object as a string")
	CmdRunStore.AddCommand(cmdRunStoreGet)
}

func storeGet(cmd *cobra.Command, args []string) error {
	// We don't use the formatter here because we may want to redirect the object we get into
	// a file or similar situation.
	cl.State.Fmt.Finish()

	pipelineID := args[0]
	runIDRaw := args[1]
	runID, err := strconv.Atoi(runIDRaw)
	if err != nil {
		return err
	}

	key := args[2]

	conn, err := cl.State.Connect()
	if err != nil {
		fmt.Println(err)
		return err
	}

	client := proto.NewGoferClient(conn)

	md := metadata.Pairs("Authorization", "Bearer "+cl.State.Config.Token)
	ctx := metadata.NewOutgoingContext(context.Background(), md)
	resp, err := client.GetRunObject(ctx, &proto.GetRunObjectRequest{
		NamespaceId: cl.State.Config.Namespace,
		PipelineId:  pipelineID,
		RunId:       int64(runID),
		Key:         key,
	})
	if err != nil {
		fmt.Printf("could not read object: %v\n", err)
		return err
	}

	stringify, err := cmd.Flags().GetBool("stringify")
	if err != nil {
		fmt.Println(err)
		return nil
	}

	if stringify {
		fmt.Printf("%s", resp.Content)
	} else {
		_ = binary.Write(os.Stdout, binary.LittleEndian, resp.Content)
	}

	return nil
}
