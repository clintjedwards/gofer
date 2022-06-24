package sdk

import (
	"context"

	"github.com/clintjedwards/gofer/gofer_sdk/go/proto"
	"google.golang.org/grpc/metadata"
)

// UninstallTrigger attempts to uninstall the trigger plugin with the given configuration.
// It attempts to make a connection with the Gofer service and uses the variables passed to it
// in order to complete the installation.
//
// Config needs to be a mapping of the config env var to the proper key
// Example:
// "MIN_DURATION": <value>
func UninstallTrigger() error {
	vars, err := getInstallData()
	if err != nil {
		return err
	}

	conn, err := connect(vars.Host)
	if err != nil {
		return err
	}

	client := proto.NewGoferClient(conn)
	md := metadata.Pairs("Authorization", "Bearer "+vars.Token)
	ctx := metadata.NewOutgoingContext(context.Background(), md)
	_, err = client.UninstallTrigger(ctx, &proto.UninstallTriggerRequest{
		Name: vars.Name,
	})
	if err != nil {
		return err
	}

	return nil
}

// UninstallNotifier attempts to install the notifier plugin with the given configuration.
// It attempts to make a connection with the Gofer service and uses the variables passed to it
// in order to complete the installation.
//
// Config needs to be a mapping of the config env var to the proper key
// Example:
// "MIN_DURATION": <value>
func UninstallNotifier() error {
	vars, err := getInstallData()
	if err != nil {
		return err
	}

	conn, err := connect(vars.Host)
	if err != nil {
		return err
	}

	client := proto.NewGoferClient(conn)
	md := metadata.Pairs("Authorization", "Bearer "+vars.Token)
	ctx := metadata.NewOutgoingContext(context.Background(), md)
	_, err = client.UninstallNotifier(ctx, &proto.UninstallNotifierRequest{
		Name: vars.Name,
	})
	if err != nil {
		return err
	}

	return nil
}
