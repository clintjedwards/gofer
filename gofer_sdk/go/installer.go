package sdk

import (
	"context"
	"crypto/tls"
	"fmt"
	"os"
	"strings"

	"github.com/clintjedwards/gofer/proto"
	"google.golang.org/grpc"
	"google.golang.org/grpc/credentials"
	"google.golang.org/grpc/metadata"
)

// installData is a representation of information needed by trigger/notifier installer scripts.
type installData struct {
	Kind  string
	Host  string // The url of the Gofer server.
	Token string
	Image string
	User  string
	Pass  string
}

// panics if env var is empty
func getEnv(key string) (string, error) {
	value := os.Getenv(key)
	if value != "" {
		return value, nil
	}

	return "", fmt.Errorf("value for key %q cannot be empty", key)
}

// getInstallData returns a data structure with information needed by typical trigger/notification installer
// scripts.
func getInstallData() (installData, error) {
	kind, err := getEnv("GOFER_TRIGGER_INSTALLER_KIND")
	if err != nil {
		return installData{}, err
	}
	host, err := getEnv("GOFER_TRIGGER_INSTALLER_SERVER_HOST")
	if err != nil {
		return installData{}, err
	}
	image, err := getEnv("GOFER_TRIGGER_INSTALLER_IMAGE")
	if err != nil {
		return installData{}, err
	}
	token := os.Getenv("GOFER_TRIGGER_INSTALLER_TOKEN")
	user := os.Getenv("GOFER_TRIGGER_INSTALLER_IMAGE_USER")
	pass := os.Getenv("GOFER_TRIGGER_INSTALLER_IMAGE_PASS")

	return installData{
		Kind:  kind,
		Host:  host,
		Token: token,
		Image: image,
		User:  user,
		Pass:  pass,
	}, nil
}

func connect(url string) (*grpc.ClientConn, error) {
	host, port, _ := strings.Cut(url, ":")

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

// InstallTrigger attempts to install the trigger plugin with the given configuration.
// It attempts to make a connection with the Gofer service and uses the variables passed to it
// in order to complete the installation.
//
// Config needs to be a mapping of the config env var to the proper key
// Example:
// "MIN_DURATION": <value>
func InstallTrigger(config map[string]string) error {
	vars, err := getInstallData()
	if err != nil {
		return err
	}

	conn, err := connect(vars.Host)
	if err != nil {
		return err
	}

	triggerConfig := &proto.TriggerConfig{
		Kind:    vars.Kind,
		Image:   vars.Image,
		User:    vars.User,
		Pass:    vars.Pass,
		EnvVars: config,
	}

	client := proto.NewGoferClient(conn)
	md := metadata.Pairs("Authorization", "Bearer "+vars.Token)
	ctx := metadata.NewOutgoingContext(context.Background(), md)
	_, err = client.InstallTrigger(ctx, &proto.InstallTriggerRequest{
		Trigger: triggerConfig,
	})
	if err != nil {
		return err
	}

	return nil
}

// InstallNotifier attempts to install the notifier plugin with the given configuration.
// It attempts to make a connection with the Gofer service and uses the variables passed to it
// in order to complete the installation.
//
// Config needs to be a mapping of the config env var to the proper key
// Example:
// "MIN_DURATION": <value>
func InstallNotifier(config map[string]string) error {
	vars, err := getInstallData()
	if err != nil {
		return err
	}

	conn, err := connect(vars.Host)
	if err != nil {
		return err
	}

	notifierConfig := &proto.NotifierConfig{
		Kind:    vars.Kind,
		Image:   vars.Image,
		User:    vars.User,
		Pass:    vars.Pass,
		EnvVars: config,
	}

	client := proto.NewGoferClient(conn)
	md := metadata.Pairs("Authorization", "Bearer "+vars.Token)
	ctx := metadata.NewOutgoingContext(context.Background(), md)
	_, err = client.InstallNotifier(ctx, &proto.InstallNotifierRequest{
		Notifier: notifierConfig,
	})
	if err != nil {
		return err
	}

	return nil
}
