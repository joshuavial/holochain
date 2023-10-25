//! Functions for constructing causal graphs

use std::collections::{HashMap, HashSet};
use std::fmt::Display;
use std::hash::Hash;
use std::io::{Read, Write};

use crate::cause::*;

/// A DAG of Facts linked by their causal relationships
#[derive(Debug, derive_more::From, derive_more::Deref, derive_more::DerefMut)]
pub struct CausalGraph<T: Display>(petgraph::graph::DiGraph<Cause<T>, ()>);

impl<T: Display + Clone + Eq + Hash> CausalGraph<T> {
    /// Just return the nodes.
    pub fn nodes(&self) -> HashSet<Cause<T>> {
        self.node_weights().cloned().collect::<HashSet<_>>()
    }
}

impl<T: Display> Default for CausalGraph<T> {
    fn default() -> Self {
        Self(Default::default())
    }
}

/// A traversal of the causal graph, potentially resulting in a DAG of failing facts.
#[derive(Debug, derive_more::From)]
pub enum Traversal<T: Fact> {
    /// The target fact is true; nothing more needs to be said
    Pass,
    /// The target is false and there is no path to any true fact
    Groundless,
    /// The target fact is false, and all paths which lead to true facts
    /// are present in this graph
    Fail {
        /// The DAG of failing facts
        graph: CausalGraph<T>,
        /// The facts which did pass (the leaves of the DAG point to these)
        passes: Vec<Cause<T>>,
    },
}

impl<T: Fact> Traversal<T> {
    /// If failure, just return the contents
    pub fn fail(self) -> Option<(CausalGraph<T>, Vec<Cause<T>>)> {
        match self {
            Traversal::Fail {
                graph: tree,
                passes,
            } => Some((tree, passes)),
            _ => None,
        }
    }
}

type TraversalMap<T> = HashMap<Cause<T>, Option<Check<T>>>;

/// Traverse the causal graph implied by the specified Cause.
///
/// The Traversal is recorded as a sparse adjacency matrix.
/// Each cause which is visited in the traversal gets added as a node in the graph,
/// initially with no edges.
/// For each cause with a failing "check", we recursively visit its cause(s).
/// Any time we encounter a cause with a passing "check", we backtrack and add edges
/// to add this path to the graph.
/// If a path ends in a failing check, or if it forms a loop without encountering
/// a passing check, we don't add that path to the graph.
#[tracing::instrument(skip(ctx))]
pub fn traverse<F: Fact>(cause: &Cause<F>, ctx: &F::Context) -> Traversal<F> {
    let mut table = TraversalMap::default();
    match traverse_inner(cause, ctx, &mut table) {
        Some(check) => {
            if check.is_pass() {
                Traversal::Pass
            } else {
                let (graph, passes) = produce_graph(&table, cause);
                Traversal::Fail { graph, passes }
            }
        }
        None => Traversal::Groundless,
    }
}

