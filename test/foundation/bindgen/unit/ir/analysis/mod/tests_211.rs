#[cfg(test)]
mod tests {
    use super::*;
    use crate::HashSet;

    // Here we find the set of nodes that are reachable from any given
    // node. This is a lattice mapping nodes to subsets of all nodes. Our join
    // function is set union.
    //
    // This is our test graph:
    //
    //     +---+                    +---+
    //     |   |                    |   |
    //     | 1 |               .----| 2 |
    //     |   |               |    |   |
    //     +---+               |    +---+
    //       |                 |      ^
    //       |                 |      |
    //       |      +---+      '------'
    //       '----->|   |
    //              | 3 |
    //       .------|   |------.
    //       |      +---+      |
    //       |        ^        |
    //       v        |        v
    //     +---+      |      +---+    +---+
    //     |   |      |      |   |    |   |
    //     | 4 |      |      | 5 |--->| 6 |
    //     |   |      |      |   |    |   |
    //     +---+      |      +---+    +---+
    //       |        |        |        |
    //       |        |        |        v
    //       |      +---+      |      +---+
    //       |      |   |      |      |   |
    //       '----->| 7 |<-----'      | 8 |
    //              |   |             |   |
    //              +---+             +---+
    //
    // And here is the mapping from a node to the set of nodes that are
    // reachable from it within the test graph:
    //
    //     1: {3,4,5,6,7,8}
    //     2: {2}
    //     3: {3,4,5,6,7,8}
    //     4: {3,4,5,6,7,8}
    //     5: {3,4,5,6,7,8}
    //     6: {8}
    //     7: {3,4,5,6,7,8}
    //     8: {}

    #[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
    struct Node(usize);

    #[derive(Clone, Debug, Default, PartialEq, Eq)]
    struct Graph(HashMap<Node, Vec<Node>>);

    impl Graph {
        fn make_test_graph() -> Graph {
            let mut g = Graph::default();
            g.0.insert(Node(1), vec![Node(3)]);
            g.0.insert(Node(2), vec![Node(2)]);
            g.0.insert(Node(3), vec![Node(4), Node(5)]);
            g.0.insert(Node(4), vec![Node(7)]);
            g.0.insert(Node(5), vec![Node(6), Node(7)]);
            g.0.insert(Node(6), vec![Node(8)]);
            g.0.insert(Node(7), vec![Node(3)]);
            g.0.insert(Node(8), vec![]);
            g
        }

        fn reverse(&self) -> Graph {
            let mut reversed = Graph::default();
            for (node, edges) in &self.0 {
                reversed.0.entry(*node).or_insert_with(Vec::new);
                for referent in edges {
                    reversed
                        .0
                        .entry(*referent)
                        .or_insert_with(Vec::new)
                        .push(*node);
                }
            }
            reversed
        }
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    struct ReachableFrom<'a> {
        reachable: HashMap<Node, HashSet<Node>>,
        graph: &'a Graph,
        reversed: Graph,
    }

    impl<'a> MonotoneFramework for ReachableFrom<'a> {
        type Node = Node;
        type Extra = &'a Graph;
        type Output = HashMap<Node, HashSet<Node>>;

        fn new(graph: &'a Graph) -> Self {
            let reversed = graph.reverse();
            ReachableFrom {
                reachable: Default::default(),
                graph,
                reversed,
            }
        }

        fn initial_worklist(&self) -> Vec<Node> {
            self.graph.0.keys().copied().collect()
        }

        fn constrain(&mut self, node: Node) -> ConstrainResult {
            // The set of nodes reachable from a node `x` is
            //
            //     reachable(x) = s_0 U s_1 U ... U reachable(s_0) U reachable(s_1) U ...
            //
            // where there exist edges from `x` to each of `s_0, s_1, ...`.
            //
            // Yes, what follows is a **terribly** inefficient set union
            // implementation. Don't copy this code outside of this test!

            let original_size = self.reachable.entry(node).or_default().len();

            for sub_node in &self.graph.0[&node] {
                self.reachable.get_mut(&node).unwrap().insert(*sub_node);

                let sub_reachable =
                    self.reachable.entry(*sub_node).or_default().clone();

                for transitive in sub_reachable {
                    self.reachable.get_mut(&node).unwrap().insert(transitive);
                }
            }

            let new_size = self.reachable[&node].len();
            if original_size == new_size {
                ConstrainResult::Same
            } else {
                ConstrainResult::Changed
            }
        }

        fn each_depending_on<F>(&self, node: Node, mut f: F)
        where
            F: FnMut(Node),
        {
            for dep in &self.reversed.0[&node] {
                f(*dep);
            }
        }
    }

    impl<'a> From<ReachableFrom<'a>> for HashMap<Node, HashSet<Node>> {
        fn from(reachable: ReachableFrom<'a>) -> Self {
            reachable.reachable
        }
    }

    #[test]
    fn monotone() {
        let g = Graph::make_test_graph();
        let reachable = analyze::<ReachableFrom>(&g);
        println!("reachable = {reachable:#?}");

        fn nodes<A>(nodes: A) -> HashSet<Node>
        where
            A: AsRef<[usize]>,
        {
            nodes.as_ref().iter().copied().map(Node).collect()
        }

        let mut expected = HashMap::default();
        expected.insert(Node(1), nodes([3, 4, 5, 6, 7, 8]));
        expected.insert(Node(2), nodes([2]));
        expected.insert(Node(3), nodes([3, 4, 5, 6, 7, 8]));
        expected.insert(Node(4), nodes([3, 4, 5, 6, 7, 8]));
        expected.insert(Node(5), nodes([3, 4, 5, 6, 7, 8]));
        expected.insert(Node(6), nodes([8]));
        expected.insert(Node(7), nodes([3, 4, 5, 6, 7, 8]));
        expected.insert(Node(8), nodes([]));
        println!("expected = {expected:#?}");

        assert_eq!(reachable, expected);
    }
}
