package namespaces

import (
	"bytes"
	"context"
	"fmt"
	"text/template"

	"github.com/clintjedwards/gofer/internal/cli/cl"
	cliformat "github.com/clintjedwards/gofer/internal/cli/format"
	"github.com/clintjedwards/gofer/models"
	proto "github.com/clintjedwards/gofer/proto/go"

	"github.com/fatih/color"
	"github.com/spf13/cobra"
	"google.golang.org/grpc/metadata"
)

var cmdNamespacesGet = &cobra.Command{
	Use:     "get <id>",
	Short:   "Get details on a specific namespace",
	Example: `$ gofer namespaces get new_namespace`,
	RunE:    namespacesGet,
	Args:    cobra.ExactArgs(1),
}

func init() {
	CmdNamespaces.AddCommand(cmdNamespacesGet)
}

func namespacesGet(_ *cobra.Command, args []string) error {
	id := args[0]

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
	resp, err := client.GetNamespace(ctx, &proto.GetNamespaceRequest{
		Id: id,
	})
	if err != nil {
		cl.State.Fmt.PrintErr(fmt.Sprintf("could not get namespace: %v", err))
		cl.State.Fmt.Finish()
		return err
	}

	namespace := models.Namespace{}
	namespace.FromProto(resp.Namespace)

	output, err := formatNamespace(&namespace, cl.State.Config.Detail)
	if err != nil {
		cl.State.Fmt.PrintErr(fmt.Sprintf("could not render namespace: %v", err))
		cl.State.Fmt.Finish()
		return err
	}

	cl.State.Fmt.Println(output)
	cl.State.Fmt.Finish()
	return nil
}

type data struct {
	ID          string
	Name        string
	Description string
	Created     string
	Deleted     string
}

func formatNamespace(namespace *models.Namespace, detail bool) (string, error) {
	data := data{
		ID:          color.BlueString(namespace.ID),
		Name:        namespace.Name,
		Description: namespace.Description,
		Created:     cliformat.UnixMilli(namespace.Created, "Never", detail),
	}

	const formatTmpl = `[{{.ID}}] {{.Name}} :: Created {{.Created}}
	{{.Description}}`

	var tpl bytes.Buffer
	t := template.Must(template.New("tmp").Parse(formatTmpl))
	_ = t.Execute(&tpl, data)
	return tpl.String(), nil
}
