package storage

import (
	"os"
	"testing"

	"github.com/google/go-cmp/cmp"
)

func TestCRUDGlobalExtensionRegistrations(t *testing.T) {
	path := tempFile()
	db, err := New(path, 200)
	if err != nil {
		t.Fatal(err)
	}
	defer os.Remove(path)

	reg := GlobalExtensionRegistration{
		ID:           "test_registration",
		Image:        "test_image",
		RegistryAuth: "test_reg",
		Variables:    "test_vars",
		Created:      "0",
		Status:       "ACTIVE",
		KeyID:        "1",
	}

	err = db.InsertGlobalExtensionRegistration(db, &reg)
	if err != nil {
		t.Fatal(err)
	}

	regs, err := db.ListGlobalExtensionRegistrations(db, 0, 0)
	if err != nil {
		t.Fatal(err)
	}

	if len(regs) != 1 {
		t.Errorf("expected 1 element in list found %d", len(regs))
	}

	if diff := cmp.Diff(reg, regs[0]); diff != "" {
		t.Errorf("unexpected map values (-want +got):\n%s", diff)
	}

	fetchedTask, err := db.GetGlobalExtensionRegistration(db, "test_registration")
	if err != nil {
		t.Fatal(err)
	}

	if diff := cmp.Diff(reg, fetchedTask); diff != "" {
		t.Errorf("unexpected map values (-want +got):\n%s", diff)
	}
}
