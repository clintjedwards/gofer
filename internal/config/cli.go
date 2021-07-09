package config

import (
	"fmt"
	"os"

	"github.com/hashicorp/hcl/v2/hclsimple"
	"github.com/kelseyhightower/envconfig"
)

type CLI struct {
	Namespace string `split_words:"true" hcl:"namespace,optional"`
	Detail    bool   `hcl:"detail,optional"`
	Format    string `hcl:"format,optional"`
	Host      string `hcl:"host,optional"`
	NoColor   bool   `split_words:"true" hcl:"no_color,optional"`
	Token     string `hcl:"token,optional"`
}

// DefaultCLIConfig returns a pre-populated configuration struct that is used as the base for super imposing user configuration
// settings.
func DefaultCLIConfig() *CLI {
	return &CLI{
		Host:   "localhost:8080",
		Format: "pretty",
	}
}

// FromEnv parses environment variables into the config object based on envconfig name
func (c *CLI) FromEnv() error {
	err := envconfig.Process("gofer_cli", c)
	if err != nil {
		return err
	}

	return nil
}

// FromBytes attempts to parse a given HCL configuration.
func (c *CLI) FromBytes(content []byte) error {
	err := hclsimple.Decode("config.hcl", content, nil, c)
	if err != nil {
		return err
	}

	c.convertDurationFromHCL()

	return nil
}

func (c *CLI) FromFile(path string) error {
	err := hclsimple.DecodeFile(path, nil, c)
	if err != nil {
		return err
	}

	c.convertDurationFromHCL()

	return nil
}

// convertDurationFromHCL attempts to move the string value of a duration written in HCL to
// the real time.Duration type. This is needed due to advanced types like time.Duration being not handled particularly
// well during HCL parsing: https://github.com/hashicorp/hcl/issues/202
func (c *CLI) convertDurationFromHCL() {}

// Get the final configuration for the CLI.
// This involves correctly finding and ordering different possible paths for the configuration file.
//
// 1) The function is intended to be called with paths gleaned from the -config flag
// 2) Then combine that with possible other config locations that the user might store a config file.
// 3) Then try to see if the user has set an envvar for the config file, which overrides
// all previous config file paths.
// 4) Finally, pass back whatever is deemed the final config path from that process.
//
// We then use that path data to find the config file and read it in via HCL parsers. Once that is finished
// we then take any configuration from the environment and superimpose that on top of the final config struct.
func InitCLIConfig(flagPath string) (*CLI, error) {
	// First we initiate the default values for the config.
	config := DefaultCLIConfig()

	homeDir, _ := os.UserHomeDir()
	possibleConfigPaths := []string{
		flagPath,
		fmt.Sprintf("%s/%s", homeDir, ".gofer.hcl"),
		fmt.Sprintf("%s/%s/%s", homeDir, ".config", "gofer.hcl"),
	}

	path := searchFilePaths(possibleConfigPaths...)

	// envVars top all other entries so if its not empty we just insert it over the current path
	// regardless of if we found one.
	envPath := os.Getenv("GOFER_CLI_CONFIG_PATH")
	if envPath != "" {
		path = envPath
	}

	if path != "" {
		err := config.FromFile(path)
		if err != nil {
			return nil, err
		}
	}

	err := config.FromEnv()
	if err != nil {
		return nil, err
	}

	return config, nil
}

func PrintCLIEnvs() error {
	var config CLI
	err := envconfig.Usage("gofer_cli", &config)
	if err != nil {
		return err
	}
	fmt.Println("GOFER_CLI_CONFIG_PATH")

	return nil
}
