package run

import (
	"bytes"
	"context"
	"fmt"
	"io"
	"os"
	"strconv"
	"strings"

	"github.com/clintjedwards/gofer/internal/cli/cl"
	proto "github.com/clintjedwards/gofer/proto/go"

	"github.com/spf13/cobra"
	"google.golang.org/grpc/metadata"
)

var cmdRunStorePut = &cobra.Command{
	Use:   "put <pipeline_id> <run_id> <key>=<object>",
	Short: "Write an object into the run store",
	Long: `Write an object into the run store.

The run store allows storage of objects as key-values pairs that individual runs might need to reference. These run
level objects allow for more objects to be stored than their pipeline counterparts, but are kept on a much shorter
time scale. Run level objects are removed once their run limit is reached(this may be different depending on
configuration). This run limit is related to the number of runs in a pipeline.

For instance, after a run is 10 runs old, gofer may clean up its objects.

You can store both regular text values or read in entire files using the '@' prefix.`,
	Example: `$ gofer run store put simple_test_pipeline my_key=my_value
$ gofer run store put simple_test_pipeline my_key=@file_path`,
	RunE: storePut,
	Args: cobra.ExactArgs(3),
}

func init() {
	cmdRunStorePut.Flags().BoolP("force", "f", false, "replace value if exists")
	CmdRunStore.AddCommand(cmdRunStorePut)
}

func storePut(cmd *cobra.Command, args []string) error {
	pipelineID := args[0]
	runIDRaw := args[1]
	runID, err := strconv.Atoi(runIDRaw)
	if err != nil {
		return err
	}

	force, err := cmd.Flags().GetBool("force")
	if err != nil {
		fmt.Println(err)
		return err
	}

	keyValueStr := args[2]
	key, value, ok := strings.Cut(keyValueStr, "=")
	if !ok {
		fmt.Println("Key-value pair malformed; should be in format <key>=<value>")
		return fmt.Errorf("key-value pair malformed; should be <key>=<value>")
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
	resp, err := client.PutRunObject(ctx, &proto.PutRunObjectRequest{
		NamespaceId: cl.State.Config.Namespace,
		Key:         key,
		Content:     object.Bytes(),
		PipelineId:  pipelineID,
		RunId:       int64(runID),
		Force:       force,
	})
	if err != nil {
		cl.State.Fmt.PrintErr(fmt.Sprintf("could not upload object: %v", err))
		cl.State.Fmt.Finish()
		return err
	}

	cl.State.Fmt.PrintSuccess(fmt.Sprintf("Uploaded %d bytes", resp.Bytes))
	cl.State.Fmt.Finish()

	return nil
}
