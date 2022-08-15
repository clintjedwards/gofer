package pipeline

import (
	"bytes"
	"context"
	"fmt"
	"io"
	"os"
	"strings"

	"github.com/clintjedwards/gofer/internal/cli/cl"
	proto "github.com/clintjedwards/gofer/proto/go"

	"github.com/spf13/cobra"
	"google.golang.org/grpc/metadata"
)

var cmdPipelineStorePut = &cobra.Command{
	Use:   "put <pipeline_id> <key>=<object>",
	Short: "Write an object into the pipeline store",
	Long: `Write an object into the pipeline store.

The pipeline store allows storage of objects as key-values pairs that many runs might need to reference. These pipeline
level objects are kept forever until the limit of number of pipeline objects is reached(this may be different depending
on configuration). Once this limit is reached the _oldest_ object will be removed to make space for the new object.

You can store both regular text values or read in entire files using the '@' prefix.
`,
	Example: `$ gofer pipeline store put simple_test_pipeline my_key=my_value
$ gofer pipeline store put simple_test_pipeline my_key=@/test/folder/file_path`,
	RunE: pipelineStorePut,
	Args: cobra.ExactArgs(2),
}

func init() {
	cmdPipelineStorePut.Flags().BoolP("force", "f", false, "replace value if exists")
	CmdPipelineStore.AddCommand(cmdPipelineStorePut)
}

func pipelineStorePut(cmd *cobra.Command, args []string) error {
	pipelineID := args[0]
	keyValueStr := args[1]
	key, value, ok := strings.Cut(keyValueStr, "=")
	if !ok {
		fmt.Println("Key-value pair malformed; should be key=value")
		return fmt.Errorf("key-value pair malformed; should be key=value")
	}

	force, err := cmd.Flags().GetBool("force")
	if err != nil {
		fmt.Println(err)
		return err
	}

	object := bytes.NewBuffer([]byte{})
	if strings.HasPrefix(value, "@") {
		file, err := os.Open(value[1:])
		if err != nil {
			cl.State.Fmt.PrintErr(err)
			cl.State.Fmt.Finish()
			return err
		}
		defer file.Close()
		if _, err = io.Copy(object, file); err != nil {
			cl.State.Fmt.PrintErr(err)
			cl.State.Fmt.Finish()
			return err
		}
	} else {
		object.WriteString(value)
	}

	cl.State.Fmt.Print("Uploading object")

	conn, err := cl.State.Connect()
	if err != nil {
		cl.State.Fmt.PrintErr(err)
		cl.State.Fmt.Finish()
		return err
	}

	client := proto.NewGoferClient(conn)

	md := metadata.Pairs("Authorization", "Bearer "+cl.State.Config.Token)
	ctx := metadata.NewOutgoingContext(context.Background(), md)
	resp, err := client.PutPipelineObject(ctx, &proto.PutPipelineObjectRequest{
		NamespaceId: cl.State.Config.Namespace,
		PipelineId:  pipelineID,
		Key:         key,
		Content:     object.Bytes(),
		Force:       force,
	})
	if err != nil {
		cl.State.Fmt.PrintErr(fmt.Sprintf("could not upload object: %v", err))
		cl.State.Fmt.Finish()
		return err
	}

	if resp.ObjectEvicted != "" {
		cl.State.Fmt.Println(fmt.Sprintf("Object evicted due to pipeline object limit(%d): %q", resp.ObjectLimit, resp.ObjectEvicted))
	}

	cl.State.Fmt.PrintSuccess(fmt.Sprintf("Uploaded %d bytes", resp.Bytes))
	cl.State.Fmt.Finish()

	return nil
}
