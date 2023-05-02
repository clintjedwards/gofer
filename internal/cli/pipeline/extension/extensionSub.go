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
	Use:   "sub <pipeline_id> <name> <label>",
	Short: "Subscribe a pipeline to a extension",
	Long: `Subscribe a pipeline to a extension.

Extensions extend the functionality of your pipeline, allowing it to do many things like automatically run based on
some event or post to Slack. To take advantage of this first your pipeline has to "subscribe" to a particular extension.

You can find the current extensions your Gofer instances supports by using the 'gofer extension list' command.

Usually extensions will require some type of configuration for each pipeline subscribed. You can pass this configuration
by using the '--setting' flag.

For example, the "interval" extension requires the subscribing pipeline to specify which interval it would like to be
run on. The parameter is called "every". So one might subscribe to the interval extension like so:

ex. gofer pipeline extend simple interval every_5_seconds -s every="5s"

Passing the config 'every="5s"' as the config parameter required`,
	Example: `$ gofer pipeline extend simple interval every_5_seconds
$ gofer pipeline extend simple interval every_5_seconds -s every="5s"`,
	RunE: pipelineExtensionSub,
	Args: cobra.ExactArgs(3),
}

func init() {
	cmdPipelineExtensionSub.Flags().StringSliceP("setting", "s", []string{}, "set pipeline extension setting")
	CmdPipelineExtension.AddCommand(cmdPipelineExtensionSub)
}

func pipelineExtensionSub(cmd *cobra.Command, args []string) error {
	id := args[0]
	name := args[1]
	label := args[2]

	interactive, err := cmd.Flags().GetBool("interactive")
	if err != nil {
		cl.State.Fmt.PrintErr(err)
		cl.State.Fmt.Finish()
		return err
	}

	settingsList, err := cmd.Flags().GetStringSlice("setting")
	if err != nil {
		cl.State.Fmt.PrintErr(err)
		cl.State.Fmt.Finish()
		return err
	}

	if interactive && len(settingsList) > 0 {
		cl.State.Fmt.PrintErr("Cannot use both the interactive flag and setting flag at the same time")
		cl.State.Fmt.Finish()
		return fmt.Errorf("flag mismatch")
	}

	conn, err := cl.State.Connect()
	if err != nil {
		cl.State.Fmt.PrintErr(err)
		cl.State.Fmt.Finish()
		return err
	}

	client := proto.NewGoferClient(conn)
	md := metadata.Pairs("Authorization", "Bearer "+cl.State.Config.Token)

	settingsMap := map[string]string{}

	if interactive {
		cl.State.Fmt.Print("Attaching to extension to retrieve setting information for interactive mode")

		ctx := metadata.NewOutgoingContext(context.Background(), md)
		interactiveConn, err := client.RunPipelineConfigurator(ctx)
		if err != nil {
			cl.State.Fmt.PrintErr(fmt.Sprintf("Could not connect to extension to get setting instructions %v", err))
			cl.State.Fmt.Finish()
			return err
		}

		interactiveConn.Send(&proto.RunPipelineConfiguratorClientMessage{
			MessageType: &proto.RunPipelineConfiguratorClientMessage_Init_{
				Init: &proto.RunPipelineConfiguratorClientMessage_Init{
					Name: name,
				},
			},
		})

		cl.State.Fmt.PrintSuccess("Attached to extension " + name)

		for {
			msg, err := interactiveConn.Recv()
			if err != nil {
				// If we got an EOF error then the extension closed the connection.
				if strings.Contains(err.Error(), "EOF") {
					break
				}

				// If the context was cancelled, that means that the extension is done and we should process the installation.
				if strings.Contains(err.Error(), "context canceled") {
					break
				}

				// If the client disconnected, exit cleanly.
				if strings.Contains(err.Error(), "client disconnected") {
					break
				}

				cl.State.Fmt.PrintErr(fmt.Sprintf("Could not read from extension connection %v", err))
				cl.State.Fmt.Finish()
			}

			switch extensionMsg := msg.MessageType.(type) {
			case *proto.RunPipelineConfiguratorExtensionMessage_ParamSetting_:
				settingsMap[extensionMsg.ParamSetting.Param] = extensionMsg.ParamSetting.Value
			case *proto.RunPipelineConfiguratorExtensionMessage_Msg:
				cl.State.Fmt.Println(extensionMsg.Msg)
			case *proto.RunPipelineConfiguratorExtensionMessage_Query:
				answer := cl.State.Fmt.PrintQuestion(extensionMsg.Query)
				err := interactiveConn.Send(&proto.RunPipelineConfiguratorClientMessage{
					MessageType: &proto.RunPipelineConfiguratorClientMessage_Msg{
						Msg: answer,
					},
				})
				if err != nil {
					cl.State.Fmt.PrintErr(fmt.Sprintf("Could not send answer to extension query back to extension %v", err))
					cl.State.Fmt.Finish()
					return err
				}
			default:
				cl.State.Fmt.PrintErr(fmt.Sprintf("Could not properly decode incoming message from extension; incorrect type %T", extensionMsg))
				cl.State.Fmt.Finish()
				return err
			}

		}

		cl.State.Fmt.PrintSuccess("Completed interactive extension routine")
	}

	if !interactive {
		for _, setting := range settingsList {
			key, value, found := strings.Cut(setting, "=")
			if !found {
				cl.State.Fmt.PrintErr("Key-value pair malformed; should be in format: <key>=<value>")
				cl.State.Fmt.Finish()
				return err
			}
			settingsMap[key] = value
		}
	}

	cl.State.Fmt.Print("Subscribing pipeline to extension")

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
