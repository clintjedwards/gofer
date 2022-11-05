package main

import (
	"log"

	sdk "github.com/clintjedwards/gofer/sdk/go/config"
)

func main() {
	err := sdk.NewPipeline("common_task", "Common Task Pipeline").
		Description("This pipeline shows off the common tasks feature of Gofer. Common Tasks allow administrators to" +
			" install tasks that can be shared amongst all pipelines. This allows you to provide users with tasks that might require" +
			" variables and credentials that you might not want to manually include in every pipeline.").
		Tasks(sdk.NewCommonTask("debug", "debug_task")).Finish()
	if err != nil {
		log.Fatal(err)
	}
}
