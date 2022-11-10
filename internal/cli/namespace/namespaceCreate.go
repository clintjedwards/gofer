package namespace

import (
	"context"
	"fmt"

	"github.com/clintjedwards/gofer/internal/cli/cl"
	proto "github.com/clintjedwards/gofer/proto/go"

	"github.com/spf13/cobra"
	"google.golang.org/grpc/metadata"
)

var cmdNamespaceCreate = &cobra.Command{
	Use:   "create <id> <name>",
	Short: "Create a new namespace",
	Long: `Create a new namespace.

Namespaces act as divider lines between different sets of pipelines.
`,
	Example: `$ gofer namespace create new_namespace "New Namespace"
$ gofer namespace create new_namespace "New Namespace" --description="my new namespace"
`,
	RunE: namespaceCreate,
	Args: cobra.ExactArgs(2),
}

func init() {
	cmdNamespaceCreate.Flags().StringP("description", "d", "", "Description on use for namespace")
	CmdNamespace.AddCommand(cmdNamespaceCreate)
}

func namespaceCreate(cmd *cobra.Command, args []string) error {
	id := args[0]
	name := args[1]

	description, err := cmd.Flags().GetString("description")
	if err != nil {
		return err
	}

	cl.State.Fmt.Print("Creating namespace")

	conn, err := cl.State.Connect()
	if err != nil {
		cl.State.Fmt.PrintErr(err)
		cl.State.Fmt.Finish()
		return err
	}

	client := proto.NewGoferClient(conn)

	md := metadata.Pairs("Authorization", "Bearer "+cl.State.Config.Token)
	ctx := metadata.NewOutgoingContext(context.Background(), md)
	resp, err := client.CreateNamespace(ctx, &proto.CreateNamespaceRequest{
		Id:          id,
		Name:        name,
		Description: description,
	})
	if err != nil {
		cl.State.Fmt.PrintErr(fmt.Sprintf("could not create namespace: %v", err))
		cl.State.Fmt.Finish()
		return err
	}
	cl.State.Fmt.PrintSuccess(fmt.Sprintf("Created namespace: [%s] %q", resp.Namespace.Id, resp.Namespace.Name))
	cl.State.Fmt.Finish()
	return nil
}
