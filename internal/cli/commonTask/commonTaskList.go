package commonTask

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

var cmdCommonTaskList = &cobra.Command{
	Use:     "list",
	Short:   "List all commontasks",
	Example: `$ gofer common-task list`,
	RunE:    commontaskList,
}

func init() {
	CmdCommonTask.AddCommand(cmdCommonTaskList)
}

func commontaskList(_ *cobra.Command, _ []string) error {
	cl.State.Fmt.Print("Retrieving common tasks")

	conn, err := cl.State.Connect()
	if err != nil {
		cl.State.Fmt.PrintErr(err)
		cl.State.Fmt.Finish()
		return err
	}

	client := proto.NewGoferClient(conn)

	md := metadata.Pairs("Authorization", "Bearer "+cl.State.Config.Token)
	ctx := metadata.NewOutgoingContext(context.Background(), md)
	resp, err := client.ListCommonTasks(ctx, &proto.ListCommonTasksRequest{})
	if err != nil {
		cl.State.Fmt.PrintErr(fmt.Sprintf("could not list common tasks: %v", err))
		cl.State.Fmt.Finish()
		return err
	}

	data := [][]string{}
	for _, commonTask := range resp.CommonTasks {
		data = append(data, []string{
			commonTask.Name,
			commonTask.Image,
			cliformat.ColorizeCommonTaskRegistrationStatus(cliformat.NormalizeEnumValue(commonTask.Status.String(), "Unknown")),
			commonTask.Documentation,
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

	table.SetHeader([]string{"Name", "Image", "Status", "Documentation Link"})
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
