use std::{cell::{Cell, RefCell}, collections::{HashMap, HashSet}, fmt::Debug};
use crate::{ debug::*, collections::{Get, GetMut, SparseSet} };

#[derive(Debug)]
pub enum ConflictGraphError<N> where N: ConflictGraphNode {
    PushFailed(N),
    NodeDoesntExist,
}

pub trait ConflictGraphNode {
    type Dependency: PartialEq + Debug;
    fn dependencies(&self) -> Vec<Self::Dependency>;
    fn mutable_dependencies(&self) -> Vec<Self::Dependency>;
}

#[derive(Debug)]
struct InnerNode<N: ConflictGraphNode> {
    outer: N,
    edges: RefCell<HashSet<usize>>,
    color: Cell<usize>,
}

impl<N: ConflictGraphNode> InnerNode<N> {
    fn new(outer: N) -> Self {
        InnerNode {
            outer: outer,
            edges: RefCell::new(HashSet::new()),
            color: Cell::new(0usize),
        }
    }
}

#[derive(Debug)]
pub struct ConflictGraph<N> where N: ConflictGraphNode {
    nodes: SparseSet<InnerNode<N>>,
    ready: bool,
}

impl<N: ConflictGraphNode + Debug> ConflictGraph<N> {
    pub fn new() -> Self {
        Self {
            nodes: SparseSet::new(),
            ready: false,
        }
    }

    pub fn insert(&mut self, node: N) -> Result<(), ConflictGraphError<N>> {
        self.nodes.insert(InnerNode::new(node)).map_err(|result| ConflictGraphError::PushFailed(result.outer))?;
        self.ready = false;

        // a conflict can only happen when a write occurs
        // a conflict exists only if two nodes mutate the same dependency, or one node mutates, and one does not
        // create an edge between every node where a conflict exists

        self.rebuild()?;
        
        //self.color()?;
        //self.ready = true;

        Ok(())
    }

    /// Consumes the conflict graph and returns a set of conflict-free cliques as a Vec<Vec<N>>
    pub fn cliques(self) -> Result<Vec<Vec<N>>, ConflictGraphError<N>> {
        self.into()
    }

    fn rebuild(&mut self) -> Result<(), ConflictGraphError<N>> {
        for (_first_key, first) in self.nodes.kv_pairs() {
            for (second_key, second) in self.nodes.kv_pairs() {
                let mut conflict = false;
                for dep in first.outer.dependencies() {
                    for mut_dep in second.outer.mutable_dependencies() {
                        if dep == mut_dep {
                            conflict = true;
                        }
                    }
                }

                for mut_dep in first.outer.mutable_dependencies() {
                    for dep in second.outer.dependencies() {
                        if mut_dep == dep {
                            conflict = true;
                        }
                    }

                    for other_mut_dep in second.outer.dependencies() {
                        if mut_dep == other_mut_dep {
                            conflict = true;
                        }
                    }
                }

                if conflict {
                    first.edges.borrow_mut().insert(second_key);
                }
            }
        }

        Ok(())
    }

