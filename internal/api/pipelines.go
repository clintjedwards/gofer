package api

import (
	"context"
	"fmt"

	"github.com/clintjedwards/gofer/models"
	proto "github.com/clintjedwards/gofer/proto/go"
	sdk "github.com/clintjedwards/gofer/sdk/go"
	"github.com/rs/zerolog/log"
	"google.golang.org/grpc/metadata"
)

// validateConfigTriggers makes sure the triggers in a potential trigger config exists within the currently registered
// API triggers.
func (api *API) configTriggersIsValid(triggers map[string]models.PipelineTriggerSettings) error {
	for _, potentialTrigger := range triggers {
		_, exists := api.triggers.Get(potentialTrigger.Name)
		if !exists {
			return fmt.Errorf("could not find trigger %q in gofer registered triggers: %w", potentialTrigger.Name,
				ErrTriggerNotFound)
		}
	}

	return nil
}

// unsubscribeTriggers removes passed triggers from a particular pipeline object.
// The trigger map is "label" -> "name"
func (api *API) unsubscribeTriggers(namespace, pipeline string, trigger map[string]string) error {
	for label, name := range trigger {
		err := api.unsubscribeTrigger(namespace, pipeline, name, label)
		if err != nil {
			log.Error().Err(err).Msg("could not unsubscribe trigger")
			continue
		}
	}

	return nil
}

// unsubscribeTrigger contacts the trigger and remove the pipeline subscription.
func (api *API) unsubscribeTrigger(namespace, pipeline, name, label string) error {
	trigger, exists := api.triggers.Get(name)
	if !exists {
		return fmt.Errorf("could not find trigger name %q in registered trigger list", name)
	}

	conn, err := grpcDial(trigger.URL)
	if err != nil {
		log.Error().Err(err).Str("name", trigger.Registration.Name).Msg("could not connect to trigger")
	}
	defer conn.Close()

	client := proto.NewTriggerServiceClient(conn)

	ctx := metadata.AppendToOutgoingContext(context.Background(), "authorization", "Bearer "+string(*trigger.Key))
	_, err = client.Unsubscribe(ctx, &proto.TriggerUnsubscribeRequest{
		PipelineTriggerLabel: label,
		NamespaceId:          namespace,
		PipelineId:           pipeline,
	})
	if err != nil {
		log.Error().Err(err).Str("namespace", namespace).
			Str("pipeline", pipeline).Str("trigger_label", label).
			Str("name", trigger.Registration.Name).Msg("could not unsubscribe from trigger")
		return err
	}

	log.Debug().Str("namespace", namespace).
		Str("pipeline", pipeline).Str("trigger_label", label).
		Str("name", trigger.Registration.Name).Msg("unsubscribed trigger")

	return nil
}

// subscribeTriggers takes a list of triggers for a particular pipeline and attempts to subscribe them all.
// It will return an error if one of the subscription processes fails but it will always
// return a list of trigger names that have been successfully subscribed.
// This returned list can be used to rollback subscriptions if need be.
func (api *API) subscribeTriggers(namespace, pipeline string, configs []sdk.PipelineTriggerConfig) ([]sdk.PipelineTriggerConfig, error) {
	successfulSubscriptions := []sdk.PipelineTriggerConfig{}

	for _, config := range configs {
		err := api.subscribeTrigger(namespace, pipeline, &config)
		if err != nil {
			return successfulSubscriptions, fmt.Errorf("could not subscribe to trigger %q (%q)", config.Label, config.Name)
		}

		successfulSubscriptions = append(successfulSubscriptions, config)
	}

	return successfulSubscriptions, nil
}

// subscribeTrigger takes a pipeline config requested trigger and communicates with the trigger container
// in order appropriately make sure the trigger is aware for the pipeline.
func (api *API) subscribeTrigger(namespace, pipeline string, config *sdk.PipelineTriggerConfig) error {
	trigger, exists := api.triggers.Get(config.Name)
	if !exists {
		return fmt.Errorf("trigger %q not found;", config.Name)
	}

	convertedSettings := convertVarsToSlice(config.Settings, models.VariableSourcePipelineConfig)
	interpolatedSettings, err := api.interpolateVars(namespace, pipeline, nil, convertedSettings)
	if err != nil {
		return fmt.Errorf("could not subscribe trigger %q for pipeline %q - namespace %q; %w",
			config.Label, pipeline, namespace, err)
	}
	parsedSettings := convertVarsToMap(interpolatedSettings)

	conn, err := grpcDial(trigger.URL)
	if err != nil {
		log.Error().Err(err).Str("name", trigger.Registration.Name).Msg("could not connect to trigger")
	}
	defer conn.Close()

	client := proto.NewTriggerServiceClient(conn)

	ctx := metadata.AppendToOutgoingContext(context.Background(), "authorization", "Bearer "+string(*trigger.Key))
	_, err = client.Subscribe(ctx, &proto.TriggerSubscribeRequest{
		NamespaceId:          namespace,
		PipelineTriggerLabel: config.Label,
		PipelineId:           pipeline,
		Config:               parsedSettings,
	})
	if err != nil {
		log.Error().Err(err).Str("name", trigger.Registration.Name).Msg("could not subscribe to trigger")
		return err
	}

	log.Debug().Str("pipeline", pipeline).Str("trigger_name", config.Name).
		Str("trigger_label", config.Label).Msg("subscribed pipeline to trigger")

	return nil
}

// collectAllPipelines attempts to return a single list of all pipelines within the gofer service.
// Useful for functions where we must operate globally.
func (api *API) collectAllPipelines() ([]models.Pipeline, error) {
	allNamespaces := []models.Namespace{}

	offset := 0
	for {
		namespaces, err := api.db.ListNamespaces(offset, 0)
		if err != nil {
			return []models.Pipeline{}, fmt.Errorf("could not get all namespaces; %w", err)
		}

		if len(namespaces) == 0 {
			break
		}

		allNamespaces = append(allNamespaces, namespaces...)
		offset += 100
	}

	allPipelines := []models.Pipeline{}

	for _, namespace := range allNamespaces {
		offset := 0
		for {
			pipelines, err := api.db.ListPipelines(offset, 0, namespace.ID)
			if err != nil {
				return []models.Pipeline{}, fmt.Errorf("could not get all pipelines; %w", err)
			}

			if len(pipelines) == 0 {
				break
			}

			allPipelines = append(allPipelines, pipelines...)
			offset += 100
		}
	}

	return allPipelines, nil
}
