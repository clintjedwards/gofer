package registry

import (
	"github.com/spf13/cobra"
)

var CmdRegistry = &cobra.Command{
	Use:   "registry",
	Short: "Manage docker registry authentication",
	Long: `Gofer manages access to private docker registries through pre-registering credentials for that registry using
these tools. Once registered any downstream pipelines using images from these registries are passed along the credentials
mentioned here.`,
}
