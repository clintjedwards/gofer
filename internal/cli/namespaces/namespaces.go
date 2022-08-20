package namespaces

import (
	"github.com/spf13/cobra"
)

var CmdNamespaces = &cobra.Command{
	Use:   "namespaces",
	Short: "Manage namespaces",
	Long: `Manage namespaces.

A namespace is a divider between sets of pipelines. It's usually common to divide namespaces based on
team or environment or some combination of both.`,
}
