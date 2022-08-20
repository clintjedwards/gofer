package namespaces

import (
	"context"
	"fmt"
	"strings"

	"github.com/clintjedwards/gofer/internal/cli/cl"
	proto "github.com/clintjedwards/gofer/proto/go"

	"github.com/spf13/cobra"
	"google.golang.org/grpc/metadata"
)

var cmdNamespacesDelete = &cobra.Command{
	Use:     "delete <id>",
	Short:   "Delete namespace",
	Long:    `Delete namespace.`,
	Example: `$ gofer namespaces delete my_namespace`,
	RunE:    namespacesDelete,
	Args:    cobra.ExactArgs(1),
}

func init() {
	CmdNamespaces.AddCommand(cmdNamespacesDelete)
}

func namespacesDelete(_ *cobra.Command, args []string) error {
	id := args[0]

	cl.State.Fmt.Print("Deleting namespace")
	cl.State.Fmt.Finish()

	var input string

	for {
		fmt.Print("Please type the ID of the namespace to confirm: ")
		fmt.Scanln(&input)
		if strings.EqualFold(input, id) {
			break
		}
	}

	cl.State.NewFormatter()

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
