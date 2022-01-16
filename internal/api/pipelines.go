package api

import (
	"bytes"
	"context"
	"fmt"
	"io"
	"io/ioutil"
	"os"
	"reflect"
	"strings"
	"time"

	"github.com/clintjedwards/gofer/internal/models"
	"github.com/clintjedwards/gofer/internal/storage"
	sdkProto "github.com/clintjedwards/gofer/sdk/proto"
	getter "github.com/hashicorp/go-getter/v2"
	"github.com/rs/zerolog/log"
	"google.golang.org/grpc/metadata"
)

// getRemoteDirectory finds and downloads a remote directory to a temporary directory in the destination provided and
// returns the path of the download.
func getRemoteDirectory(src, dst string) (string, error) {
	tmpDir, err := ioutil.TempDir(dst, "gofer_*")
	if err != nil {
		return "", fmt.Errorf("could not create temporary folder to download config file: %w", err)
	}

	result, err := getter.Get(context.Background(), tmpDir, src)
	if err != nil {
		return "", fmt.Errorf("could not download config file: %w", err)
	}

	return result.Dst, nil
}

// processConfigurationByURL goes through the full pipeline configuration parsing process. It ties together several
// functions to give the caller an easier time extracting the config object from a given config URL.
func (api *API) processConfigurationByURL(configURL string) (*models.HCLPipelineConfig, error) {
	configDirPath, err := getRemoteDirectory(configURL, api.config.Server.TmpDir)
	if err != nil {
		return nil, fmt.Errorf("could not retrieve configuration: %w", err)
	}

	configFileRaw, err := parseConfigDir(configDirPath)
	if err != nil {
		return nil, fmt.Errorf("could not parse configuration: %w", err)
	}

	config := models.HCLPipelineConfig{}
	err = config.FromBytes(configFileRaw, configURL)
	if err != nil {
		return nil, fmt.Errorf("could not parse configuration file: %w", err)
	}

	err = os.RemoveAll(configDirPath)
	if err != nil {
		log.Error().Err(err).Msg("Could not cleanup directory meant for temporary config parsing storage")
	}

	return &config, nil
}

// parseConfigDir parses a particular directory for .hcl files and combines them into a single byte slice.
// Useful for parsing the entire configuration at once.
func parseConfigDir(path string) ([]byte, error) {
	combinedFile := bytes.Buffer{}

	files, err := ioutil.ReadDir(path)
	if err != nil {
		return nil, fmt.Errorf("could not parse config directory: %w", err)
	}

	for _, fileInfo := range files {
		if fileInfo.IsDir() || !strings.HasSuffix(fileInfo.Name(), "hcl") {
			continue
		}

		srcFile, err := os.Open(path + "/" + fileInfo.Name())
		if err != nil {
			return nil, fmt.Errorf("could not parse file in config directory: %w", err)
		}

		_, err = io.Copy(&combinedFile, srcFile)
		if err != nil {
			return nil, fmt.Errorf("could not parse file in config directory: %w", err)
		}

		srcFile.Close()
	}

	return combinedFile.Bytes(), nil
}

// validateConfigTriggers makes sure the triggers in a potential trigger config exists within the currently registered
// API triggers.
func (api *API) configTriggersIsValid(triggers []models.PipelineTriggerConfig) error {
	for _, potentialTrigger := range triggers {
		_, exists := api.triggers[potentialTrigger.Kind]
		if !exists {
			return fmt.Errorf("could not find trigger %q in gofer registered triggers: %w", potentialTrigger.Kind, ErrTriggerNotFound)
		}
	}

	return nil
}

// createPipeline creates a new pipeline based on configuration. If the pipeline triggers mention in the config
//  don't exist the entire pipeline creation process is cancelled.
func (api *API) createPipeline(location string, config *models.PipelineConfig) (*models.Pipeline, error) {
	newPipeline := models.NewPipeline(location, config)
	newPipeline.State = models.PipelineStateActive

	err := api.configTriggersIsValid(config.Triggers)
	if err != nil {
		return nil, err
	}

	err = api.storage.AddPipeline(storage.AddPipelineRequest{Pipeline: newPipeline})
	if err != nil {
		return nil, err
	}

	for _, triggerConfig := range config.Triggers {
		err := api.subscribeTrigger(newPipeline.Namespace, newPipeline.ID, &triggerConfig)
		if err != nil {
			unsubErr := api.unsubscribeAllTriggers(newPipeline)
			if unsubErr != nil {
				return nil, err
			}

			return nil, err
		}

		log.Debug().Str("kind", triggerConfig.Kind).Str("trigger_label", triggerConfig.Label).Str("pipeline_id", newPipeline.ID).
			Msg("successfully subscribed trigger")
	}

	return newPipeline, nil
}

