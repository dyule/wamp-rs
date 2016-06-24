//! Contains the `PatternNode` struct, which is used for constructing a trie corresponding
//! to pattern based subscription
use super::{ConnectionInfo, random_id};
use ::{ID, URI, WampResult, Error, ErrorKind, MatchingPolicy};
use messages::Reason;
use std::sync::{Arc};
use std::cell::RefCell;
use std::collections::HashMap;
use std::slice::Iter;
use std::mem;
use std::fmt::{Debug, Formatter, self};


/// Contains a trie corresponding to the subscription patterns that connections have requested.
///
/// Each level of the trie corresponds to a fragment of a uri between the '.' character.
/// Thus each subscription that starts with 'com' for example will be grouped together.
/// Subscriptions can be added and removed, and the connections that match a particular URI
/// can be iterated over using the `filter()` method.
///
/// # Example
///
/// ```
/// # use wamp::{URI, MatchingPolicy, ID};
/// # use wamp::router::patterns::{PatternNode, PatternData};
/// # struct MockData {
/// #     id: ID
/// # }
/// #
/// # impl PatternData for MockData {
/// #     fn get_id(&self) -> ID {
/// #         self.id
/// #     }
/// # }
/// #
/// # impl MockData {
/// #     fn new(id: ID) -> MockData {
/// #        MockData {
/// #            id: id
/// #        }
/// #     }
/// # }
/// #
/// # let connection1 = MockData::new(1);
/// # let connection2 = MockData::new(2);
/// # let connection3 = MockData::new(3);
/// # let connection4 = MockData::new(4);
/// let mut root = PatternNode::new();
///
/// root.subscribe_with(&URI::new("com.example.test..topic"), connection1,
///                     MatchingPolicy::Wildcard).unwrap();
/// root.subscribe_with(&URI::new("com.example.test.specific.topic"), connection2,
///                     MatchingPolicy::Strict).unwrap();
/// root.subscribe_with(&URI::new("com.example"), connection3, MatchingPolicy::Prefix).unwrap();
/// root.subscribe_with(&URI::new("com.example.test"), connection4, MatchingPolicy::Prefix).unwrap();
/// for (connection, _id, _policy) in root.filter(URI::new("com.example.test.specific.topic")) {
///      println!("Connection ID: {}", connection.get_id());
///      // Will print connections ids in the order 3, 4, 1, 2
///      // The `_id` is a randomly assigned value for that subscription
///      // `_policy` is the `MatchingPolicy` that was used when the connection was added
/// }
/// ```

pub struct PatternNode<P:PatternData> {
    edges: HashMap<String, PatternNode<P>>,
    connections: Vec<DataWrapper<P>>,
    prefix_connections: Vec<DataWrapper<P>>,
    id: ID,
    prefix_id: ID
}

/// Represents data that a pattern trie will hold
pub trait PatternData {
    fn get_id(&self) -> ID;
}

struct DataWrapper<P: PatternData> {
    subscriber: P,
    policy: MatchingPolicy
}

/// A lazy iterator that traverses the pattern trie.  See `PatternNode` for more.
pub struct MatchIterator<'a, P> where
        P : PatternData,
        P : 'a {
    uri: Vec<String>,
    current: Box<StackFrame<'a, P>>
}

struct StackFrame<'a, P> where
        P : PatternData,
        P : 'a {
    node: &'a PatternNode<P>,
    state: IterState<'a, P>,
    depth: usize,
    parent: Option<Box<StackFrame<'a, P>>>
}

#[derive(Clone)]
enum IterState<'a, P: PatternData> where
        P : PatternData,
        P : 'a {
    None,
    Wildcard,
    Strict,
    Prefix(Iter<'a, DataWrapper<P>>),
    PrefixComplete,
    Subs(Iter<'a, DataWrapper<P>>),
    AllComplete
}

impl PatternData for Arc<RefCell<ConnectionInfo>> {
    fn get_id(&self) -> ID {
        self.borrow().id
    }
}

impl<'a, P:PatternData> Debug for IterState<'a, P>{
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", match self {
            &IterState::None => "None",
            &IterState::Wildcard => "Wildcard",
            &IterState::Strict => "Strict",
            &IterState::Prefix(_) => "Prefix",
            &IterState::PrefixComplete => "PrefixComplete",
            &IterState::Subs(_) => "Subs",
            &IterState::AllComplete => "AllComplete"
        })
    }
}

impl<P:PatternData> Debug for PatternNode <P>{
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        self.fmt_with_indent(f, 0)
    }
}

impl<P:PatternData> PatternNode<P> {

    fn fmt_with_indent(&self, f: &mut Formatter, indent: usize) -> fmt::Result {
        try!(writeln!(f, "{} pre: {:?} subs: {:?}",
            self.id,
            self.prefix_connections.iter().map(|sub| sub.subscriber.get_id()).collect::<Vec<_>>(),
            self.connections.iter().map(|sub| sub.subscriber.get_id()).collect::<Vec<_>>()));
        for (chunk, node) in self.edges.iter() {
            for _ in 0..indent * 2 {
                try!(write!(f, "  "));
            }
            try!(write!(f, "{} - ", chunk));
            try!(node.fmt_with_indent(f, indent + 1));
        }
        Ok(())
    }

