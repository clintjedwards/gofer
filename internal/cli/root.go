// Package cli controls the main user entry point into both the API and interacting with it.
// It provides not only administrators an easy way to interact with gofer, but is the main entry point
// for how non-UI users interact with Gofer.
package cli

import (
	"fmt"
	"strings"

	"github.com/clintjedwards/gofer/internal/cli/cl"
	"github.com/clintjedwards/gofer/internal/cli/commonTask"
	"github.com/clintjedwards/gofer/internal/cli/event"
	"github.com/clintjedwards/gofer/internal/cli/namespace"
	"github.com/clintjedwards/gofer/internal/cli/pipeline"
	"github.com/clintjedwards/gofer/internal/cli/runs"
	"github.com/clintjedwards/gofer/internal/cli/secrets"
	"github.com/clintjedwards/gofer/internal/cli/service"
	"github.com/clintjedwards/gofer/internal/cli/taskruns"
	"github.com/clintjedwards/gofer/internal/cli/triggers"
	"github.com/spf13/cobra"
)

var appVersion = "0.0.dev_000000"

// RootCmd is the base of the cli
var RootCmd = &cobra.Command{
	Use:   "gofer",
	Short: "Gofer is a distributed, continuous thing do-er.",
	Long: `Gofer is a distributed, continuous thing do-er.

It uses a similar model to concourse(https://concourse-ci.org/), leveraging the docker container as a key mechanism
to run short-lived workloads. The benefits of this is simplicity. No foreign agents, no cluster setup, just run
containers.

Read more at https://clintjedwards.com/gofer
`,
	Version: " ", // We leave this added but empty so that the rootcmd will supply the -v flag
	PersistentPreRun: func(cmd *cobra.Command, _ []string) {
		cl.InitState(cmd)
	},
}

func init() {
	RootCmd.SetVersionTemplate(humanizeVersion(appVersion))
	RootCmd.AddCommand(service.CmdService)
	RootCmd.AddCommand(pipeline.CmdPipeline)
	RootCmd.AddCommand(runs.CmdRuns)
	RootCmd.AddCommand(taskruns.CmdTaskRuns)
	RootCmd.AddCommand(triggers.CmdTriggers)
	RootCmd.AddCommand(namespace.CmdNamespace)
	RootCmd.AddCommand(secrets.CmdSecrets)
	RootCmd.AddCommand(event.CmdEvent)
	RootCmd.AddCommand(commonTask.CmdCommonTask)

	RootCmd.PersistentFlags().String("config", "", "configuration file path")
	RootCmd.PersistentFlags().Bool("detail", false, "show extra detail for some commands (ex. Exact time instead of humanized)")
	RootCmd.PersistentFlags().String("format", "", "output format; accepted values are 'pretty', 'json', 'silent'")
	RootCmd.PersistentFlags().String("namespace", "", "specify which namespace the command should be run against")
	RootCmd.PersistentFlags().Bool("no-color", false, "disable color output")
	RootCmd.PersistentFlags().String("host", "", "specify the URL of the server to communicate to")
}

// Execute adds all child commands to the root command and sets flags appropriately.
func Execute() error {
	return RootCmd.Execute()
}

func humanizeVersion(version string) string {
	semver, hash, err := strings.Cut(version, "_")
	if !err {
		return ""
	}
	return fmt.Sprintf("gofer %s [%s]\n", semver, hash)
}
