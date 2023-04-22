package taskrun

import (
	"bufio"
	"context"
	"fmt"
	"io"
	"os"
	"strconv"
	"strings"

	"github.com/clintjedwards/gofer/internal/cli/cl"
	proto "github.com/clintjedwards/gofer/proto/go"

	"github.com/spf13/cobra"
)

var cmdTaskRunAttach = &cobra.Command{
	Use:   "attach <pipeline> <run> <id>",
	Short: "Attach to a running container",
	Long: `Attach to a running container.

Gofer allows you to connect your terminal to a container and run commands.
This is useful for debugging or just general informational gathering.

The connection to the container only lasts as long as it's running.`,
	Example: `$ gofer taskrun attach simple 23 example_task`,
	RunE:    taskrunAttach,
	Args:    cobra.ExactArgs(3),
}

func init() {
	cmdTaskRunAttach.Flags().StringP("command", "c", "", "The command to run when attached.")
	CmdTaskRun.AddCommand(cmdTaskRunAttach)
}

func taskrunAttach(cmd *cobra.Command, args []string) error {
	// We don't use the formatter here because we may want to redirect logs we get into
	// a file or such.
	cl.State.Fmt.Finish()

	command := []string{}

	commandRaw, err := cmd.Flags().GetString("command")
	if err != nil {
		fmt.Println(err)
		return err
	}

	if commandRaw != "" {
		command = strings.Split(commandRaw, " ")
	}

	pipeline := args[0]

	runIDRaw := args[1]
	runID, err := strconv.Atoi(runIDRaw)
	if err != nil {
		return err
	}

	id := args[2]

	conn, err := cl.State.Connect()
	if err != nil {
		fmt.Println(err)
		return err
	}
	defer conn.Close()

	client := proto.NewGoferClient(conn)

	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	stream, err := client.AttachToTaskRun(ctx)
	if err != nil {
		fmt.Printf("could not attach to task run: %v\n", err)
		return err
	}

	err = stream.Send(&proto.AttachToTaskRunRequest{
		RequestType: &proto.AttachToTaskRunRequest_Init{
			Init: &proto.AttachToTaskRunInit{
				NamespaceId: cl.State.Config.Namespace,
				PipelineId:  pipeline,
				RunId:       int64(runID),
				Id:          id,
				Command:     command,
			},
		},
	})
	if err != nil {
		fmt.Printf("could not attach to task run: %v\n", err)
		return err
	}

	go func() {
		for {
			resp, err := stream.Recv()
			if err != nil {
				if err == io.EOF {
					break
				}
				fmt.Printf("could not get task run output: %v\n", err)
				return
			}

			fmt.Print(resp.Output)
		}
	}()

	reader := bufio.NewReader(os.Stdin)

	fmt.Printf("Connected to task run %q\n", id)
	for {
		line, err := reader.ReadString('\n')
		if err != nil {
			fmt.Printf("could not read input from cli: %v\n", err)
			return err
		}

		err = stream.Send(&proto.AttachToTaskRunRequest{
			RequestType: &proto.AttachToTaskRunRequest_Input{
				Input: &proto.AttachToTaskRunInput{
					Input: line,
				},
			},
		})
		if err != nil {
			if err.Error() == "EOF" {
				fmt.Println("Server closed the connection. This usually means the container has finished running.")
				return nil
			}

			fmt.Printf("could not send input to container: %v\n", err)
			return err
		}
	}
}
