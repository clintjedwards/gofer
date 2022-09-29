package main

import (
	"context"
	"testing"

	sdkProto "github.com/clintjedwards/gofer/sdk/proto"
	"github.com/google/go-cmp/cmp"
)

func TestMatchSubscriptions(t *testing.T) {
	trigger := trigger{
		events:        make(chan *sdkProto.CheckResponse, 100),
		subscriptions: map[string]map[string][]pipelineSubscription{},
	}

	_, err := trigger.Subscribe(context.Background(), &sdkProto.SubscribeRequest{
		PipelineTriggerLabel: "test_trigger",
		NamespaceId:          "test_namespace",
		PipelineId:           "test_pipeline",
		Config: map[string]string{
			"events":     "push,create",
			"repository": "clintjedwards/experimental",
		},
	})
	if err != nil {
		t.Fatal(err)
	}

	subs1 := trigger.matchSubscriptions("create", "clintjedwards/experimental")
	subs2 := trigger.matchSubscriptions("pull_request", "clintjedwards/experimental")

	result1 := []pipelineSubscription{
		{
			event:        "create",
			repository:   "clintjedwards/experimental",
			triggerLabel: "test_trigger",
			namespace:    "test_namespace",
			pipeline:     "test_pipeline",
		},
	}

	result2 := []pipelineSubscription{}

	if diff := cmp.Diff(result1, subs1, cmp.AllowUnexported(pipelineSubscription{})); diff != "" {
		t.Errorf("mismatch (-want +got):\n%s", diff)
	}

	if diff := cmp.Diff(result2, subs2, cmp.AllowUnexported(pipelineSubscription{})); diff != "" {
		t.Errorf("mismatch (-want +got):\n%s", diff)
	}
}