// unsubscribeAllTriggers removes all triggers from a particular pipeline.
func (api *API) unsubscribeAllTriggers(pipeline *models.Pipeline) error {
	for label := range pipeline.Triggers {
		err := api.unsubscribeTrigger(label, pipeline)
		if err != nil {
			log.Error().Err(err).Msg("could not unsubscribe trigger")
			continue
		}
		delete(pipeline.Triggers, label)
	}

	err := api.storage.UpdatePipeline(storage.UpdatePipelineRequest{
		Pipeline: pipeline,
	})
	if err != nil {
		return err
	}

	return nil
}

// unsubscribeTrigger
func (api *API) unsubscribeTrigger(triggerLabel string, pipeline *models.Pipeline) error {
	subscription, exists := pipeline.Triggers[triggerLabel]
	if !exists {
		return fmt.Errorf("could not find trigger label %q; %w", triggerLabel, ErrTriggerNotFound)
	}

	trigger, exists := api.triggers[subscription.Kind]
	if !exists {
		return fmt.Errorf("could not find trigger kind %q in registered trigger list", subscription.Kind)
	}

	conn, err := grpcDial(trigger.URL)
	if err != nil {
		log.Error().Err(err).Str("kind", trigger.Kind).Msg("could not connect to trigger")
	}
	defer conn.Close()

	client := sdkProto.NewTriggerClient(conn)

	ctx := metadata.AppendToOutgoingContext(context.Background(), "authorization", "Bearer "+string(trigger.Key))
	_, err = client.Unsubscribe(ctx, &sdkProto.UnsubscribeRequest{
		PipelineTriggerLabel: subscription.Label,
		PipelineId:           pipeline.ID,
		NamespaceId:          pipeline.Namespace,
	})
	if err != nil {
		log.Error().Err(err).Str("namespace", pipeline.Namespace).
			Str("pipeline", pipeline.ID).Str("trigger_label", subscription.Label).
			Str("kind", trigger.Kind).Msg("could not unsubscribe from trigger")
		return err
	}

	log.Debug().Str("namespace", pipeline.Namespace).
		Str("pipeline", pipeline.ID).Str("trigger_label", subscription.Label).
		Str("kind", trigger.Kind).Msg("unsubscribed trigger")

	return nil
}

// updatePipeline makes the necessary change to the pipeline and handles the sync between pipeline and trigger
// before saving the updated pipeline.
//
// We attempt to update trigger subscriptions by matching user provided names in both profiles.
// If the names don't match or the settings are different then we must unsubscribe and resubscribe the triggers.
func (api *API) updatePipeline(url, namespace, id string, hclConfig *models.HCLPipelineConfig) (*models.Pipeline, error) {
	// Get the old pipeline first so that we can store the old values that we need before inserting
	// the new values from the content buffer.
	currentPipeline, err := api.storage.GetPipeline(storage.GetPipelineRequest{
		NamespaceID: namespace,
		ID:          id,
	})
	if err != nil {
		return nil, fmt.Errorf("could not get previous pipeline: %w", err)
	}

	if currentPipeline.State == models.PipelineStateAbandoned {
		return nil, ErrPipelineAbandoned
	}

	// Before update can continue pipeline must be disabled and inactive.
	if currentPipeline.State == models.PipelineStateActive {
		return nil, ErrPipelineActive
	}

	if api.hasActiveRuns(currentPipeline.Namespace, currentPipeline.ID) {
		return nil, ErrPipelineRunsInProgress
	}

	config, err := models.FromHCL(hclConfig)
	if err != nil {
		return nil, fmt.Errorf("could not parse config file; %w", err)
	}

	// store currentTriggers so we can compare them to potential to figure out which need to be unsubscribed.
	currentTriggerMap := currentPipeline.Triggers
	currentTriggers := []models.PipelineTriggerConfig{}
	for _, triggerConfig := range currentTriggerMap {
		currentTriggers = append(currentTriggers, triggerConfig)
	}

	currentPipeline.FromConfig(config)
	currentPipeline.Location = url
	currentPipeline.Updated = time.Now().UnixMilli()

	if currentPipeline.Namespace == "" {
		currentPipeline.Namespace = namespace
	}

	err = api.configTriggersIsValid(config.Triggers)
	if err != nil {
		return nil, err
	}

	// Find the list of triggers we should unsubscribe by comparing what we have currently to the list of unchanged
	// triggers.
	// 2) For anything new that shows up we add to a subscribe list
	// 3) For anything that is not different by name we make sure it not different by config
	// IF it is different by config, we unsubscribe the old one and subscribe the new one
	// 4) For things that aren't different by name OR config we do nothing.
	removals, additions := findTriggerDifferences(currentTriggers, config.Triggers)

	for _, sub := range removals {
		// TODO(clintjedwards): This is best effort, but it probably shouldn't be.
		// maybe when we fail to unsubscribe a trigger and get into this half state we should
		// disable the pipeline.
		_ = api.unsubscribeTrigger(sub.Label, currentPipeline)
	}

	for _, newTrigger := range additions {
		err := api.subscribeTrigger(currentPipeline.Namespace, currentPipeline.ID, &newTrigger)
		if err != nil {
			return nil, err
		}
	}

	err = api.storage.UpdatePipeline(storage.UpdatePipelineRequest{Pipeline: currentPipeline})
	if err != nil {
		return nil, err
	}

	return currentPipeline, nil
}

