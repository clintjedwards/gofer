package service

import (
	"github.com/clintjedwards/gofer/internal/cli/service/tokens"
	"github.com/spf13/cobra"
)

var CmdService = &cobra.Command{
	Use:   "service",
	Short: "Manages service related commands for Gofer.",
	Long: `Manages service related commands for the Gofer Service/API.

These commands help with managing and running the Gofer service.`,
}

func init() {
	CmdService.AddCommand(tokens.CmdTokens)
}
