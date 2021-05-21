use std::{cell::{Cell, RefCell}, collections::{HashMap, HashSet}, error::Error, fmt::{Debug, Display}};
use crate::{ debug::*, collections::{Get, SparseSet} };

const UNCOLORED: usize = std::usize::MAX;

#[derive(Debug)]
pub enum ConflictGraphError {
    InsertFailed,
    NodeDoesntExist,
    UnresolvedConflict,
    UncoloredNode,
}

impl Display for ConflictGraphError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InsertFailed => { write!(f, "failed to insert into internal set") }
            Self::NodeDoesntExist => { write!(f, "node doesn't exist") }
            Self::UnresolvedConflict => { write!(f, "unresolved conflict") }
            Self::UncoloredNode => { write!(f, "uncolored node") }
        }
    }
}

impl Error for ConflictGraphError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}

pub trait ConflictCmp {
    fn conflict_cmp(&self, other: &Self) -> bool;
}

#[derive(Debug, Clone)]
struct InnerNode<N: ConflictCmp> {
    udata: N,

    // using cells here is for graph convenience
    edges: RefCell<HashSet<usize>>,
    color: Cell<usize>,
}

impl<N: ConflictCmp> InnerNode<N> {
    fn new(outer: N) -> Self {
        InnerNode {
            udata: outer,
            edges: RefCell::new(HashSet::new()),
            color: Cell::new(0usize),
        }
    }
}

/// Defer to the external types ConflictCmp impl
impl<N> ConflictCmp for InnerNode<N> where N: ConflictCmp {
    fn conflict_cmp(&self, other: &Self) -> bool {
        self.udata.conflict_cmp(&other.udata)
    }
}

#[derive(Debug, Clone)]
pub struct ConflictGraph<N> where N: ConflictCmp {
    nodes: SparseSet<InnerNode<N>>,
}

impl<N: ConflictCmp> ConflictGraph<N> {
    pub fn new() -> Self {
        Self {
            nodes: SparseSet::new(),
        }
    }
    
    /// Insert a new node
    /// ConflictGraph only supports insertion, it does not support removal or access to its contents, it is not a container
    pub fn insert(&mut self, node: N) -> Result<(), ConflictGraphError> {
        let _key = self.nodes.insert(InnerNode::new(node));

        Ok(())
    }
    
    /// Consumes the conflict graph and returns a set of conflict-free cliques as a Vec<Vec<N>>
    /// 
    /// Each inner Vec represents a group of items which do not conflict with one another based on their ConflictCmp implementation
    ///
    /// Not currently guaranteed to produce the smallest number of cliques possible, but makes a reasonable effort
    pub fn cliques(mut self) -> Result<Vec<Vec<N>>, ConflictGraphError> {
        self.rebuild()?;
        self.color()?;
        self.check()?;
        self.into()
    }

    fn rebuild(&mut self) -> Result<(), ConflictGraphError> {
        // clear existing edges and start from scratch
        for node in self.nodes.iter() {
            node.edges.borrow_mut().clear();
        }

        for (node_key, node) in self.nodes.kv_pairs() {
            for (other_key, other) in self.nodes.kv_pairs() {
                if node_key == other_key {
                    continue;
                }

                let conflict = node.conflict_cmp(other);

                if conflict {
                    node.edges.borrow_mut().insert(other_key);
                }
            }
        }

        Ok(())
    }
    