    /// Add a new subscription to the pattern trie with the given pattern and matching policy.
    pub fn subscribe_with(&mut self, topic: &URI, subscriber: P, matching_policy: MatchingPolicy) -> WampResult<ID> {
        let mut uri_bits = topic.uri.split(".");
        let initial = match uri_bits.next() {
            Some(initial) => initial,
            None          => return Err(Error::new(ErrorKind::ErrorReason(Reason::InvalidURI)))
        };
        let edge = self.edges.entry(initial.to_string()).or_insert(PatternNode::new());
        edge.add_subscription(uri_bits, subscriber, matching_policy)
    }

    /// Removes a subscription from the pattern trie.
    pub fn unsubscribe_with(&mut self, topic: &str, subscriber: &P, is_prefix: bool) -> WampResult<(ID)> {
        let uri_bits = topic.split(".");
        self.remove_subscription(uri_bits, subscriber.get_id(), is_prefix)
    }

    /// Constructs a new PatternNode to be used as the root of the trie
    #[inline]
    pub fn new() -> PatternNode<P> {
        PatternNode {
            edges: HashMap::new(),
            connections: Vec::new(),
            prefix_connections: Vec::new(),
            id: random_id(),
            prefix_id: random_id()
        }
    }

    fn add_subscription<'a, I>(&mut self, mut uri_bits: I, subscriber: P, matching_policy: MatchingPolicy) -> WampResult<ID> where I: Iterator<Item=&'a str> {
        match uri_bits.next() {
            Some(uri_bit) => {
                if uri_bit.len() == 0 {
                    if matching_policy != MatchingPolicy::Wildcard {
                        return Err(Error::new(ErrorKind::ErrorReason(Reason::InvalidURI)));
                    }
                }
                let edge = self.edges.entry(uri_bit.to_string()).or_insert(PatternNode::new());
                edge.add_subscription(uri_bits, subscriber, matching_policy)
            },
            None => {
                if matching_policy == MatchingPolicy::Prefix {
                    self.prefix_connections.push(DataWrapper {
                        subscriber: subscriber,
                        policy: matching_policy
                    });
                    Ok(self.prefix_id)
                } else {
                    self.connections.push(DataWrapper {
                        subscriber: subscriber,
                        policy: matching_policy
                    });
                    Ok(self.id)
                }
            }
        }
    }

    fn remove_subscription<'a, I>(&mut self, mut uri_bits: I, subscriber_id: u64, is_prefix: bool) -> WampResult<(ID)> where I: Iterator<Item=&'a str> {
        // TODO consider deleting nodes in the tree if they are no longer in use.
        match uri_bits.next() {
            Some(uri_bit) => {
                if let Some(mut edge) = self.edges.get_mut(uri_bit) {
                    edge.remove_subscription(uri_bits, subscriber_id, is_prefix)
                } else {
                    return Err(Error::new(ErrorKind::ErrorReason(Reason::InvalidURI)))
                }
            },
            None => {
                if is_prefix {
                    self.prefix_connections.retain(|sub| sub.subscriber.get_id() != subscriber_id);
                    Ok((self.prefix_id))
                } else {
                    self.connections.retain(|sub| sub.subscriber.get_id() != subscriber_id);
                    Ok((self.id))
                }
            }
        }
    }

    /// Constructs a lazy iterator over all of the connections whose subscription patterns
    /// match the given uri.
    ///
    /// This iterator returns a triple with the connection info, the id of the subscription and
    /// the matching policy used when the subscription was created.
    pub fn filter<'a>(&'a self, topic: URI) -> MatchIterator<'a, P> {
        MatchIterator {
            current: Box::new(StackFrame {
                node: self,
                depth: 0,
                state: IterState::None,
                parent: None
            }),
            uri: topic.uri.split('.').map(|s| s.to_string()).collect()
        }
    }
 }

impl <'a, P: PatternData> MatchIterator<'a, P> {

    fn push(&mut self, child: &'a PatternNode<P>) {
        let new_node = Box::new(StackFrame {
            parent: None,
            depth: self.current.depth + 1,
            node: child,
            state: IterState::None
        });
        let parent = mem::replace(&mut self.current, new_node);
        self.current.parent = Some(parent);
    }