    fn color(&mut self) -> Result<(), ConflictGraphError<N>> {
        const UNCOLORED: usize = std::usize::MAX;
        let mut used_colors: Vec<usize> = vec![0usize];
        let mut uncolored_count = self.nodes.len();

        // helper struct, only used in this function
        struct Candidate {
            set_key: usize,
            c_adjacent: usize,
            u_adjacent: usize,
            forbidden_colors: Vec<usize>,
        }

        impl Candidate {
            fn new() -> Self {
                Self {
                    set_key: 0usize,
                    c_adjacent: 0usize,
                    u_adjacent: 0usize,
                    forbidden_colors: Vec::new(),
                }
            }
        }

        for node in self.nodes.iter() {
            node.color.set(UNCOLORED);
        }

        let mut passes = 0usize;
        while uncolored_count > 0 {
            passes += 1;

            let mut candidate = Candidate::new();

            for (_i, (key, node)) in self.nodes.kv_pairs().enumerate() {    
                // skip already colored nodes
                if node.color.get() == UNCOLORED {
                    // which has the most colored neighbors
                    let mut adjacent_uncolored_count = 0usize;
                    let mut adjacent_colors = Vec::new();
                    for edge in node.edges.borrow().iter() {
                        let adjacent_color = self.nodes.get(*edge).unwrap().color.get();
                        if adjacent_color != UNCOLORED {
                            adjacent_colors.push(adjacent_color);
                        } else {
                            adjacent_uncolored_count += 1;
                        }
                    }

                    // test if this candidate is better than our existing candidate, break a tie if there is one
                    if adjacent_colors.len() > candidate.c_adjacent {
                        // new candidate has more colored neighbors than existing candidate, no tie
                        candidate = Candidate {
                            set_key: key,
                            c_adjacent: adjacent_colors.len(),
                            u_adjacent: adjacent_uncolored_count,
                            forbidden_colors: adjacent_colors.clone(),
                        };
                    
                    } else if adjacent_colors.len() == candidate.c_adjacent {
                        // new candidate is tied with existing candidate
                        if adjacent_uncolored_count > candidate.u_adjacent {
                            // new candidate has more colored neighbors and more uncolored neighbors, select it
                            candidate = Candidate {
                                set_key: key,
                                c_adjacent: adjacent_colors.len(),
                                u_adjacent: adjacent_uncolored_count,
                                forbidden_colors: adjacent_colors.clone(),
                            };
                        }
                    }
                }
            }

            //println!("{:?} nodes, {:?} uncolored", self.nodes.len(), uncolored_count);
            //for (i, node) in self.nodes.iter().enumerate() {
            //    println!("node {:?} color {:?}", i, node.color.get())
            //}

            // choose the "smallest" color for the candidate, excluding its neighbors colors
            'outer: loop {
                for color in used_colors.iter() {
                    if !candidate.forbidden_colors.contains(&color) {
                        match self.nodes.get_mut(candidate.set_key) {
                            Some(node) => {
                                node.color.set(*color);
                                uncolored_count -= 1;

                                break 'outer;
                            },
                            None => {
                                return Err(ConflictGraphError::NodeDoesntExist);
                            }
                        }
                    }
                }
                used_colors.push(used_colors.len());
            }
        }

        //log!("conflict graph colored in {:?} passes, used {:?} colors", passes, used_colors.len());

        Ok(())
    }
}

impl<N> From<ConflictGraph<N>> for Result<Vec<Vec<N>>, ConflictGraphError<N>> where N: ConflictGraphNode + Debug {

    /// Converts the ConflictGraph into an unordered conflict free set of "cliques"
    ///
    /// Each inner Vec represents a set of nodes which are mutually conflict free
    /// while nodes in separate Vec's likely (but aren't guaranteed to) conflict with each other
    fn from(mut graph: ConflictGraph<N>) -> Self {
        if !graph.ready {
            graph.rebuild()?;
            graph.color()?;
        }
        
        let mut result: HashMap<usize, Vec<N>> = HashMap::new();

        for (i, node) in graph.nodes.into_iter().enumerate() {
            result.entry(node.color.get()).or_insert_with(|| Vec::default()).push(node.outer);
        }

        Ok(result.into_iter().map(|item| item.1).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insertion() {
        #[derive(Debug, Clone)]
        struct TestNode {
            w: Vec<i32>,
            r: Vec<i32>,
        }

        impl ConflictGraphNode for TestNode {
            type Dependency = i32;

            fn dependencies(&self) -> Vec<Self::Dependency> {
                self.r.clone()
            }

            fn mutable_dependencies(&self) -> Vec<Self::Dependency> {
                self.w.clone()
            }
        }

        let nodes = [
            TestNode { w: vec![1], r: vec![1, 2] },
            TestNode { w: vec![], r: vec![1, 2] },
            TestNode { w: vec![], r: vec![2] },
            TestNode { w: vec![], r: vec![1, 2] },
            TestNode { w: vec![2], r: vec![] },
            TestNode { w: vec![3], r: vec![3] },
            TestNode { w: vec![], r: vec![3] },
            TestNode { w: vec![], r: vec![1, 2, 3] },
            TestNode { w: vec![1], r: vec![1, 2] },
            TestNode { w: vec![2, 3], r: vec![1] },
            TestNode { w: vec![4], r: vec![1, 2, 3] },
            TestNode { w: vec![5], r: vec![2, 1] },
            TestNode { w: vec![4], r: vec![3] },
            TestNode { w: vec![1, 3, 2], r: vec![] },
        ];

        let mut graph = ConflictGraph::new();

        for node in nodes.iter() {
            let _ = graph.insert((*node).clone());
        }
        
        let cliques = graph.cliques().unwrap();
        println!("\n{:?} cliques", cliques.len());
        for (i, clique) in cliques.iter().enumerate() {
            println!("\tclique {:?} size {:?}", i, clique.len());
        }
    }
}