// findTriggerDifferences returns the trigger subscriptions that should be removed and should be added. It compares
// the incoming trigger list to the current trigger list and uses the name, kind, and config to determine the difference
// between them.
func findTriggerDifferences(current, potential []models.PipelineTriggerConfig) (
	removeableTriggers []models.PipelineTriggerConfig,
	pendingTriggers []models.PipelineTriggerConfig,
) {
	unchangedTriggers := []models.PipelineTriggerConfig{}

	// First find all triggers between current and potential which remain the same
	for _, potentialTrigger := range potential {
		for _, currentTrigger := range current {
			if reflect.DeepEqual(potentialTrigger, currentTrigger) {
				unchangedTriggers = append(unchangedTriggers, potentialTrigger)
			}
		}
	}

	// Once we have a list of triggers that haven't changed, anything outside the list that was previously registered
	// needs to be unsubscribed.
	for _, currentTrigger := range current {
		exists := false
		for _, unchangedTrigger := range unchangedTriggers {
			if reflect.DeepEqual(unchangedTrigger, currentTrigger) {
				exists = true
				break
			}
		}

		if !exists {
			removeableTriggers = append(removeableTriggers, currentTrigger)
		}
	}

	// Anything that is in our potential list that isn't in the unchanged list must be added.
	for _, potentialTrigger := range potential {
		exists := false
		for _, unchangedTrigger := range unchangedTriggers {
			if reflect.DeepEqual(unchangedTrigger, potentialTrigger) {
				exists = true
				break
			}
		}

		if !exists {
			pendingTriggers = append(pendingTriggers, potentialTrigger)
		}
	}

	return removeableTriggers, pendingTriggers
}

// hasActiveRuns checks to see if the last 10 runs in a pipeline has a running state.
func (api *API) hasActiveRuns(namespace, id string) bool {
	runs, err := api.storage.GetAllRuns(storage.GetAllRunsRequest{NamespaceID: namespace, PipelineID: id, Offset: 0, Limit: 10})
	if err != nil {
		return true
	}

	for _, run := range runs {
		if run.State == models.RunProcessing || run.State == models.RunRunning || run.State == models.RunWaiting {
			return true
		}
	}

	return false
}

// subscribeTrigger takes a pipeline config requested trigger and communicates with the trigger container
// in order appropriately make sure the trigger is aware for the pipeline.
func (api *API) subscribeTrigger(namespaceID, pipelineID string, config *models.PipelineTriggerConfig) error {
	trigger, exists := api.triggers[config.Kind]
	if !exists {
		return fmt.Errorf("trigger %q not found;", config.Kind)
	}

	conn, err := grpcDial(trigger.URL)
	if err != nil {
		log.Error().Err(err).Str("kind", trigger.Kind).Msg("could not connect to trigger")
	}
	defer conn.Close()

	client := sdkProto.NewTriggerClient(conn)

	subConfig, err := api.populateSecrets(namespaceID, pipelineID, config.Config)
	if err != nil {
		return fmt.Errorf("could not subscribe trigger %q for pipeline %q - namespace %q; %w",
			config.Label, pipelineID, namespaceID, err)
	}

	ctx := metadata.AppendToOutgoingContext(context.Background(), "authorization", "Bearer "+string(trigger.Key))
	_, err = client.Subscribe(ctx, &sdkProto.SubscribeRequest{
		NamespaceId:          namespaceID,
		PipelineTriggerLabel: config.Label,
		PipelineId:           pipelineID,
		Config:               subConfig,
	})
	if err != nil {
		log.Error().Err(err).Str("kind", trigger.Kind).Msg("could not subscribe to trigger")
		return err
	}

	log.Debug().Str("pipeline", pipelineID).Str("trigger_kind", config.Kind).
		Str("trigger_label", config.Label).Msg("subscribed pipeline to trigger")

	return nil
}

// collectAllPipelines attempts to return a single list of all pipelines within the gofer service.
// Useful for functions where we must operate globally.
func (api *API) collectAllPipelines() ([]*models.Pipeline, error) {
	allNamespaces := []*models.Namespace{}

	offset := 0
	for {
		namespaces, err := api.storage.GetAllNamespaces(storage.GetAllNamespacesRequest{Offset: offset})
		if err != nil {
			return []*models.Pipeline{}, fmt.Errorf("could not get all namespaces; %w", err)
		}

		if len(namespaces) == 0 {
			break
		}

		allNamespaces = append(allNamespaces, namespaces...)
		offset += 100
	}

	allPipelines := []*models.Pipeline{}

	for _, namespace := range allNamespaces {
		offset := 0
		for {
			pipelines, err := api.storage.GetAllPipelines(storage.GetAllPipelinesRequest{
				Offset:      offset,
				NamespaceID: namespace.ID,
			})
			if err != nil {
				return []*models.Pipeline{}, fmt.Errorf("could not get all pipelines; %w", err)
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
