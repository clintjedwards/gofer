package service

import (
	"fmt"

	"github.com/clintjedwards/gofer/internal/cli/cl"
	"github.com/clintjedwards/gofer/internal/config"
	"github.com/spf13/cobra"
)

var cmdServicePrintEnv = &cobra.Command{
	Use:   "printenv",
	Short: "Print the list of environment variables the server looks for on startup.",
	Long: `Print the list of environment variables the server looks for on startup.

This is helpful for setting variables for controlling how the server should work.

All configuration set by environment variable overrides default and config file read configuration.`,
	RunE: serverPrintEnv,
}

func init() {
	CmdService.AddCommand(cmdServicePrintEnv)
}

func serverPrintEnv(_ *cobra.Command, _ []string) error {
	cl.State.Fmt.Finish()
	err := config.PrintAPIEnvs()
	if err != nil {
		fmt.Printf("could not print envs: %v\n", err)
		return nil
	}

	return nil
}