fn traverse_inner<F: Fact>(
    cause: &Cause<F>,
    ctx: &F::Context,
    table: &mut TraversalMap<F>,
) -> Option<Check<F>> {
    tracing::trace!("enter {:?}", cause);
    match table.get(cause) {
        None => {
            tracing::trace!("marked visited");
            // Mark this node as visited but undetermined in case the traversal leads to a loop
            table.insert(cause.clone(), None);
        }
        Some(None) => {
            tracing::trace!("loop encountered");
            // We're currently processing a traversal that started from this cause.
            // Not even sure if this is even valid, but in any case
            // we certainly can't say anything about this traversal.
            return None;
        }
        Some(Some(check)) => {
            tracing::trace!("return cached: {:?}", check);
            return Some(check.clone());
        }
    }

    let check = match cause {
        Cause::Fact(f) => {
            if f.check(ctx) {
                tracing::trace!("fact pass");
                Check::Pass
            } else {
                if let Some(cause) = f.cause(ctx) {
                    tracing::trace!("fact fail with cause, traversing");
                    let check = traverse_inner(&cause, ctx, table)?;
                    tracing::trace!("traversal done, check: {:?}", check);
                    Check::Fail(vec![cause])
                } else {
                    tracing::trace!("fact fail with no cause, terminating");
                    Check::Fail(vec![])
                }
            }
        }
        Cause::Any(cs) => {
            let checks: Vec<_> = cs
                .iter()
                .filter_map(|c| Some((c.clone(), traverse_inner(c, ctx, table)?)))
                .collect();
            tracing::trace!("Any. checks: {:?}", checks);
            if checks.is_empty() {
                // All loops
                tracing::debug!("All loops");
                return None;
            }
            let num_checks = checks.len();
            let fails: Vec<_> = checks
                .into_iter()
                .filter_map(|(cause, check)| (!check.is_pass()).then_some(cause))
                .collect();
            tracing::trace!("Any. fails: {:?}", fails);
            if fails.len() < num_checks {
                Check::Pass
            } else {
                Check::Fail(fails)
            }
        }
        Cause::Every(cs) => {
            let checks: Vec<_> = cs
                .iter()
                .filter_map(|c| Some((c.clone(), traverse_inner(c, ctx, table)?)))
                .collect();
            tracing::trace!("Every. checks: {:?}", checks);
            if checks.is_empty() {
                // All loops
                tracing::debug!("All loops");
                return None;
            }
            let fails = checks.iter().filter(|(_, check)| !check.is_pass()).count();
            let causes: Vec<_> = checks.into_iter().map(|(cause, _)| cause).collect();
            tracing::trace!("Every. num fails: {}", fails);
            if fails == 0 {
                Check::Pass
            } else {
                Check::Fail(causes)
            }
        }
    };
    table.insert(cause.clone(), Some(check.clone()));
    tracing::trace!("exit. check: {:?}", check);
    Some(check)
}

/// Prune away any extraneous nodes or edges from a Traversal.
/// After pruning, the graph contains all edges starting with the specified cause
/// and ending with a true cause.
/// Passing facts are returned separately.
fn prune_traversal<'a, 'b: 'a, T: Fact + Eq + Hash>(
    table: &'a TraversalMap<T>,
    start: &'b Cause<T>,
) -> (HashMap<&'a Cause<T>, &'a [Cause<T>]>, Vec<&'a Cause<T>>) {
    let mut sub = HashMap::<&Cause<T>, &[Cause<T>]>::new();
    let mut passes = vec![];
    let mut to_add = vec![start];

    while let Some(next) = to_add.pop() {
        match table[&next].as_ref() {
            Some(Check::Fail(causes)) => {
                to_add.extend(causes.iter());
                sub.insert(next, causes.as_slice());
            }
            Some(Check::Pass) => {
                passes.push(next);
            }
            None => {}
        }
    }
    (sub, passes)
}

fn produce_graph<'a, 'b: 'a, T: Fact + Eq + Hash>(
    table: &'a TraversalMap<T>,
    start: &'b Cause<T>,
) -> (CausalGraph<T>, Vec<Cause<T>>) {
    let mut g = CausalGraph::default();

    let (sub, passes) = prune_traversal(table, start);

    let rows: Vec<_> = sub.into_iter().collect();
    let mut nodemap = HashMap::new();
    for (i, (k, _)) in rows.iter().enumerate() {
        let id = g.add_node((*k).to_owned());
        nodemap.insert(k, id);
        assert_eq!(id.index(), i);
    }

    for (k, v) in rows.iter() {
        for c in v.iter() {
            if let (Some(k), Some(c)) = (nodemap.get(k), nodemap.get(&c)) {
                g.add_edge(*k, *c, ());
            }
        }
    }

    (g, passes.into_iter().cloned().collect())
}

/// If a `graph-easy` binary is installed, render an ASCII graph from the
/// provided dot syntax.
pub fn graph_easy(dot: &str) -> anyhow::Result<String> {
    let process = std::process::Command::new("graph-easy")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()?;

    process.stdin.unwrap().write_all(dot.as_bytes()).unwrap();
    let mut s = String::new();
    process.stdout.unwrap().read_to_string(&mut s).unwrap();

    Ok(s)
}
