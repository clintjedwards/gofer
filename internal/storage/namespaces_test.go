package storage

import (
	"errors"
	"os"
	"testing"

	"github.com/google/go-cmp/cmp"
)

func TestCRUDNamespaces(t *testing.T) {
	path := tempFile()
	db, err := New(path, 200)
	if err != nil {
		t.Fatal(err)
	}
	defer os.Remove(path)

	namespace := Namespace{
		ID:          "test_namespace",
		Name:        "Test Namespace",
		Description: "This is a test namespace",
		Created:     "0",
		Modified:    "0",
	}

	err = db.InsertNamespace(db, &namespace)
	if err != nil {
		t.Fatal(err)
	}

	namespaces, err := db.ListNamespaces(db, 0, 0)
	if err != nil {
		t.Fatal(err)
	}

	if len(namespaces) != 1 {
		t.Errorf("expected 1 element in list found %d", len(namespaces))
	}

	if diff := cmp.Diff(namespace, namespaces[0]); diff != "" {
		t.Errorf("unexpected map values (-want +got):\n%s", diff)
	}

	fetchedNamespace, err := db.GetNamespace(db, namespace.ID)
	if err != nil {
		t.Fatal(err)
	}

	if diff := cmp.Diff(namespace, fetchedNamespace); diff != "" {
		t.Errorf("unexpected map values (-want +got):\n%s", diff)
	}

	namespace.Name = "Updated Namespace"
	namespace.Description = "updated namespace"
	namespace.Modified = "1"

	err = db.UpdateNamespace(db, namespace.ID, UpdatableNamespaceFields{
		Name:        &namespace.Name,
		Description: &namespace.Description,
		Modified:    &namespace.Modified,
	})
	if err != nil {
		t.Fatal(err)
	}

	fetchedNamespace, err = db.GetNamespace(db, namespace.ID)
	if err != nil {
		t.Fatal(err)
	}

	if diff := cmp.Diff(namespace, fetchedNamespace); diff != "" {
		t.Errorf("unexpected map values (-want +got):\n%s", diff)
	}

	err = db.DeleteNamespace(db, namespace.ID)
	if err != nil {
		t.Fatal(err)
	}

	_, err = db.GetNamespace(db, namespace.ID)
	if !errors.Is(err, ErrEntityNotFound) {
		t.Fatal("expected error Not Found; found alternate error")
	}
}
