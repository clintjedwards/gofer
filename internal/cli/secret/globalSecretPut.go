package secret

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

var cmdGlobalSecretPut = &cobra.Command{
	Use:   "put <key>=<secret>",
	Short: "Write a secret to the global secret store",
	Long: `Write a secret to the global secret store.

You can store both regular text values or read in entire files using the '@' prefix.

Global secrets are namespaced to allow the segregation of global secrets among different groups.
These namespaces strings allow simple regex expressions to match the actual namespaces within your
environment.

By default, omitting the namespace allows it to match ALL namespaces.

For example an environment that uses prefixes to separate minor teams within an organization might look something
like this: "ops-teama", "ops-teamb".

In this case a global secret can be assigned to a specific team by just using the flag '-n "ops-teama"'. In the case
that you had a global secret that need to be shared amongst all ops teams you could simply write a namespace filter
that has a prefix like so '-n "ops-*"'.`,
	Example: `$ gofer secret global put my_key=my_value
$ gofer secret global put my_key=@/test/folder/file_path
$ gofer secret global put my_key=@/test/folder/file_path -n "ops"
$ gofer secret global put my_key=@/test/folder/file_path -n "ops" -n "marketing"
$ gofer secret global put my_key=@/test/folder/file_path -n "ops-*"`,
	RunE: globalSecretStorePut,
	Args: cobra.ExactArgs(1),
}

func init() {
	cmdGlobalSecretPut.Flags().StringSliceP("namespaces", "n", []string{"*"}, "list of namespaces allowed to access this secret")
	cmdGlobalSecretPut.Flags().BoolP("force", "f", false, "replace value if exists")
	CmdGlobalSecret.AddCommand(cmdGlobalSecretPut)
}

func globalSecretStorePut(cmd *cobra.Command, args []string) error {
	keyValueStr := args[0]

	key, value, ok := strings.Cut(keyValueStr, "=")
	if !ok {
		fmt.Println("Key-value pair malformed; should be in format: <key>=<value>")
		return fmt.Errorf("key-value pair malformed; should be in format <key>=<value>")
	}

	namespaces, err := cmd.Flags().GetStringSlice("namespaces")
	if err != nil {
		fmt.Println(err)
		return err
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
	resp, err := client.PutGlobalSecret(ctx, &proto.PutGlobalSecretRequest{
		Key:        key,
		Content:    secret.String(),
		Namespaces: namespaces,
		Force:      force,
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
