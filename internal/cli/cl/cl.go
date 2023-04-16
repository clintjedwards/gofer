// Package cl contains global variables used across the cli package. Yeah its probably a bad pattern
// but it works and removes us from dependency hell.
package cl

import (
	"crypto/tls"
	"fmt"
	"log"
	"os"
	"strings"

	"github.com/clintjedwards/gofer/internal/config"
	"github.com/clintjedwards/polyfmt"
	"github.com/fatih/color"
	"github.com/hashicorp/hcl/v2/gohcl"
	"github.com/hashicorp/hcl/v2/hclwrite"
	"github.com/spf13/cobra"
	"google.golang.org/grpc"
	"google.golang.org/grpc/credentials"
)

// Harness is a structure for values that all commands need access to.
type Harness struct {
	Fmt            polyfmt.Formatter
	Config         *config.CLI
	ConfigFilePath string
}

// State holds values that aid in the lifetime of a command.
var State *Harness

func (s *Harness) Connect() (*grpc.ClientConn, error) {
	host, port, _ := strings.Cut(s.Config.Host, ":")

	// If we are not given a port we assume that port is 443
	if port == "" {
		port = "443"
	}

	var opt []grpc.DialOption
	var tlsConf *tls.Config
	if host == "localhost" || host == "127.0.0.1" {
		tlsConf = &tls.Config{
			InsecureSkipVerify: true,
		}
	}

	opt = append(opt, grpc.WithTransportCredentials(credentials.NewTLS(tlsConf)))
	conn, err := grpc.Dial(fmt.Sprintf("%s:%s", host, port), opt...)
	if err != nil {
		return nil, fmt.Errorf("could not connect to server: %w", err)
	}

	return conn, nil
}

// Init harness for command line functions, used to provide different functionality during the life of a command line run.
func InitState(cmd *cobra.Command) {
	// Including these in the pre run hook instead of in the enclosing/parent command definition
	// allows cobra to still print errors and usage for its own cli verifications, but
	// ignore our errors.
	cmd.SilenceUsage = true  // Don't print the usage if we get an upstream error
	cmd.SilenceErrors = true // Let us handle error printing ourselves

	// Now we need to provide the command line with some state which we use to display the spinner
	// and also make sure the command line inherits the proper variable chain(config file -> envvar -> flags)
	State = &Harness{}

	// This is a hack. Because the start command needs to use the --config global variable for its own purposes
	// we tell it to skip parsing the as if its a CLI config and supply it with some defaults.
	if cmd.Name() == "start" && cmd.Parent().Name() == "service" {
		State.Config = &config.CLI{
			Format: "silent",
		}
	} else {
		config, _ := cmd.Flags().GetString("config")
		State.NewConfig(config)
	}

	// Initiate the formatter(this controls the command line output)
	format, _ := cmd.Flags().GetString("format")
	if format != "" {
		State.Config.Format = format
	}

	State.NewFormatter()

	overlayGlobalFlags(cmd)
}

// Flags are the last possible way to provide variables to the command line. For global variables we allow the user
// to specify them through envvars and configuration. Because of this we need to take whatever we have in the config
// from previous steps that retrieve them from those locations and then if the user has passed in a flag overwrite
// whatever those variables are.
func overlayGlobalFlags(cmd *cobra.Command) {
	// Now we include all other global flags into the config. Flags are always highest on the variable chain.
	noColor, _ := cmd.Flags().GetBool("no-color")
	if noColor {
		color.NoColor = true // turn off color globally
		State.Config.NoColor = noColor
	}

	detail, _ := cmd.Flags().GetBool("detail")
	if detail {
		State.Config.Detail = detail
	}

	namespace, _ := cmd.Flags().GetString("namespace")
	if namespace != "" {
		State.Config.Namespace = namespace
	}

	host, _ := cmd.Flags().GetString("host")
	if host != "" {
		State.Config.Host = host
	}
}

func (s *Harness) NewFormatter() {
	clifmt, err := polyfmt.NewFormatter(polyfmt.Mode(s.Config.Format), polyfmt.DefaultOptions())
	if err != nil {
		log.Fatal(err)
	}

	s.Fmt = clifmt
}

func (s *Harness) NewConfig(configPath string) {
	config, err := config.InitCLIConfig(configPath, true)
	if err != nil {
		log.Fatal(err)
	}

	s.Config = config
	s.ConfigFilePath = configPath
}

// writeConfig takes the current representation of config and writes it to the file.
func (s *Harness) WriteConfig() error {
	if s.ConfigFilePath == "" {
		homeDir, _ := os.UserHomeDir()
		s.ConfigFilePath = fmt.Sprintf("%s/%s", homeDir, ".gofer.hcl")
	}

	f := hclwrite.NewEmptyFile()

	gohcl.EncodeIntoBody(s.Config, f.Body())

	err := os.WriteFile(s.ConfigFilePath, f.Bytes(), 0o644)
	if err != nil {
		return err
	}

	return nil
}
