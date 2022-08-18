package secrets

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

var cmdPipelineSecretsPut = &cobra.Command{
	Use:   "put <pipeline_id> <key>=<secret>",
	Short: "Write a secret to the pipeline secret store",
	Long: `Write a secret to the pipeline secret store.

You can store both regular text values or read in entire files using the '@' prefix.
`,
	Example: `$ gofer secrets pipeline put simple_test_pipeline my_key=my_value
$ gofer secrets pipeline put simple_test_pipeline my_key=@/test/folder/file_path`,
	RunE: pipelineSecretsStorePut,
	Args: cobra.ExactArgs(2),
}

func init() {
	cmdPipelineSecretsPut.Flags().BoolP("force", "f", false, "replace value if exists")
	CmdPipelineSecrets.AddCommand(cmdPipelineSecretsPut)
}

func pipelineSecretsStorePut(cmd *cobra.Command, args []string) error {
	pipelineID := args[0]
	keyValueStr := args[1]

	key, value, ok := strings.Cut(keyValueStr, "=")
	if !ok {
		fmt.Println("Key-value pair malformed; should be in format: <key>=<value>")
		return fmt.Errorf("key-value pair malformed; should be in format <key>=<value>")
	}

	force, err := cmd.Flags().GetBool("force")
	if err != nil {
		fmt.Println(err)
		return err
	}

	secret := bytes.NewBuffer([]byte{})
	if strings.HasPrefix(value, "@") {
		file, err := os.Open(value[1:])
		if err != nil {
			cl.State.Fmt.PrintErr(err)
			cl.State.Fmt.Finish()
			return err
		}
		defer file.Close()
		if _, err = io.Copy(secret, file); err != nil {
			cl.State.Fmt.PrintErr(err)
			cl.State.Fmt.Finish()
			return err
		}
	} else {
		secret.WriteString(value)
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
	resp, err := client.PutPipelineSecret(ctx, &proto.PutPipelineSecretRequest{
		NamespaceId: cl.State.Config.Namespace,
		PipelineId:  pipelineID,
		Key:         key,
		Content:     secret.String(),
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
