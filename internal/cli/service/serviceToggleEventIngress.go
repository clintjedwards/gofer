package service

import (
	"context"
	"fmt"
	"strings"

	"github.com/clintjedwards/gofer/internal/cli/cl"
	"github.com/clintjedwards/gofer/proto"
	"github.com/spf13/cobra"
	"google.golang.org/grpc/metadata"
)

var cmdRunToggleEventIngress = &cobra.Command{
	Use:     "toggle-event-ingress",
	Short:   "Allows the operator to control run ingress",
	Long:    `Allows the operator to control whether it is possible to start new runs on the Gofer service or not`,
	Example: `$ gofer service toggle-event-ingress`,
	RunE:    runToggleEventIngress,
}

func init() {
	CmdService.AddCommand(cmdRunToggleEventIngress)
}

func runToggleEventIngress(_ *cobra.Command, _ []string) error {
	cl.State.Fmt.Print("Toggling event ingress")
	cl.State.Fmt.Finish()

	var input string

	for {
		fmt.Print("Please type 'iamsure' to confirm: ")
		fmt.Scanln(&input)
		if strings.EqualFold(input, "iamsure") {
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
	resp, err := client.ToggleEventIngress(ctx, &proto.ToggleEventIngressRequest{})
	if err != nil {
		cl.State.Fmt.PrintErr(fmt.Sprintf("could not toggle event ingress: %v", err))
		cl.State.Fmt.Finish()
		return err
	}

	cl.State.Fmt.PrintSuccess(fmt.Sprintf("successfully toggled event ingress; accept new runs = %t", resp.Value))
	cl.State.Fmt.Finish()

	return nil
}
