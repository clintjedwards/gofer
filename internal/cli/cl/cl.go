// Package cl contains global variables used across the cli package. Yeah its probably a bad pattern
// but it works and removes us from dependency hell.
package cl

import (
	"crypto/tls"
	"fmt"
	"log"
	"strings"

	"github.com/clintjedwards/gofer/internal/config"
	"github.com/clintjedwards/polyfmt"
	"github.com/fatih/color"
	"github.com/spf13/cobra"
	"google.golang.org/grpc"
	"google.golang.org/grpc/credentials"
)

// Harness is a structure for values that all commands need access to.
type Harness struct {
	Fmt    polyfmt.Formatter
	Config *config.CLI
}

// State holds values that aid in the lifetime of a command.
var State *Harness

func (s *Harness) Connect() (*grpc.ClientConn, error) {
	hostPortTuple := strings.Split(s.Config.Host, ":")

	if len(hostPortTuple) != 2 {
		return nil, fmt.Errorf("malformed host string; must be in format: <host>:<port>")
	}

	var opt []grpc.DialOption
	var tlsConf *tls.Config
	if hostPortTuple[0] == "localhost" || hostPortTuple[0] == "127.0.0.1" {
		tlsConf = &tls.Config{
			InsecureSkipVerify: true,
		}
		opt = append(opt, grpc.WithTransportCredentials(credentials.NewTLS(tlsConf)))
	}

	conn, err := grpc.Dial(fmt.Sprintf("%s:%s", hostPortTuple[0], hostPortTuple[1]), opt...)
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
	config, _ := cmd.Flags().GetString("config")
	State.NewConfig(config)

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
	clifmt, err := polyfmt.NewFormatter(polyfmt.Mode(s.Config.Format))
	if err != nil {
		log.Fatal(err)
	}

	s.Fmt = clifmt
}

func (s *Harness) NewConfig(configPath string) {
	config, err := config.InitCLIConfig(configPath)
	if err != nil {
		log.Fatal(err)
	}

	s.Config = config
}
