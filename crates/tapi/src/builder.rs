use std::collections::BTreeMap;

use crate::DynTapi;

pub struct TypesBuilder {
    pub prelude: String,
    pub start_namespace: Box<dyn Fn(&[String], &str) -> String>,
    pub end_namespace: Box<dyn Fn(&[String], &str) -> String>,
    pub decl: Box<dyn Fn(DynTapi) -> Option<String>>,
}

impl TypesBuilder {
    pub fn types(&self, tys: impl IntoIterator<Item = DynTapi>) -> String {
        let mut s = self.prelude.trim_start().to_string();

        pub struct Node<'a> {
            path: Vec<String>,
            builder: &'a TypesBuilder,
            children: BTreeMap<String, Node<'a>>,
            decls: Vec<DynTapi>,
        }

        let mut root = Node::new(Vec::new(), self);

        for ty in tys {
            let mut node = &mut root;
            let mut path = Vec::new();
            for p in ty.path() {
                node = node
                    .children
                    .entry(p.to_string())
                    .or_insert_with(|| Node::new(path.clone(), self));
                path.push(p.to_string());
            }
            node.decls.push(ty);
        }

        impl<'a> Node<'a> {
            fn new(path: Vec<String>, builder: &'a TypesBuilder) -> Self {
                Self {
                    builder,
                    path,
                    children: Default::default(),
                    decls: Default::default(),
                }
            }

            fn write(&self, s: &mut String, indent: usize) {
                for decl in &self.decls {
                    if let Some(decl) = (self.builder.decl)(*decl) {
                        for l in decl.lines() {
                            for _ in 0..indent {
                                s.push_str("  ");
                            }
                            s.push_str(l);
                            s.push('\n');
                        }
                    }
                }
                for (name, node) in &self.children {
                    for _ in 0..indent {
                        s.push_str("  ");
                    }
                    s.push_str(&(self.builder.start_namespace)(&node.path, name));
                    s.push('\n');
                    node.write(s, indent + 1);
                    for _ in 0..indent {
                        s.push_str("  ");
                    }
                    s.push_str(&(self.builder.end_namespace)(&node.path, name));
                    s.push('\n');
                }
            }
        }

        root.write(&mut s, 0);

        s
    }
}
