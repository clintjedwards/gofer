package models

import (
	"io/ioutil"
	"os"
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

	task "1" "hello_world" {
		description = "test description"
		registry_auth {
			user = "obama"
			pass = "{{secret}}"
		}
		depends_on = {
			"2": "any",
			"3": "successful",
		}
		env_vars = {
			"LOGS_HEADER": "example test string 123",
			"SECRET_LOGS_HEADER": "{{ secret_logs_header }}"
		}
	}

	task "2" "hello_world" {
		description = "test description 1"
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
				RegistryAuth: RegistryAuth{
					User: "obama",
					Pass: "{{secret}}",
				},
				DependsOn: map[string]RequiredParentState{
					"2": RequiredParentStateAny,
					"3": RequiredParentStateSuccess,
				},
				Image: "hello_world",
				EnvVars: map[string]string{
					"LOGS_HEADER":        "example test string 123",
					"SECRET_LOGS_HEADER": "{{ secret_logs_header }}",
				},
			},
			{ID: "2", Description: "test description 1", Image: "hello_world", DependsOn: map[string]RequiredParentState{}},
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

func TestExampleConfigsFromFile(t *testing.T) {
	files, err := ioutil.ReadDir("../../examplePipelines")
	if err != nil {
		t.Fatal(err)
	}
	for _, f := range files {
		t.Run(f.Name(), func(t *testing.T) {
			file, err := os.ReadFile("../../examplePipelines/" + f.Name())
			if err != nil {
				t.Fatal(err)
			}
			hclconf := HCLPipelineConfig{}
			err = hclconf.FromBytes(file, f.Name())
			if err != nil {
				t.Fatal(err)
			}
		})
	}
}

func TestCLIExampleConfigFile(t *testing.T) {
	file, err := os.ReadFile("../cli/config/examplePipeline.hcl")
	if err != nil {
		t.Fatal(err)
	}
	hclconf := HCLPipelineConfig{}
	err = hclconf.FromBytes(file, "examplePipeline.hcl")
	if err != nil {
		t.Fatal(err)
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
