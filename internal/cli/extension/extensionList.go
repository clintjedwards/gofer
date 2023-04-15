package extension

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

var cmdExtensionList = &cobra.Command{
	Use:     "list",
	Short:   "List all extensions",
	Example: `$ gofer extension list`,
	RunE:    extensionList,
}

func init() {
	CmdExtension.AddCommand(cmdExtensionList)
}

func extensionList(_ *cobra.Command, _ []string) error {
	cl.State.Fmt.Print("Retrieving extensions")

	conn, err := cl.State.Connect()
	if err != nil {
		cl.State.Fmt.PrintErr(err)
		cl.State.Fmt.Finish()
		return err
	}

	client := proto.NewGoferClient(conn)

	md := metadata.Pairs("Authorization", "Bearer "+cl.State.Config.Token)
	ctx := metadata.NewOutgoingContext(context.Background(), md)
	resp, err := client.ListExtensions(ctx, &proto.ListExtensionsRequest{})
	if err != nil {
		cl.State.Fmt.PrintErr(fmt.Sprintf("could not list extensions: %v", err))
		cl.State.Fmt.Finish()
		return err
	}

	data := [][]string{}
	for _, extension := range resp.Extensions {
		data = append(data, []string{
			extension.Name,
			extension.Url,
			cliformat.ColorizeExtensionState(cliformat.NormalizeEnumValue(extension.State.String(), "Unknown")),
			cliformat.ColorizeExtensionStatus(cliformat.NormalizeEnumValue(extension.Status.String(), "Unknown")),
		})
	}

	table := formatTable(data, true)

	cl.State.Fmt.Println(table)
	cl.State.Fmt.Finish()

	return nil
}

func formatTable(data [][]string, color bool) string {
	tableString := &strings.Builder{}
	table := tablewriter.NewWriter(tableString)

	table.SetHeader([]string{"Name", "URL", "State", "Status"})
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