    fn color(&mut self) -> Result<(), ConflictGraphError> {
        // colors the graph using a pretty simple brute force approach. This allocates a lot, and isn't
        // particularly efficient. Suitable for small graphs that are created infrequently, but will break
        // down in performance quickly with large graphs. Maybe worth improving in the future if
        // it becomes a problem or finds use elsewhere

        let mut used_colors: Vec<usize> = Vec::with_capacity(8);

        // quick and dirty helper struct, only used in this function for clarities sake
        #[derive(Default, PartialEq, Eq)]
        struct Candidate {
            graph_key: usize,
            uncolored_adjacent: usize,
            forbidden_colors: Vec<usize>,
        }
        
        impl Candidate {
            fn new(k: usize, u: usize, f: Vec<usize>) -> Self {
                Self {
                    graph_key: k,
                    uncolored_adjacent: u,
                    forbidden_colors: f,
                }
            }
        }

        // clear the colors of all nodes incase some are already colored somehow
        for node in self.nodes.iter() {
            node.color.set(UNCOLORED);
        }

        let mut passes = 0usize;
        let mut uncolored_count = self.nodes.len();
        while uncolored_count > 0 {
            passes += 1;

            let mut candidate = Candidate::default();

            for (_i, (key, node)) in self.nodes.kv_pairs().enumerate() {    
                // skip already colored nodes
                if node.color.get() == UNCOLORED {
                    
                    let mut test_candidate = Candidate::new(key, 0, Vec::new());

                    // get every adjacent node and record its color, count how many are uncolored
                    for edge in node.edges.borrow().iter() {
                        let adjacent_color = self.nodes.get(*edge).unwrap().color.get();
                        if adjacent_color == UNCOLORED {
                            assert_ne!(uncolored_count, 1);
                            test_candidate.uncolored_adjacent += 1;
                        } else {
                            test_candidate.forbidden_colors.push(adjacent_color);
                        }
                    }

                    if uncolored_count == 1 {
                        candidate = test_candidate;
                        break;
                    }

                    if node.edges.borrow().is_empty() {
                        candidate = test_candidate;
                        break;
                    }

                    // test if this candidate is better than our existing candidate, break a tie if there is one
                    if test_candidate.forbidden_colors.len() > candidate.forbidden_colors.len() {
                        // new candidate has more colored neighbors than existing candidate, no tie
                        candidate = test_candidate;
                        break;
                    } else if test_candidate.forbidden_colors.len() == candidate.forbidden_colors.len() {
                        // new candidate is tied with existing candidate
                        if test_candidate.uncolored_adjacent > candidate.uncolored_adjacent {
                            // new candidate has more colored neighbors and more uncolored neighbors, select it
                            candidate = test_candidate;
                            break;
                        }
                    }
                }
            }
            
            // choose the "smallest" color for the candidate, excluding its neighbors colors
            'outer: loop {
                for color in used_colors.iter() {
                    if candidate.forbidden_colors.contains(&color) {
                        continue;
                    } else {
                        match self.nodes.get(candidate.graph_key) {
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

        log_context!(("conflict graph"){
            debug!("conflict graph of {} nodes colored in {:?} passes, used {:?} colors", self.nodes.len(), passes, used_colors.len());
        });

        Ok(())
    }
    
    #[allow(dead_code)]
    fn check(&self) -> Result<(), ConflictGraphError> {
        for node in self.nodes.iter().enumerate() {
            for other in self.nodes.iter().enumerate() {
                if node.0 != other.0 {

                    // uncolored nodes indicate an invalid graph
                    if node.1.color.get() == UNCOLORED { return Err(ConflictGraphError::UncoloredNode) }
                    if other.1.color.get() == UNCOLORED { return Err(ConflictGraphError::UncoloredNode) }

                    match node.1.conflict_cmp(other.1) {
                        true => {
                            if node.1.color.get() == other.1.color.get() { return Err(ConflictGraphError::UnresolvedConflict) }
                        },
                        false => {
                            // no conflict expected
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

impl<N> From<ConflictGraph<N>> for Result<Vec<Vec<N>>, ConflictGraphError> where N: ConflictCmp {
    /// Converts the ConflictGraph into an unordered conflict free set of "cliques"
    ///
    /// Each inner Vec represents a set of nodes which are mutually conflict free
    /// while nodes in separate Vec's likely (but aren't guaranteed to) conflict with each other
    fn from(graph: ConflictGraph<N>) -> Self {        
        let mut result: HashMap<usize, Vec<N>> = HashMap::new();

        for (_i, node) in graph.nodes.into_iter().enumerate() {
            result.entry(node.color.get()).or_insert_with(|| Vec::default()).push(node.udata);
        }

        Ok(result.into_iter().map(|item| item.1).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // simple LCG RNG
    // threw this together as a quick solution for testing that doesn't require any external dependencies
    static mut _QUICK_RAND_SEED: u128 = 0;
    fn quick_rand() -> usize {
        unsafe { 
            if _QUICK_RAND_SEED == 0 {
                _QUICK_RAND_SEED = ::std::time::SystemTime::now().duration_since(::std::time::UNIX_EPOCH).unwrap().as_micros();
                debug!("quick rand seed: {:?}", _QUICK_RAND_SEED);
            }

            let a = 492876863; // some big primes
            let c = 15485867;
            let m = 2u128.pow(33);
            _QUICK_RAND_SEED = (a * _QUICK_RAND_SEED + c) % m;

            return _QUICK_RAND_SEED as usize;
        }
    }

    #[derive(Debug, Clone)]
    struct TestNode {
        writes: Vec<usize>,
        reads: Vec<usize>,
    }

    impl ConflictCmp for TestNode {
        fn conflict_cmp(&self, other: &Self) -> bool {
            // conflict if two nodes write the same data or one node reads and the other node writes

            for read in &self.reads {
                for write in &other.writes {
                    if read == write {
                        return true
                    }
                }
            }

            for write in &self.writes {
                for read in &other.reads {
                    if write == read {
                        return true
                    }
                }

                for other_write in &other.writes {
                    if write == other_write {
                        return true
                    }
                }
            }
            return false
        }
    }
    
    fn do_graph(nodes: &[TestNode]) -> (ConflictGraph<TestNode>, Vec<Vec<TestNode>>) {
        let mut graph = ConflictGraph::new();
                
        for node in nodes.iter() {
            graph.insert((*node).clone()).unwrap();
        }
        
        let cliques = graph.clone().cliques().unwrap();

        return (graph, cliques)
    }

    fn expect_cliques(expected: usize, result: (ConflictGraph<TestNode>, Vec<Vec<TestNode>>)) -> (ConflictGraph<TestNode>, Vec<Vec<TestNode>>) {
        assert_eq!(result.1.len(), expected);
        result
    }

    fn validate_conflict_free(result: (ConflictGraph<TestNode>, Vec<Vec<TestNode>>)) -> (ConflictGraph<TestNode>, Vec<Vec<TestNode>>) {
        for clique in result.1.iter().enumerate() {
            for node in clique.1.iter().enumerate() {
                for other in clique.1.iter().enumerate() {
                    if node.0 != other.0 {
                        match node.1.conflict_cmp(other.1) {
                            true => {
                                println!("\nfound conflict in test data");
                                println!("node {} conflicts with node {} in clique {}", node.0, other.0, clique.0);
                                println!("{} total nodes in the graph", result.0.nodes.len());
                                print_cliques(&result.1);
                                print_cliques_details(&result.1);
                                // println!("graph dump\n{:#?}", result.0);
                                assert!(false);
                            }
                            false => continue,
                        }
                    }
                }
            }
        }
        return result
    }

    // useful if you need to debug something here and quickly print the output
    #[allow(dead_code)]
    fn print_cliques(cliques: &Vec<Vec<TestNode>>) {
        println!("{:?} cliques", cliques.len());
        for clique in cliques.iter().enumerate() {
            println!("   clique {:?} size {:?}", clique.0, clique.1.len());
        }
    }

    #[allow(dead_code)]
    fn print_cliques_details(cliques: &Vec<Vec<TestNode>>) {
        println!("details");
        for clique in cliques.iter().enumerate() {
            println!("{:?}", clique);
        }
    }

    #[test]
    fn test_cliques() {
        println!("");

        log_context!(("test") {

            log_context!(("asjhdbaskdb") {
                debug!("test message");
            });
            
            
        let nodes = [
            TestNode { writes: vec![0], reads: vec![0] },
            TestNode { writes: vec![1], reads: vec![1] },
            TestNode { writes: vec![2], reads: vec![2] },
            TestNode { writes: vec![4], reads: vec![4] },
            TestNode { writes: vec![5], reads: vec![5] },
            TestNode { writes: vec![6], reads: vec![6] },
            TestNode { writes: vec![7], reads: vec![7] },
            TestNode { writes: vec![8], reads: vec![8] },
        ];
        
        
        expect_cliques(1, validate_conflict_free(do_graph(&nodes)));
        
        let nodes = [
            TestNode { writes: vec![0], reads: vec![0] },
            TestNode { writes: vec![0], reads: vec![1] },
            TestNode { writes: vec![0], reads: vec![0] },
            TestNode { writes: vec![0], reads: vec![1] },
            TestNode { writes: vec![0], reads: vec![0] },
            TestNode { writes: vec![0], reads: vec![1] },
            TestNode { writes: vec![0], reads: vec![0] },
            TestNode { writes: vec![0], reads: vec![1] },
        ];
        
        expect_cliques(8, validate_conflict_free(do_graph(&nodes)));

        let nodes = [
            TestNode { writes: vec![0], reads: vec![0] },
            TestNode { writes: vec![1], reads: vec![1] },
            TestNode { writes: vec![0], reads: vec![0] },
            TestNode { writes: vec![1], reads: vec![1] },
            TestNode { writes: vec![5], reads: vec![0] },
            TestNode { writes: vec![6], reads: vec![1] },
            TestNode { writes: vec![7], reads: vec![0] },
            TestNode { writes: vec![8], reads: vec![1] },
        ];

        expect_cliques(3, validate_conflict_free(do_graph(&nodes)));

        let nodes = [
            TestNode { writes: vec![0], reads: vec![0, 1] },
            TestNode { writes: vec![1], reads: vec![1, 2] },
            TestNode { writes: vec![2], reads: vec![0, 1, 2, 3] },
            TestNode { writes: vec![4], reads: vec![1, 2, 3] },
            TestNode { writes: vec![2], reads: vec![2, 3] },
            TestNode { writes: vec![2], reads: vec![3] },
            TestNode { writes: vec![3], reads: vec![1, 3] },
            TestNode { writes: vec![4], reads: vec![0, 3] },
        ];

        expect_cliques(6, validate_conflict_free(do_graph(&nodes)));

        debug!("beginning random data test");
        
        // test some random input
        const ITERATIONS: usize = 10;
        for i in 0..ITERATIONS {
            let max_nodes: usize = 40;
            let max_writes: usize = 2;
            let max_reads: usize = 3;
            let rw_options: usize = 30;
            
            let mut nodes = Vec::new();

            for _ in 0..(quick_rand() % max_nodes) {
                let mut writes = Vec::new();
                let mut reads = Vec::new();
                for _ in 0..(quick_rand() % max_writes) {
                    writes.push(quick_rand() % rw_options);
                }

                for _ in 0..(quick_rand() % max_reads) {
                    reads.push(quick_rand() % rw_options);
                }

                nodes.push(TestNode {
                    writes: writes,
                    reads: reads,
                })
            }

            match std::panic::catch_unwind(|| {
                validate_conflict_free(do_graph(&nodes));
            }) {
                Ok(()) => {},
                Err(e) => {
                    println!("failed on iteration {}, {:?}", i, e);
                    panic!();
                }
            }
        }

        ::std::thread::sleep(::std::time::Duration::from_millis(10));
        });
    }
}
