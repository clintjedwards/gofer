// This small script is used to generate the sdk's openAPI spec. Since generating the spec requires you to grab
// the huma.API object it is somewhat tough to do without initializing some part of the program which we do here.
package main

import (
	"os"

	sdk "github.com/clintjedwards/gofer/sdk/go/extensions"

	"github.com/danielgtaylor/huma/v2"
)

func main() {
	_, apiDesc := sdk.InitRouter(nil, "dummy")
	generateOpenAPISpecFile(apiDesc)
}

// Generates OpenAPI Yaml files that other services can use to generate code for Gofer's API.
func generateOpenAPISpecFile(apiDescription huma.API) {
	output, err := apiDescription.OpenAPI().YAML()
	if err != nil {
		panic(err)
	}

	file, err := os.Create("sdk/openapi.yaml")
	if err != nil {
		panic(err)
	}
	defer file.Close()

	_, err = file.Write(output)
	if err != nil {
		panic(err)
	}
}
