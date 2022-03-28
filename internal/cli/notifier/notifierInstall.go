package notifier

import (
	"bufio"
	"context"
	"fmt"
	"io"
	"os"
	"strings"

	"github.com/clintjedwards/gofer/internal/cli/cl"
	"github.com/docker/docker/api/types"
	"github.com/docker/docker/api/types/container"
	"github.com/docker/docker/client"
	"github.com/rs/zerolog/log"
	"github.com/spf13/cobra"
	"golang.org/x/term"
)

var cmdNotifierInstall = &cobra.Command{
	Use:   "install <kind> <image>",
	Short: "Install a notifier by image",
	Long: "Install a notifier by image.\n\n" +
		"Gofer allows two ways to install a notifier. The first is by configuration where you enter the notifier's required" +
		"config into Gofer's main configuration file. The second is through the command line.\n\n" +
		`The command line process is a bit different from the configuration process:

1) Gofer attempts to run the notifier container locally and connects the user's terminal to stdout/in/err
2) The notifier container will walk the user through the installation steps required for the notifier
3) The notifier container will attempt to install the notifier into Gofer on behalf of the user.`,
	Example: `$ gofer notifier install log gchr.io/clintjedwards/gofer-containers/notifiers/log`,
	RunE:    notifierInstall,
	Args:    cobra.ExactArgs(3),
}

func init() {
	cmdNotifierInstall.Flags().StringP("host", "h", "", "URL of Gofer server")
	cmdNotifierInstall.Flags().StringP("user", "u", "", "The username needed for authentication to docker image repository")
	cmdNotifierInstall.Flags().StringP("pass", "p", "", "The password needed for authentication to docker image repository")
	CmdNotifier.AddCommand(cmdNotifierInstall)
}

