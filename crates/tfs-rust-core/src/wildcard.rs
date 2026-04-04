//! Prefix trie for player names (`Player::getName` / wildcard lookups).
// C++ reference: `WildcardTree` (`wildcardtree.cpp`).

use std::collections::HashMap;

#[derive(Debug, Default)]
struct Node {
    children: HashMap<char, Node>,
    terminal: bool,
}

/// Case-sensitive prefix tree (TFS name rules can be layered on top).
#[derive(Debug, Default)]
pub struct WildcardTree {
    root: Node,
}

impl WildcardTree {
    pub fn insert(&mut self, name: &str) {
        let mut n = &mut self.root;
        for ch in name.chars() {
            n = n.children.entry(ch).or_default();
        }
        n.terminal = true;
    }

    pub fn remove(&mut self, name: &str) -> bool {
        let chars: Vec<char> = name.chars().collect();
        Self::remove_chars(&mut self.root, &chars, 0)
    }

    fn remove_chars(node: &mut Node, chars: &[char], i: usize) -> bool {
        if i == chars.len() {
            let was = node.terminal;
            node.terminal = false;
            return was;
        }
        let ch = chars[i];
        let Some(child) = node.children.get_mut(&ch) else {
            return false;
        };
        let ok = Self::remove_chars(child, chars, i + 1);
        if ok && child.children.is_empty() && !child.terminal {
            node.children.remove(&ch);
        }
        ok
    }

    pub fn get_by_prefix(&self, prefix: &str) -> Vec<String> {
        let mut n = &self.root;
        for ch in prefix.chars() {
            match n.children.get(&ch) {
                Some(c) => n = c,
                None => return Vec::new(),
            }
        }
        let mut out = Vec::new();
        Self::collect(n, &mut String::from(prefix), &mut out);
        out
    }

    fn collect(node: &Node, buf: &mut String, out: &mut Vec<String>) {
        if node.terminal {
            out.push(buf.clone());
        }
        for (&ch, child) in &node.children {
            buf.push(ch);
            Self::collect(child, buf, out);
            buf.pop();
        }
    }
}
