package namespace

import (
	"context"
	"fmt"

	"github.com/clintjedwards/gofer/internal/cli/cl"
	proto "github.com/clintjedwards/gofer/proto/go"

	"github.com/spf13/cobra"
	"google.golang.org/grpc/metadata"
)

var cmdNamespaceDelete = &cobra.Command{
	Use:     "delete <id>",
	Short:   "Delete namespace",
	Long:    `Delete namespace.`,
	Example: `$ gofer namespace delete my_namespace`,
	RunE:    namespaceDelete,
	Args:    cobra.ExactArgs(1),
}

func init() {
	CmdNamespace.AddCommand(cmdNamespaceDelete)
}

func namespaceDelete(_ *cobra.Command, args []string) error {
	id := args[0]

	cl.State.Fmt.Print("Deleting namespace")

	conn, err := cl.State.Connect()
	if err != nil {
		cl.State.Fmt.PrintErr(err)
		cl.State.Fmt.Finish()
		return err
	}

	client := proto.NewGoferClient(conn)

	md := metadata.Pairs("Authorization", "Bearer "+cl.State.Config.Token)
	ctx := metadata.NewOutgoingContext(context.Background(), md)
	_, err = client.DeleteNamespace(ctx, &proto.DeleteNamespaceRequest{
		Id: id,
	})
	if err != nil {
		cl.State.Fmt.PrintErr(fmt.Sprintf("could not delete namespace: %v", err))
		cl.State.Fmt.Finish()
		return err
	}

	cl.State.Fmt.PrintSuccess(fmt.Sprintf("Deleted namespace %q", id))
	cl.State.Fmt.Finish()
	return nil
}