func notifierInstall(cmd *cobra.Command, args []string) error {
	ctx := context.Background()

	kind := args[0]
	image := args[1]

	host, err := cmd.Flags().GetString("host")
	if err != nil {
		cl.State.Fmt.PrintErr(err)
		cl.State.Fmt.Finish()
		return err
	}

	user, err := cmd.Flags().GetString("user")
	if err != nil {
		cl.State.Fmt.PrintErr(err)
		cl.State.Fmt.Finish()
		return err
	}

	pass, err := cmd.Flags().GetString("pass")
	if err != nil {
		cl.State.Fmt.PrintErr(err)
		cl.State.Fmt.Finish()
		return err
	}

	if host == "" {
		host = cl.State.Config.Host
	}

	cl.State.Fmt.Print("Installing notifier")

	client, err := client.NewClientWithOpts(client.FromEnv, client.WithAPIVersionNegotiation())
	if err != nil {
		cl.State.Fmt.PrintErr(err)
		cl.State.Fmt.Finish()
		return err
	}

	// Check connection to docker
	_, err = client.Info(context.Background())
	if err != nil {
		cl.State.Fmt.PrintErr(err)
		cl.State.Fmt.Finish()
		return err
	}

	cl.State.Fmt.Print("Downloading docker image")

	r, err := client.ImagePull(context.Background(), image, types.ImagePullOptions{})
	if err != nil {
		if strings.Contains(err.Error(), "manifest unknown") {
			cl.State.Fmt.PrintErr(fmt.Sprintf("image %q not found or missing auth: %v", image, err))
			cl.State.Fmt.Finish()
			return err
		}
		cl.State.Fmt.PrintErr(err)
		cl.State.Fmt.Finish()
		return err
	}
	_, _ = io.Copy(io.Discard, r)
	defer r.Close() // We don't care about pull logs only the errors

	cl.State.Fmt.PrintSuccess("Downloaded docker image")

	containerConfig := &container.Config{
		Image:      image,
		Entrypoint: []string{"./notifier", "installer"},
		Env: []string{
			fmt.Sprintf("GOFER_NOTIFIER_INSTALLER_KIND=%s", kind),
			fmt.Sprintf("GOFER_NOTIFIER_INSTALLER_SERVER_HOST=%s", host),
			fmt.Sprintf("GOFER_NOTIFIER_INSTALLER_TOKEN=%s", cl.State.Config.Token),
			fmt.Sprintf("GOFER_NOTIFIER_INSTALLER_IMAGE_USER=%s", user),
			fmt.Sprintf("GOFER_NOTIFIER_INSTALLER_IMAGE_PASS=%s", pass),
		},
		AttachStdout: true,
		AttachStderr: true,
		AttachStdin:  true,
		Tty:          true,
		OpenStdin:    true, // Required to connect to Stdin
		StdinOnce:    true,
	}

	hostConfig := &container.HostConfig{
		AutoRemove: true,
		Privileged: false,
	}

	removeOptions := types.ContainerRemoveOptions{
		RemoveVolumes: true,
		Force:         true,
	}

	cl.State.Fmt.Print("Starting installer container")

	// Attempt to remove the container to avoid naming collisions if the autoremove didn't work.
	_ = client.ContainerRemove(ctx, fmt.Sprintf("gofer_notifier_installer_%s", kind), removeOptions)

	createResp, err := client.ContainerCreate(ctx, containerConfig, hostConfig, nil, nil,
		fmt.Sprintf("gofer_notifier_installer_%s", kind))
	if err != nil {
		cl.State.Fmt.PrintErr(err)
		cl.State.Fmt.Finish()
		return err
	}

	err = client.ContainerStart(ctx, createResp.ID, types.ContainerStartOptions{})
	if err != nil {
		cl.State.Fmt.PrintErr(err)
		cl.State.Fmt.Finish()
		return err
	}
	defer client.ContainerStop(ctx, createResp.ID, nil) // nolint: errcheck

	cl.State.Fmt.PrintSuccess("Started installer container")
	cl.State.Fmt.Print("Attaching to installer container")

	hijack, err := client.ContainerAttach(ctx, createResp.ID, types.ContainerAttachOptions{
		Stdin:  true,
		Stderr: true,
		Stdout: true,
		Logs:   true, // If Logs is false we miss whatever lines are printed before we can connect to the container.
		Stream: true, // If Stream is false we get only the lines that were printed before we could connect and not new lines.
	})
	if err != nil {
		cl.State.Fmt.PrintErr(err)
		cl.State.Fmt.Finish()
		return err
	}
	defer hijack.Close()

	cl.State.Fmt.PrintSuccess("Attached to installer container")
	cl.State.Fmt.Finish()

	go func() {
		_, err := io.Copy(os.Stdout, hijack.Reader)
		if err != nil {
			log.Fatal().Err(err).Msg("could not properly connect to stdin")
		}
	}()

	fd := int(os.Stdin.Fd())
	oldState, err := term.MakeRaw(fd)
	if err != nil {
		log.Error().Err(err).Msg("could not properly connect to stdin")
		return err
	}

	defer term.Restore(fd, oldState) // nolint: errcheck

	go func() {
		consoleReader := bufio.NewReaderSize(os.Stdin, 1)
		for {
			input, err := consoleReader.ReadByte()
			if err != nil {
				log.Fatal().Err(err).Msg("could not properly connect to stdin")
			}
			_, err = hijack.Conn.Write([]byte{input})
			if err != nil {
				log.Fatal().Err(err).Msg("could not properly connect to stdin")
			}
		}
	}()

	// We wait on the end of the container so we can display the result to the user and then exit cleanly.
	wait, waitErr := client.ContainerWait(ctx, createResp.ID, container.WaitConditionNotRunning)
	select {
	case <-wait:
		cl.State.NewFormatter()
		cl.State.Fmt.PrintSuccess(fmt.Sprintf("Successfully installed notifier %q", kind))
		return nil
	case err = <-waitErr:
		log.Error().Err(err).Msg("could not wait on container")
		return err
	}
}
