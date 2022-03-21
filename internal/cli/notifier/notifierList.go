package notifier

import (
	"context"
	"fmt"
	"strings"

	"github.com/clintjedwards/gofer/internal/cli/cl"
	"github.com/clintjedwards/gofer/proto"
	"github.com/olekukonko/tablewriter"
	"github.com/spf13/cobra"
	"google.golang.org/grpc/metadata"
)

var cmdNotifierList = &cobra.Command{
	Use:     "list",
	Short:   "List all notifiers",
	Example: `$ gofer notifier list`,
	RunE:    notifierList,
}

func init() {
	CmdNotifier.AddCommand(cmdNotifierList)
}

func notifierList(_ *cobra.Command, _ []string) error {
	cl.State.Fmt.Print("Retrieving notifiers")

	conn, err := cl.State.Connect()
	if err != nil {
		cl.State.Fmt.PrintErr(err)
		cl.State.Fmt.Finish()
		return err
	}

	client := proto.NewGoferClient(conn)

	md := metadata.Pairs("Authorization", "Bearer "+cl.State.Config.Token)
	ctx := metadata.NewOutgoingContext(context.Background(), md)
	resp, err := client.ListNotifiers(ctx, &proto.ListNotifiersRequest{})
	if err != nil {
		cl.State.Fmt.PrintErr(fmt.Sprintf("could not list notifiers: %v", err))
		cl.State.Fmt.Finish()
		return err
	}

	data := [][]string{}
	for _, notifier := range resp.Notifiers {
		data = append(data, []string{
			notifier.Kind,
			notifier.Image,
			notifier.Documentation,
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

	table.SetHeader([]string{"ID", "Image", "Documentation Link"})
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
		)
		table.SetColumnColor(
			tablewriter.Color(tablewriter.FgYellowColor),
			tablewriter.Color(0),
			tablewriter.Color(0),
		)
	}

	table.AppendBulk(data)

	table.Render()
	return tableString.String()
}
