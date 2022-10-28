use std::{cell::RefCell, collections::HashMap, fmt::Display, rc::Rc};

#[derive(thiserror::Error, Debug, PartialEq, Eq)]
pub enum DAGError {
    #[error("entity not found")]
    EntityNotFound,

    #[error("entity already exists")]
    EntityExists,

    #[error("edges {0} and {1} would create a cycle")]
    EdgeCreatesCycle(String, String),
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Node {
    pub id: String,
    pub edges: Vec<Rc<RefCell<Node>>>,
}

impl Display for Node {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {:?}", self.id, self.edges)
    }
}

pub struct Dag(HashMap<String, Rc<RefCell<Node>>>);

impl Dag {
    pub fn new() -> Self {
        Dag(HashMap::<String, Rc<RefCell<Node>>>::new())
    }

    pub fn add_node(&mut self, id: &str) -> Result<(), DAGError> {
        if self.0.contains_key(id) {
            return Err(DAGError::EntityExists);
        }

        self.0.insert(
            id.to_string(),
            Rc::new(RefCell::new(Node {
                id: id.to_string(),
                edges: vec![],
            })),
        );

        Ok(())
    }

    pub fn add_edge(&mut self, from: &str, to: &str) -> Result<(), DAGError> {
        if !self.0.contains_key(from) {
            return Err(DAGError::EntityNotFound);
        }

        if !self.0.contains_key(to) {
            return Err(DAGError::EntityNotFound);
        }

        if self.is_cyclic(from, to) {
            return Err(DAGError::EdgeCreatesCycle(from.to_string(), to.to_string()));
        }

        let node1 = self.0.get(from).unwrap();
        let node2 = self.0.get(to).unwrap();

        node1.borrow_mut().edges.push(node2.clone());

        Ok(())
    }

    pub fn exists(&self, id: &str) -> bool {
        self.0.contains_key(id)
    }

    pub fn edges(&self, id: &str) -> Result<Vec<Rc<RefCell<Node>>>, DAGError> {
        if !self.0.contains_key(id) {
            return Err(DAGError::EntityNotFound);
        }

        Ok(self.0.get(id).unwrap().borrow().edges.clone())
    }

    /// Recursively look to see if a potential connection from node1 -> node2 will become cyclical.
    /// Iterates through node2 to make sure that it and none of it's edges are the same as node1.
    /// Once an edge is checked for sameness we recursively check all of its edges.
    /// This allows us to check all potential paths for the starting node.
    fn is_cyclic(&self, node1: &str, node2: &str) -> bool {
        if !self.0.contains_key(node1) {
            return false;
        }

        if !self.0.contains_key(node2) {
            return false;
        }

        if node1 == node2 {
            return true;
        }

        let node2_edges = &self.0.get(node2).unwrap().borrow().edges;

        for edge in node2_edges {
            let edge = edge.borrow();
            if node1 == edge.id {
                return true;
            }

            if self.is_cyclic(node1, &edge.id) {
                return true;
            }
        }

        false
    }
}

impl Display for Dag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (key, value) in &self.0 {
            write!(f, "{}: {}", key, value.borrow_mut())?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_dag() {
        let mut dag = Dag::new();

        dag.add_node("1").unwrap();
        dag.add_node("2").unwrap();

        dag.add_edge("1", "2").unwrap();

        assert!(dag.exists("1"))
    }

    #[test]
    fn test_dag_is_cylic() {
        let mut dag = Dag::new();

        dag.add_node("1").unwrap();
        dag.add_node("2").unwrap();

        dag.add_edge("1", "2").unwrap();

        dag.add_node("3").unwrap();
        dag.add_node("4").unwrap();

        dag.add_edge("2", "3").unwrap();
        dag.add_edge("2", "4").unwrap();

        dag.add_node("5").unwrap();
        dag.add_edge("4", "5").unwrap();

        dag.add_node("6").unwrap();
        dag.add_edge("5", "6").unwrap();
        let err = dag.add_edge("6", "4").unwrap_err();

        assert_eq!(
            std::mem::discriminant(&err),
            std::mem::discriminant(&DAGError::EdgeCreatesCycle("".to_string(), "".to_string()))
        );

        dag.add_edge("6", "3").unwrap();
    }

    #[test]
    fn test_dag_is_acylic() {
        let mut dag = Dag::new();

        dag.add_node("1").unwrap();
        dag.add_node("2").unwrap();

        dag.add_edge("1", "2").unwrap();

        dag.add_node("3").unwrap();
        dag.add_node("4").unwrap();
        dag.add_edge("2", "3").unwrap();
        dag.add_edge("2", "4").unwrap();

        dag.add_node("5").unwrap();
        dag.add_edge("4", "5").unwrap();

        dag.add_node("6").unwrap();
        dag.add_edge("4", "6").unwrap();
        dag.add_edge("5", "6").unwrap();
        dag.add_edge("6", "3").unwrap();
    }
}
