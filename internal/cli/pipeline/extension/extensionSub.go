package extension

import (
	"context"
	"fmt"
	"strings"

	"github.com/clintjedwards/gofer/internal/cli/cl"
	proto "github.com/clintjedwards/gofer/proto/go"
	"github.com/spf13/cobra"
	"google.golang.org/grpc/metadata"
)

var cmdPipelineExtensionSub = &cobra.Command{
	Use:     "sub <pipeline_id> <name> <label>",
	Short:   "Subscribe a pipeline to a extension",
	Long:    `Subscribe a pipeline to a extension.`,
	Example: `$ gofer pipeline extension sub simple cron every_5_seconds`,
	RunE:    pipelineExtensionSub,
	Args:    cobra.ExactArgs(3),
}

func init() {
	cmdPipelineExtensionSub.Flags().StringSliceP("setting", "s", []string{}, "set pipeline extension setting")
	CmdPipelineExtension.AddCommand(cmdPipelineExtensionSub)
}

func pipelineExtensionSub(cmd *cobra.Command, args []string) error {
	id := args[0]
	name := args[1]
	label := args[2]

	settingsList, err := cmd.Flags().GetStringSlice("setting")
	if err != nil {
		cl.State.Fmt.PrintErr(err)
		cl.State.Fmt.Finish()
		return err
	}

	settingsMap := map[string]string{}
	for _, setting := range settingsList {
		key, value, found := strings.Cut(setting, "=")
		if !found {
			cl.State.Fmt.PrintErr("Key-value pair malformed; should be in format: <key>=<value>")
			cl.State.Fmt.Finish()
			return err
		}
		settingsMap[key] = value
	}

	cl.State.Fmt.Print("Subscribing pipeline to extension")

	conn, err := cl.State.Connect()
	if err != nil {
		cl.State.Fmt.PrintErr(err)
		cl.State.Fmt.Finish()
		return err
	}

	client := proto.NewGoferClient(conn)

	md := metadata.Pairs("Authorization", "Bearer "+cl.State.Config.Token)
	ctx := metadata.NewOutgoingContext(context.Background(), md)
	_, err = client.CreatePipelineExtensionSubscription(ctx, &proto.CreatePipelineExtensionSubscriptionRequest{
		NamespaceId:    cl.State.Config.Namespace,
		PipelineId:     id,
		ExtensionName:  name,
		ExtensionLabel: label,
		Settings:       settingsMap,
	})
	if err != nil {
		cl.State.Fmt.PrintErr(fmt.Sprintf("could not subscribe extension to pipeline: %v", err))
		cl.State.Fmt.Finish()
		return err
	}

	cl.State.Fmt.PrintSuccess(fmt.Sprintf("Pipeline %q subscribed to extension %q", id, name))
	cl.State.Fmt.Finish()

	return nil
}