    /// Moves through the subscription tree, looking for the next set of connections that match the
    /// given uri.
    fn traverse(&mut self) -> Option<(&'a P, ID, MatchingPolicy)> {
        // This method functions as a push down automata.  For each node, it starts by iterating
        // through the data that match a prefix of the uri
        // Then when that's done, it checks if the uri has been fully processed, and if so, iterates
        // through the connections that require exact matching
        // Otherwise, it pushes the current node on the stack, consumes another chunk of the uri
        // and moves on to any children that use wildcard matching.
        // Once it is finished traversing that part of the tree, it re-consumes the same chunk
        // of the URI, and moves on to any children that match the chunk exactly.
        // After all that is exhausted, it will pop the node of the stack and return to its parent
        match self.current.state {
            IterState::None  => {
                 self.current.state = IterState::Prefix(self.current.node.prefix_connections.iter())
            },
            IterState::Prefix(_) => {
                self.current.state = IterState::PrefixComplete;
            },
            IterState::PrefixComplete => {
                if self.current.depth == self.uri.len() {
                    self.current.state = IterState::Subs(self.current.node.connections.iter());
                } else {
                    if let Some(child) = self.current.node.edges.get("") {
                        self.current.state = IterState::Wildcard;
                        self.push(child);
                    } else {
                        if let Some(child) = self.current.node.edges.get(&self.uri[self.current.depth]) {
                            self.current.state = IterState::Strict;
                            self.push(child);
                        }
                        else {
                            self.current.state = IterState::AllComplete;
                        }
                    }
                }
           },
           IterState::Wildcard => {
               if self.current.depth == self.uri.len() {
                   self.current.state = IterState::AllComplete;
               } else {
                   if let Some(child) = self.current.node.edges.get(&self.uri[self.current.depth]) {
                       self.current.state = IterState::Strict;
                       self.push(child);
                   } else {
                       self.current.state = IterState::AllComplete;
                   }
               }
           },
           IterState::Strict => {
               self.current.state = IterState::AllComplete;
           },
           IterState::Subs(_) => {

               self.current.state = IterState::AllComplete;
           },
           IterState::AllComplete => {
               if self.current.depth == 0 {
                   return None
               } else {
                   let parent = self.current.parent.take();
                   mem::replace(&mut self.current, parent.unwrap());
               }
           }
       };
        self.next()
    }
}

 impl <'a, P: PatternData> Iterator for MatchIterator<'a, P> {
     type Item = (&'a P, ID, MatchingPolicy);

     fn next(&mut self) -> Option<(&'a P, ID, MatchingPolicy)> {

         // If we are currently iterating through connections, continue iterating
         match self.current.state {
             IterState::Prefix(ref mut prefix_iter) => {
                 let next = prefix_iter.next();
                 if let Some(next) = next {
                     return Some((&next.subscriber, self.current.node.prefix_id.clone(), next.policy))
                 }
            },
            IterState::Subs(ref mut sub_iter) => {
                let next = sub_iter.next();
                if let Some(next) = next {
                    return Some((&next.subscriber, self.current.node.prefix_id.clone(), next.policy))
                }
            },
            _ => {}

        };

        // Otherwise, it is time to traverse through the tree.
        self.traverse()

     }
 }

 #[cfg(test)]
 mod test {
     use ::{URI, MatchingPolicy, ID};
     use super::{PatternNode, PatternData};

     #[derive(Clone)]
     struct MockData {
         id: ID
     }

     impl PatternData for MockData {
         fn get_id(&self) -> ID {
             self.id
         }
     }
     impl MockData {
         pub fn new(id: ID) -> MockData {
            MockData {
                id: id
            }
         }
     }

     #[test]
     fn adding_patterns() {
          let connection1 = MockData::new(1);
          let connection2 = MockData::new(2);
          let connection3 = MockData::new(3);
          let connection4 = MockData::new(4);
          let mut root = PatternNode::new();

          root.subscribe_with(&URI::new("com.example.test..topic"), connection1, MatchingPolicy::Wildcard).unwrap();
          root.subscribe_with(&URI::new("com.example.test.specific.topic"), connection2, MatchingPolicy::Strict).unwrap();
          root.subscribe_with(&URI::new("com.example"), connection3, MatchingPolicy::Prefix).unwrap();
          root.subscribe_with(&URI::new("com.example.test"), connection4, MatchingPolicy::Prefix).unwrap();

          assert_eq!(root.filter(URI::new("com.example.test.specific.topic")).map(|(connection, _id, _policy)| connection.get_id()).collect::<Vec<_>>(), vec![
            3, 4, 1, 2
          ]);

     }

     #[test]
     fn removing_patterns() {
        let connection1 = MockData::new(1);
        let connection2 = MockData::new(2);
        let connection3 = MockData::new(3);
        let connection4 = MockData::new(4);
        let mut root = PatternNode::new();

        root.subscribe_with(&URI::new("com.example.test..topic"), connection1.clone(), MatchingPolicy::Wildcard).unwrap();
        root.subscribe_with(&URI::new("com.example.test.specific.topic"), connection2, MatchingPolicy::Strict).unwrap();
        root.subscribe_with(&URI::new("com.example"), connection3, MatchingPolicy::Prefix).unwrap();
        root.subscribe_with(&URI::new("com.example.test"), connection4.clone(), MatchingPolicy::Prefix).unwrap();

        root.unsubscribe_with("com.example.test..topic", &connection1, false).unwrap();
        root.unsubscribe_with("com.example.test", &connection4, true).unwrap();

        assert_eq!(root.filter(URI::new("com.example.test.specific.topic")).map(|(connection, _id, _policy)| connection.get_id()).collect::<Vec<_>>(), vec![
          3, 2
        ]);
     }
 }
