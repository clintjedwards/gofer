package main

import (
	"fmt"
	"reflect"
	"testing"

	"github.com/google/go-cmp/cmp"
)

func TestParseEventFilter(t *testing.T) {
	tests := []struct {
		name        string
		input       string
		wantEvent   string
		wantActions []string
	}{
		{
			name:        "No Action Provided",
			input:       "event",
			wantEvent:   "event",
			wantActions: []string{},
		},
		{
			name:        "Single Action",
			input:       "event/action",
			wantEvent:   "event",
			wantActions: []string{"action"},
		},
		{
			name:        "Multiple Actions",
			input:       "event/action1,action2",
			wantEvent:   "event",
			wantActions: []string{"action1", "action2"},
		},
		{
			name:        "Action Casing",
			input:       "event/AcTiOn",
			wantEvent:   "event",
			wantActions: []string{"action"},
		},
		{
			name:        "Empty Actions",
			input:       "event/",
			wantEvent:   "event",
			wantActions: []string{},
		},
		{
			name:        "Action With Trailing Comma",
			input:       "event/action1,",
			wantEvent:   "event",
			wantActions: []string{"action1"},
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			gotEvent, gotActions := parseEventFilter(tt.input)
			if gotEvent != tt.wantEvent {
				t.Errorf("Expected event %s, got %s", tt.wantEvent, gotEvent)
			}
			if !reflect.DeepEqual(gotActions, tt.wantActions) {
				fmt.Printf("WANT: %T\n", tt.wantActions)
				fmt.Printf("GOT: %T\n", gotActions)
				t.Errorf("Expected actions %v, got %v", tt.wantActions, gotActions)
			}
		})
	}
}

func TestMatchSubscriptions(t *testing.T) {
	tests := []struct {
		name              string
		extensionSetup    extension
		event             string
		repo              string
		action            string
		wantSubscriptions []pipelineSubscription
	}{
		{
			name: "No Subscriptions",
			extensionSetup: extension{
				subscriptions: map[string]map[string][]pipelineSubscription{},
			},
			event:             "push",
			repo:              "repo1",
			action:            "create",
			wantSubscriptions: []pipelineSubscription{},
		},
		{
			name: "Match By Event And Repo",
			extensionSetup: extension{
				subscriptions: map[string]map[string][]pipelineSubscription{
					"push": {
						"repo1": {
							{actions: map[string]struct{}{"create": {}}},
						},
					},
				},
			},
			event:  "push",
			repo:   "repo1",
			action: "",
			wantSubscriptions: []pipelineSubscription{
				{actions: map[string]struct{}{"create": {}}},
			},
		},
		{
			name: "Action Case Insensitive",
			extensionSetup: extension{
				subscriptions: map[string]map[string][]pipelineSubscription{
					"push": {
						"repo1": {
							{actions: map[string]struct{}{"create": {}}},
						},
					},
				},
			},
			event:  "push",
			repo:   "repo1",
			action: "CREATE",
			wantSubscriptions: []pipelineSubscription{
				{actions: map[string]struct{}{"create": {}}},
			},
		},
		{
			name: "Match Multiple Actions",
			extensionSetup: extension{
				subscriptions: map[string]map[string][]pipelineSubscription{
					"push": {
						"repo1": {
							{actions: map[string]struct{}{"create": {}, "delete": {}}},
						},
					},
				},
			},
			event:  "push",
			repo:   "repo1",
			action: "delete",
			wantSubscriptions: []pipelineSubscription{
				{actions: map[string]struct{}{"create": {}, "delete": {}}},
			},
		},
		{
			name: "Event and Repo Match, Action Does Not",
			extensionSetup: extension{
				subscriptions: map[string]map[string][]pipelineSubscription{
					"push": {
						"repo1": {
							{actions: map[string]struct{}{"create": {}}},
						},
					},
				},
			},
			event:             "push",
			repo:              "repo1",
			action:            "update",
			wantSubscriptions: []pipelineSubscription{},
		},
		{
			name: "Event Does Not Exist",
			extensionSetup: extension{
				subscriptions: map[string]map[string][]pipelineSubscription{
					"push": {
						"repo1": {{actions: map[string]struct{}{"create": {}}}},
					},
				},
			},
			event:             "pull_request",
			repo:              "repo1",
			action:            "create",
			wantSubscriptions: []pipelineSubscription{},
		},
		{
			name: "Multiple Repos, One Match",
			extensionSetup: extension{
				subscriptions: map[string]map[string][]pipelineSubscription{
					"push": {
						"repo1": {{actions: map[string]struct{}{"create": {}}}},
						"repo2": {{actions: map[string]struct{}{"delete": {}}}},
					},
				},
			},
			event:  "push",
			repo:   "repo1",
			action: "create",
			wantSubscriptions: []pipelineSubscription{
				{actions: map[string]struct{}{"create": {}}},
			},
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			gotSubscriptions := tt.extensionSetup.matchSubscriptions(tt.event, tt.repo, tt.action)

			// Using cmp for deep comparisons
			if diff := cmp.Diff(tt.wantSubscriptions, gotSubscriptions, cmp.AllowUnexported(pipelineSubscription{})); diff != "" {
				t.Errorf("mismatch (-want +got):\n%s", diff)
			}
		})
	}
}

// func TestMatchSubscriptions(t *testing.T) {
// 	extension := extension{
// 		subscriptions: map[string]map[string][]pipelineSubscription{},
// 	}

// 	_, err := extension.Subscribe(context.Background(), &proto.ExtensionSubscribeRequest{
// 		PipelineExtensionLabel: "test_extension",
// 		NamespaceId:            "test_namespace",
// 		PipelineId:             "test_pipeline",
// 		Config: map[string]string{
// 			"events":     "push,create",
// 			"repository": "clintjedwards/experimental",
// 		},
// 	})
// 	if err != nil {
// 		t.Fatal(err)
// 	}

// 	subs1 := extension.matchSubscriptions("create", "clintjedwards/experimental")
// 	subs2 := extension.matchSubscriptions("pull_request", "clintjedwards/experimental")

// 	result1 := []pipelineSubscription{
// 		{
// 			eventFilter:    "create",
// 			repository:     "clintjedwards/experimental",
// 			extensionLabel: "test_extension",
// 			namespace:      "test_namespace",
// 			pipeline:       "test_pipeline",
// 		},
// 	}

// 	result2 := []pipelineSubscription{}

// 	if diff := cmp.Diff(result1, subs1, cmp.AllowUnexported(pipelineSubscription{})); diff != "" {
// 		t.Errorf("mismatch (-want +got):\n%s", diff)
// 	}

// 	if diff := cmp.Diff(result2, subs2, cmp.AllowUnexported(pipelineSubscription{})); diff != "" {
// 		t.Errorf("mismatch (-want +got):\n%s", diff)
// 	}
// }
