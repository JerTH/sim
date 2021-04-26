use crate::collections::SparseSet;

/// Flowgraph

enum InsertionError<N> where N: FlowGraphNode {
    PushFailed(N)
}

pub trait FlowGraphNode {
    type Dependency: Clone;
    fn write_deps(&self) -> Vec<Self::Dependency>;
    fn read_deps(&self) -> Vec<Self::Dependency>;
}

#[derive(Debug)]
struct InternalFlowGraphNode<N> where N: FlowGraphNode {
    predecessors: Vec<FlowGraphKey>,
    successors: Vec<FlowGraphKey>,
    data: N,
}

impl<N> InternalFlowGraphNode<N> where N: FlowGraphNode {
    fn new(node: N) -> Self {
        InternalFlowGraphNode {
            predecessors: Vec::new(),
            successors: Vec::new(),
            data: node,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct FlowGraphKey(usize);

#[derive(Debug)]
pub struct FlowGraph<N> where N: FlowGraphNode {
    nodes: SparseSet<InternalFlowGraphNode<N>>,
    first: Vec<FlowGraphKey>, // First nodes to be executed, functions as a sort of "root"

    // When the flow graph is executed, any nodes that fail will have their errors returned

    // Running a flow graph can either be blocking or async-awaited
}

impl<N: FlowGraphNode> FlowGraph<N> {
    fn new() -> Self {
        Self {
            nodes: SparseSet::new(),
            first: Vec::new(),
        }
    }
    
    fn insert(&mut self, node: N) -> Result<FlowGraphKey, InsertionError<N>> {
        let wd = node.write_deps();
        let rd = node.read_deps();

        // for now, we don't care about before and after constraints, or similar runtime constraints
        // we just care about running things such that co-dependent things never run together
        
        if self.first.is_empty() {
            // First node added
            match self.nodes.insert(InternalFlowGraphNode::new(node)) {
                // Push succeeded
                Ok(key) => {
                    let key = FlowGraphKey(key);
                    self.first.push(key);
                    return Ok(key);
                },
                // Push failed, got the node back
                Err(internal_node) => {
                    return Err(InsertionError::PushFailed(internal_node.data));
                }
            }
        } else {
            unimplemented!() // Find the correct spot to add it
        }
    }

    fn remove(&mut self, key: FlowGraphKey) -> Result<N, ()> {
        Err(())
    }

    fn rebuild(&mut self) -> Result<(), ()> {
        Err(())
    }
}
