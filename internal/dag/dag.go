// Package dag is used to verify and map out directed acyclic graph implementations. This helps us verify that user's
// task dependencies actually work as a DAG and avoid entering in any cycles.
package dag

import (
	"errors"
)

type DAG map[string]Node

type Node struct {
	ID    string
	Edges []Node
}

var (
	// ErrEntityNotFound is returned when a certain entity could not be located.
	ErrEntityNotFound = errors.New("dag: entity not found")

	// ErrEntityExists is returned when a certain entity was located but not meant to be.
	ErrEntityExists = errors.New("dag: entity already exists")

	// ErrPreconditionFailure is returned when there was a validation error with the parameters passed.
	ErrPreconditionFailure = errors.New("dag: parameters did not pass validation")

	// ErrEdgeCreatesCycle is returned when the introduction of an edge would create a cycle.
	ErrEdgeCreatesCycle = errors.New("dag: edge would create a cycle")
)

func New() DAG {
	return map[string]Node{}
}

func (dag DAG) AddNode(id string) error {
	_, exists := dag[id]
	if exists {
		return ErrEntityExists
	}

	dag[id] = Node{ID: id}
	return nil
}

func (dag DAG) AddEdge(from, to string) error {
	if _, exists := dag[from]; !exists {
		return ErrEntityNotFound
	}

	if _, exists := dag[to]; !exists {
		return ErrEntityNotFound
	}

	if dag.isCyclic(from, to) {
		return ErrEdgeCreatesCycle
	}

	node1 := dag[from]
	node1.Edges = append(node1.Edges, dag[to])
	dag[from] = node1
	return nil
}

func (dag DAG) Exists(id string) bool {
	_, exists := dag[id]
	return exists
}

func (dag DAG) Edges(id string) ([]Node, error) {
	if _, exists := dag[id]; !exists {
		return nil, ErrEntityNotFound
	}
	return dag[id].Edges, nil
}

func (dag DAG) String() string {
	return ""
}

func (dag DAG) isCyclic(node1 string, node2 string) bool {
	if _, exists := dag[node1]; !exists {
		return false
	}

	if _, exists := dag[node2]; !exists {
		return false
	}

	if node1 == node2 {
		return true
	}

	for _, node := range dag[node2].Edges {
		if node.ID == dag[node1].ID {
			return true
		}

		node2 = node.ID
		if dag.isCyclic(node1, node2) {
			return true
		}
	}

	return false
}
