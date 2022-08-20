package namespaces

import (
	"context"
	"fmt"

	"github.com/clintjedwards/gofer/internal/cli/cl"
	proto "github.com/clintjedwards/gofer/proto/go"

	"github.com/spf13/cobra"
	"google.golang.org/grpc/metadata"
)

var cmdNamespacesUpdate = &cobra.Command{
	Use:     "update <id>",
	Short:   "Update details on a specific namespace",
	Example: `$ gofer namespace update old_namespace --name="New name"`,
	RunE:    namespacesUpdate,
	Args:    cobra.ExactArgs(1),
}

func init() {
	cmdNamespacesUpdate.Flags().StringP("name", "n", "", "Human readable name for namespace")
	cmdNamespacesUpdate.Flags().StringP("description", "d", "", "Description on use for namespace")
	CmdNamespaces.AddCommand(cmdNamespacesUpdate)
}

func namespacesUpdate(cmd *cobra.Command, args []string) error {
	id := args[0]
	name, err := cmd.Flags().GetString("name")
	if err != nil {
		return err
	}
	description, err := cmd.Flags().GetString("description")
	if err != nil {
		return err
	}

	cl.State.Fmt.Print("Retrieving namespace")

	conn, err := cl.State.Connect()
	if err != nil {
		cl.State.Fmt.PrintErr(err)
		cl.State.Fmt.Finish()
		return err
	}

	client := proto.NewGoferClient(conn)

	md := metadata.Pairs("Authorization", "Bearer "+cl.State.Config.Token)
	ctx := metadata.NewOutgoingContext(context.Background(), md)
	n, err := client.GetNamespace(ctx, &proto.GetNamespaceRequest{
		Id: id,
	})
	if err != nil {
		cl.State.Fmt.PrintErr(fmt.Sprintf("could not get namespace: %v", err))
		cl.State.Fmt.Finish()
		return err
	}

	if name == "" {
		name = n.Namespace.Name
	}

	if description == "" {
		description = n.Namespace.Description
	}

	_, err = client.UpdateNamespace(context.Background(), &proto.UpdateNamespaceRequest{
		Id:          id,
		Name:        name,
		Description: description,
	})
	if err != nil {
		cl.State.Fmt.PrintErr(fmt.Sprintf("could not update namespace: %v", err))
		cl.State.Fmt.Finish()
		return err
	}

	cl.State.Fmt.PrintSuccess(fmt.Sprintf("Updated namespace: [%s] %q", n.Namespace.Id, n.Namespace.Name))
	cl.State.Fmt.Finish()
	return nil
}
