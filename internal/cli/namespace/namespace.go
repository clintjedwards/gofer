package namespace

import (
	"github.com/spf13/cobra"
)

var CmdNamespace = &cobra.Command{
	Use:   "namespace",
	Short: "Manage namespaces",
	Long: `Manage namespaces.

A namespace is a divider between sets of pipelines. It's usually common to divide namespaces based on
team or environment or some combination of both.`,
}
