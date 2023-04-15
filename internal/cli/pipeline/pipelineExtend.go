package pipeline

import (
	"context"
	"fmt"
	"strings"

	"github.com/clintjedwards/gofer/internal/cli/cl"
	proto "github.com/clintjedwards/gofer/proto/go"

	"github.com/spf13/cobra"
	"google.golang.org/grpc/metadata"
)

var cmdPipelineExtend = &cobra.Command{
	Use:   "extend <pipeline> <name> <label>",
	Short: "Subscribe a pipeline to an extension",
	Long: `Subscribe a pipeline to a extension.

Extensions extend the functionality of your pipeline, allowing it to do many things like automatically run based on
some event or post to Slack. To take advantage of this first your pipeline has to "subscribe" to a particular extension.

You can find the current extensions your Gofer instances supports by using the 'gofer extension list' command.

Usually extensions will require some type of configuration for each pipeline subscribed. You can pass this configuration
by using the '--setting' flag.

For example, the "interval" trigger requires the subscribing pipeline to specify which interval it would like to be
run on. The parameter is called "every". So one might subscribe to the interval trigger like so:

ex. gofer pipeline extend simple interval every_5_seconds -s every="5s"

Passing the config 'every="5s"' as the config parameter required`,
	Example: `$ gofer pipeline extend simple interval every_5_seconds
$ gofer pipeline extend simple interval every_5_seconds -s every="5s"`,
	RunE: pipelineExtend,
	Args: cobra.ExactArgs(3),
}

func init() {
	cmdPipelineExtend.Flags().StringSliceP("setting", "s", []string{}, "set pipeline extension setting")
	CmdPipeline.AddCommand(cmdPipelineExtend)
}

func pipelineExtend(cmd *cobra.Command, args []string) error {
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
