use std::collections::{BTreeMap, HashMap};

#[derive(thiserror::Error, Debug, Clone, PartialEq, Eq)]
pub enum DagError {
    #[error("task '{0}' is defined more than once")]
    DuplicateNode(String),

    #[error("task '{node}' depends on '{dependency}', which doesn't exist")]
    DependencyNotFound { node: String, dependency: String },

    /// The tasks in the cycle in order, each depending on the next, ending where it started.
    #[error("tasks depend on each other in a cycle: {}", .0.join(" -> "))]
    Cycle(Vec<String>),
}

/// Checks that tasks and their dependencies form a DAG. Takes each task's id along with the ids it depends on and
/// returns every problem found rather than stopping at the first, so they can all be fixed in one go.
///
/// Each cycle is reported once with the path around it, so the user can see which dependency to remove. The output
/// is sorted by task id so the same config always gives the same errors.
pub fn validate<'a, D>(nodes: impl IntoIterator<Item = (&'a str, D)>) -> Vec<DagError>
where
    D: IntoIterator<Item = &'a str>,
{
    let mut errors = vec![];
    let mut graph: BTreeMap<&str, Vec<&str>> = BTreeMap::new();

    for (id, dependencies) in nodes {
        if graph.contains_key(id) {
            errors.push(DagError::DuplicateNode(id.to_string()));
            continue;
        }

        let mut dependencies: Vec<&str> = dependencies.into_iter().collect();
        dependencies.sort();
        graph.insert(id, dependencies);
    }

    for (id, dependencies) in &graph {
        for dependency in dependencies {
            if !graph.contains_key(dependency) {
                errors.push(DagError::DependencyNotFound {
                    node: id.to_string(),
                    dependency: dependency.to_string(),
                });
            }
        }
    }

    let mut state = HashMap::new();
    let mut path = vec![];
    for id in graph.keys() {
        find_cycles(&graph, id, &mut state, &mut path, &mut errors);
    }

    errors
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Visit {
    InProgress,
    Done,
}

/// Depth first search that records a cycle whenever it reaches a node that's still on the current path.
fn find_cycles<'a>(
    graph: &BTreeMap<&'a str, Vec<&'a str>>,
    node: &'a str,
    state: &mut HashMap<&'a str, Visit>,
    path: &mut Vec<&'a str>,
    errors: &mut Vec<DagError>,
) {
    match state.get(node) {
        Some(Visit::Done) => return,
        Some(Visit::InProgress) => {
            let start = path.iter().position(|id| *id == node).unwrap_or_default();
            let mut cycle: Vec<String> = path[start..].iter().map(|id| id.to_string()).collect();
            cycle.push(node.to_string());
            errors.push(DagError::Cycle(cycle));
            return;
        }
        None => {}
    }

    state.insert(node, Visit::InProgress);
    path.push(node);

    // Missing dependencies are reported separately, so they're skipped here.
    for dependency in graph.get(node).into_iter().flatten() {
        if graph.contains_key(dependency) {
            find_cycles(graph, dependency, state, path, errors);
        }
    }

    path.pop();
    state.insert(node, Visit::Done);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check(nodes: &[(&'static str, Vec<&'static str>)]) -> Vec<DagError> {
        validate(nodes.iter().map(|(id, deps)| (*id, deps.iter().copied())))
    }

    #[test]
    fn test_valid_dag() {
        // A diamond with a tail, which shares paths but has no cycle.
        let errors = check(&[
            ("a", vec![]),
            ("b", vec!["a"]),
            ("c", vec!["a"]),
            ("d", vec!["b", "c"]),
            ("e", vec!["d", "a"]),
        ]);
        assert_eq!(errors, vec![]);
    }

    #[test]
    fn test_duplicate_node() {
        let errors = check(&[("a", vec![]), ("a", vec![])]);
        assert_eq!(errors, vec![DagError::DuplicateNode("a".into())]);
    }

    #[test]
    fn test_dependency_not_found() {
        let errors = check(&[("a", vec!["missing"])]);
        assert_eq!(
            errors,
            vec![DagError::DependencyNotFound {
                node: "a".into(),
                dependency: "missing".into()
            }]
        );
    }

    #[test]
    fn test_self_dependency_is_a_cycle() {
        let errors = check(&[("a", vec!["a"])]);
        assert_eq!(errors, vec![DagError::Cycle(vec!["a".into(), "a".into()])]);
    }

    #[test]
    fn test_cycle_reports_path() {
        let errors = check(&[
            ("a", vec!["c"]),
            ("b", vec!["a"]),
            ("c", vec!["b"]),
            ("d", vec!["a"]),
        ]);
        assert_eq!(
            errors,
            vec![DagError::Cycle(vec![
                "a".into(),
                "c".into(),
                "b".into(),
                "a".into()
            ])]
        );
        assert_eq!(
            errors[0].to_string(),
            "tasks depend on each other in a cycle: a -> c -> b -> a"
        );
    }

    #[test]
    fn test_reports_every_problem() {
        let errors = check(&[
            ("a", vec!["b"]),
            ("b", vec!["a"]),
            ("c", vec!["missing"]),
            ("c", vec![]),
            ("d", vec!["d"]),
        ]);
        assert_eq!(
            errors,
            vec![
                DagError::DuplicateNode("c".into()),
                DagError::DependencyNotFound {
                    node: "c".into(),
                    dependency: "missing".into()
                },
                DagError::Cycle(vec!["a".into(), "b".into(), "a".into()]),
                DagError::Cycle(vec!["d".into(), "d".into()]),
            ]
        );
    }
}
