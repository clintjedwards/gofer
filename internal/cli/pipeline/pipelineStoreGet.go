package pipeline

import (
	"context"
	"encoding/binary"
	"fmt"
	"os"

	"github.com/clintjedwards/gofer/internal/cli/cl"
	"github.com/clintjedwards/gofer/proto"
	"github.com/spf13/cobra"
	"google.golang.org/grpc/metadata"
)

var cmdPipelineStoreGet = &cobra.Command{
	Use:     "get <pipeline_id> <key>",
	Short:   "Read an object from the pipeline store",
	Example: `$ gofer pipeline store get simple_test_pipeline my_key`,
	RunE:    pipelineStoreGet,
	Args:    cobra.ExactArgs(2),
}

func init() {
	cmdPipelineStoreGet.Flags().BoolP("stringify", "s", false, "Attempt to print the object as a string")
	CmdPipelineStore.AddCommand(cmdPipelineStoreGet)
}

func pipelineStoreGet(cmd *cobra.Command, args []string) error {
	// We don't use the formatter here because we may want to redirect the object we get into
	// a file or similar situation.
	cl.State.Fmt.Finish()
	pipelineID := args[0]
	key := args[1]

	conn, err := cl.State.Connect()
	if err != nil {
		fmt.Println(err)
		return err
	}

	client := proto.NewGoferClient(conn)

	md := metadata.Pairs("Authorization", "Bearer "+cl.State.Config.Token)
	ctx := metadata.NewOutgoingContext(context.Background(), md)
	resp, err := client.GetPipelineObject(ctx, &proto.GetPipelineObjectRequest{
		NamespaceId: cl.State.Config.Namespace,
		PipelineId:  pipelineID,
		Key:         key,
	})
	if err != nil {
		fmt.Printf("could not read object: %v\n", err)
		return err
	}

	stringify, err := cmd.Flags().GetBool("stringify")
	if err != nil {
		fmt.Println(err)
		return err
	}

	if stringify {
		fmt.Printf("%s", resp.Content)
	} else {
		_ = binary.Write(os.Stdout, binary.LittleEndian, resp.Content)
	}

	return nil
}
