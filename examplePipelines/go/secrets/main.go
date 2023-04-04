package main

import (
	"log"

	sdk "github.com/clintjedwards/gofer/sdk/go/config"
)

func main() {
	err := sdk.NewPipeline("secrets", "Secrets Pipeline").
		Description(
			"This pipeline displays how one might use Gofer's object/kv store to pass container results " +
				"to other containers.").
		Tasks(
			sdk.NewCustomTask("simple_task", "ghcr.io/clintjedwards/gofer/debug/log:latest").
				Description("This task has no dependencies so it will run immediately").
				Variables(map[string]string{
					"SOME_VARIABLE":             "something here",
					"ALTERNATE_LOGS_HEADER":     sdk.PipelineSecret("alternate_logs_header"),
					"SOME_GLOBAL_SECRET_I_NEED": sdk.GlobalSecret("test_secret_global"),
				})).Finish()
	if err != nil {
		log.Fatal(err)
	}
}
