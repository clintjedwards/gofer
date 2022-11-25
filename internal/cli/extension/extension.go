package extension

import (
	"github.com/spf13/cobra"
)

var CmdExtension = &cobra.Command{
	Use:   "extension",
	Short: "Get details about Gofer extensions",
	Long: `Get details about Gofer extensions.

Extensions act as plugins for Gofer that execute a run for a pipeline based on some criteria.

An example of a extension might be the simply the passing of time for the "interval" extension. A user will _subscribe_ to
this extension in their pipeline configuration file and based on settings used in that file interval will alert Gofer
when the user's intended interval of time has passed. This automatically then kicks off a new instance of a run for
that specific pipeline.`,
}
