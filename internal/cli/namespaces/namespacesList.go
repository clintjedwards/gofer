package namespaces

import (
	"context"
	"fmt"
	"strings"

	"github.com/clintjedwards/gofer/internal/cli/cl"
	cliformat "github.com/clintjedwards/gofer/internal/cli/format"
	proto "github.com/clintjedwards/gofer/proto/go"

	"github.com/olekukonko/tablewriter"
	"github.com/spf13/cobra"
	"google.golang.org/grpc/metadata"
)

var cmdNamespacesList = &cobra.Command{
	Use:   "list",
	Short: "List all namespaces",
	Long: `List all namespaces.

Namespaces act as divider lines between different sets of pipelines.
`,
	Example: `$ gofer namespaces list`,
	RunE:    namespacesList,
}

func init() {
	cmdNamespacesList.Flags().IntP("limit", "l", 10, "limit the amount of results returned")
	CmdNamespaces.AddCommand(cmdNamespacesList)
}

func namespacesList(cmd *cobra.Command, _ []string) error {
	cl.State.Fmt.Print("Retrieving namespaces")

	limit, err := cmd.Flags().GetInt("limit")
	if err != nil {
		cl.State.Fmt.PrintErr(err)
		cl.State.Fmt.Finish()
		return err
	}

	conn, err := cl.State.Connect()
	if err != nil {
		cl.State.Fmt.PrintErr(err)
		cl.State.Fmt.Finish()
		return err
	}

	client := proto.NewGoferClient(conn)

	md := metadata.Pairs("Authorization", "Bearer "+cl.State.Config.Token)
	ctx := metadata.NewOutgoingContext(context.Background(), md)
	resp, err := client.ListNamespaces(ctx, &proto.ListNamespacesRequest{
		Limit: int64(limit),
	})
	if err != nil {
		cl.State.Fmt.PrintErr(fmt.Sprintf("could not list namespaces: %v", err))
		cl.State.Fmt.Finish()
		return err
	}

	data := [][]string{}
	for _, namespace := range resp.Namespaces {
		data = append(data, []string{
			namespace.Id,
			namespace.Name,
			namespace.Description,
			cliformat.UnixMilli(namespace.Created, "Never", cl.State.Config.Detail),
		})
	}

	table := formatTable(data, !cl.State.Config.NoColor)

	cl.State.Fmt.Println(table)
	cl.State.Fmt.Finish()
	return nil
}

func formatTable(data [][]string, color bool) string {
	tableString := &strings.Builder{}
	table := tablewriter.NewWriter(tableString)

	table.SetHeader([]string{"ID", "Name", "Description", "Created"})
	table.SetAlignment(tablewriter.ALIGN_LEFT)
	table.SetHeaderAlignment(tablewriter.ALIGN_LEFT)
	table.SetHeaderLine(true)
	table.SetBorder(false)
	table.SetAutoFormatHeaders(false)
	table.SetRowSeparator("―")
	table.SetRowLine(false)
	table.SetColumnSeparator("")
	table.SetCenterSeparator("")

	if color {
		table.SetHeaderColor(
			tablewriter.Color(tablewriter.FgBlueColor),
			tablewriter.Color(tablewriter.FgBlueColor),
			tablewriter.Color(tablewriter.FgBlueColor),
			tablewriter.Color(tablewriter.FgBlueColor),
		)
		table.SetColumnColor(
			tablewriter.Color(tablewriter.FgYellowColor),
			tablewriter.Color(0),
			tablewriter.Color(0),
			tablewriter.Color(0),
		)
	}

	table.AppendBulk(data)

	table.Render()
	return tableString.String()
}
