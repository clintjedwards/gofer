package dag

import (
	"testing"
)

func TestCreateDAG(t *testing.T) {
	dag := New()
	err := dag.AddNode("1")
	if err != nil {
		t.Fatal(err)
	}
	err = dag.AddNode("2")
	if err != nil {
		t.Fatal(err)
	}
	err = dag.AddEdge("1", "2")
	if err != nil {
		t.Fatal(err)
	}
	if !dag.Exists("1") {
		t.Fatal("node 1 should exist and does not")
	}
}

func TestDAGIsCylic(t *testing.T) {
	dag := New()

	_ = dag.AddNode("1")
	_ = dag.AddNode("2")
	err := dag.AddEdge("1", "2")
	if err != nil {
		t.Fatal(err)
	}
	_ = dag.AddNode("3")
	_ = dag.AddNode("4")
	err = dag.AddEdge("2", "3")
	if err != nil {
		t.Fatal(err)
	}
	err = dag.AddEdge("2", "4")
	if err != nil {
		t.Fatal(err)
	}
	_ = dag.AddNode("5")
	err = dag.AddEdge("4", "5")
	if err != nil {
		t.Fatal(err)
	}

	_ = dag.AddNode("6")
	err = dag.AddEdge("5", "6")
	if err != nil {
		t.Fatal(err)
	}

	err = dag.AddEdge("6", "4")
	if err == nil {
		t.Fatal("should be a cycle here and was not one")
	}
	err = dag.AddEdge("6", "3")
	if err != nil {
		t.Fatal(err)
	}
}
func TestDAGIsAcylic(t *testing.T) {
	dag := New()

	_ = dag.AddNode("1")
	_ = dag.AddNode("2")
	err := dag.AddEdge("1", "2")
	if err != nil {
		t.Fatal(err)
	}
	_ = dag.AddNode("3")
	_ = dag.AddNode("4")
	err = dag.AddEdge("2", "3")
	if err != nil {
		t.Fatal(err)
	}
	err = dag.AddEdge("2", "4")
	if err != nil {
		t.Fatal(err)
	}
	_ = dag.AddNode("5")
	err = dag.AddEdge("4", "5")
	if err != nil {
		t.Fatal(err)
	}

	_ = dag.AddNode("6")
	err = dag.AddEdge("4", "6")
	if err != nil {
		t.Fatal(err)
	}
	err = dag.AddEdge("5", "6")
	if err != nil {
		t.Fatal(err)
	}
	err = dag.AddEdge("6", "3")
	if err != nil {
		t.Fatal(err)
	}
}
