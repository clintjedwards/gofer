package sdk

import (
	"testing"

	"github.com/google/go-cmp/cmp"
)

// IMPORTANT: Changing the `expected` string in this function requires you to change the rust sister function in
// the rust sdk.
func TestInstallInstructions(t *testing.T) {
	instructions, err := NewInstructionsBuilder().AddMessage("test").AddQuery("test", "config").JSON()
	if err != nil {
		t.Fatal(err)
	}

	expected := `{"instructions":[{"message":{"text":"test"}},{"query":{"text":"test","config_key":"config"}}]}`

	if diff := cmp.Diff(expected, instructions); diff != "" {
		t.Errorf("unexpected json output (-want +got):\n%s", diff)
	}
}
