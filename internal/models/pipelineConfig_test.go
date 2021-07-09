package models

import (
	"testing"

	"github.com/google/go-cmp/cmp"
)

func TestFullParse(t *testing.T) {
	config := []byte(`
	id = "test_pipeline"
	name = "test pipeline"
	description = "a simple test pipeline"

	trigger "cron" "every_single_minute" {
		expression = "* * * * *"
	}

	trigger "interval" "every_20_seconds" {
		every = "20"
	}

	task "1" {
		description = "test description"
		image_name = "hello_world"
		depends_on = {
			"2": "any",
			"3": "successful",
		}
		env_vars = {
			"LOGS_HEADER": "example test string 123",
		}
		secrets = {
			"SECRET_LOGS_HEADER": "example/config/for/secrets"
		}
	}

	task "2" {
		description = "test description 1"
		image_name = "hello_world"
	}
		`)

	expected := &PipelineConfig{
		ID:          "test_pipeline",
		Name:        "test pipeline",
		Description: "a simple test pipeline",
		Tasks: []Task{
			{
				ID:          "1",
				Description: "test description",
				DependsOn: map[string]RequiredParentState{
					"2": RequiredParentStateAny,
					"3": RequiredParentStateSuccess,
				},
				ImageName: "hello_world",
				EnvVars: map[string]string{
					"LOGS_HEADER": "example test string 123",
				},
				Secrets: map[string]string{
					"SECRET_LOGS_HEADER": "example/config/for/secrets",
				},
			},
			{ID: "2", Description: "test description 1", ImageName: "hello_world", DependsOn: map[string]RequiredParentState{}},
		},
		Triggers: []PipelineTriggerConfig{
			{
				Kind:   "cron",
				Label:  "every_single_minute",
				Config: map[string]string{"expression": "* * * * *"},
			},
			{
				Kind:   "interval",
				Label:  "every_20_seconds",
				Config: map[string]string{"every": "20"},
			},
		},
	}

	hclconf := HCLPipelineConfig{}
	err := hclconf.FromBytes(config, "test.hcl")
	if err != nil {
		t.Fatal(err)
	}

	conf, err := FromHCL(&hclconf)
	if err != nil {
		t.Fatal(err)
	}

	diff := cmp.Diff(expected, conf)
	if diff != "" {
		t.Errorf("result is different than expected(-want +got):\n%s", diff)
	}
}

func TestIsRestrictedCharSet(t *testing.T) {
	tests := map[string]struct {
		name     string
		hasError bool
	}{
		"well-behaved": {
			name:     "A_Well_behaved_name",
			hasError: false,
		},
		"spaces": {
			name:     "spaces are not allowed",
			hasError: true,
		},
		"special": {
			name:     "$$$special_chars_are_not_allowed_$@#@#",
			hasError: true,
		},
	}

	for name, tc := range tests {
		t.Run(name, func(r *testing.T) {
			err := isRestrictedCharSet(tc.name)
			if err != nil && tc.hasError == false {
				r.Errorf("isRestrictedCharSet unexpected result for name %q", tc.name)
			}
			if err == nil && tc.hasError == true {
				r.Errorf("isRestrictedCharSet unexpected result for name %q", tc.name)
			}
		})
	}
}
