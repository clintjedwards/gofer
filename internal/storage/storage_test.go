package storage

import (
	"errors"
	"fmt"
	"os"
	"testing"

	"github.com/jmoiron/sqlx"
)

func tempFile() string {
	f, err := os.CreateTemp("", "gofer-test-")
	if err != nil {
		panic(err)
	}
	if err := f.Close(); err != nil {
		panic(err)
	}
	if err := os.Remove(f.Name()); err != nil {
		panic(err)
	}
	return f.Name()
}

func TestTransactionSuccess(t *testing.T) {
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
		Created:     0,
		Modified:    0,
	}

	newName := "Changed Namespace"

	err = InsideTx(db.DB, func(tx *sqlx.Tx) error {
		err = db.InsertNamespace(tx, &namespace)
		if err != nil {
			return err
		}

		err = db.UpdateNamespace(tx, namespace.ID, UpdatableNamespaceFields{Name: &newName})
		if err != nil {
			return err
		}

		return nil
	})
	if err != nil {
		t.Fatal(err)
	}

	retrievedNamespace, err := db.GetNamespace(db.DB, namespace.ID)
	if err != nil {
		t.Fatal(err)
	}

	if retrievedNamespace.Name != newName {
		t.Fatalf("transaction did not apply successfully; expected updated name %q; got %q", newName, retrievedNamespace.Name)
	}
}

func TestTransactionFailure(t *testing.T) {
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
		Created:     0,
		Modified:    0,
	}

	newName := "Changed Namespace"

	_ = InsideTx(db.DB, func(tx *sqlx.Tx) error {
		err = db.InsertNamespace(tx, &namespace)
		if err != nil {
			return err
		}

		err = db.UpdateNamespace(tx, namespace.ID, UpdatableNamespaceFields{Name: &newName})
		if err != nil {
			return err
		}

		return fmt.Errorf("this is a simulated error that happens inside the transaction")
	})
	// Simulate the user continuing here instead of checking the error
	// so we can check the db state.

	_, err = db.GetNamespace(db.DB, namespace.ID)
	if err != nil {
		if errors.Is(err, ErrEntityNotFound) {
			return
		}
	}

	t.Fatalf("transaction did not rollback successfully")
}
