mod parser;

use crate::expr::Expr;
use anyhow::{Context, Result};
use core::hash::{Hash, Hasher};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, serde::Serialize)]
pub struct Directives {
    pub build_command: Option<String>,
    pub build_inputs: Vec<Expr>,
    pub interpreter: Option<String>,
    pub runtime_inputs: Vec<Expr>,
    pub root: Option<PathBuf>,
}

impl Directives {
    pub fn parse(indicator: &str, source: &str) -> Result<Self> {
        let parser = parser::Parser::new(indicator).context("could not construct a parser")?;
        let fields = parser.parse(source);

        Self::from_directives(fields)
    }

    fn from_directives(fields: HashMap<&str, Vec<&str>>) -> Result<Self> {
        let build_command = Self::once("build", &fields)?.map(|s| s.to_owned());
        let build_inputs = Self::exprs("buildInputs", &fields)?;
        let interpreter = Self::once("interpreter", &fields)?.map(|s| s.to_owned());
        let runtime_inputs = Self::exprs("runtimeInputs", &fields)?;
        let root = Self::once("root", &fields)?.map(PathBuf::from);

        Ok(Directives {
            build_command,
            build_inputs,
            interpreter,
            runtime_inputs,
            root,
        })
    }

    fn once<'field>(
        field: &'field str,
        fields: &HashMap<&'field str, Vec<&'field str>>,
    ) -> Result<Option<&'field str>> {
        match fields.get(field) {
            Some(value) => {
                if value.len() != 1 {
                    anyhow::bail!("I got multiple `{}` directives, and I don't know which to use. Remove all but one and try again!", field);
                }

                Ok(Some(value[0]))
            }
            None => Ok(None),
        }
    }

    fn exprs<'field>(
        field: &'field str,
        fields: &HashMap<&'field str, Vec<&'field str>>,
    ) -> Result<Vec<Expr>> {
        match fields.get(field) {
            None => Ok(Vec::new()),
            Some(lines) => {
                Expr::parse_as_list(&lines.join(" ")).context("could not parse runtime inputs")
            }
        }
    }

    pub fn maybe_override_build_command(&mut self, maybe_new: &Option<String>) {
        if maybe_new.is_some() {
            self.build_command = maybe_new.to_owned()
        }
    }

    pub fn maybe_override_interpreter(&mut self, maybe_new: &Option<String>) {
        if maybe_new.is_some() {
            self.interpreter = maybe_new.to_owned()
        }
    }
}

impl Hash for Directives {
    fn hash<H: Hasher>(&self, hasher: &mut H) {
        if let Some(build_command) = &self.build_command {
            hasher.write(build_command.as_ref())
        }

        for input in &self.build_inputs {
            input.hash(hasher)
        }

        if let Some(interpreter) = &self.interpreter {
            hasher.write(interpreter.as_ref())
        }

        for input in &self.runtime_inputs {
            input.hash(hasher)
        }

        if let Some(root) = &self.root {
            hasher.write(root.display().to_string().as_ref())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod from_directives {
        use super::*;

        #[test]
        fn only_one_build_command_allowed() {
            let problem = Directives::from_directives(HashMap::from([("build", vec!["a", "b"])]))
                .unwrap_err();

            assert_eq!(
                String::from("I got multiple `build` directives, and I don't know which to use. Remove all but one and try again!"),
                problem.to_string(),
            )
        }

        #[test]
        fn combines_build_inputs() {
            let directives =
                Directives::from_directives(HashMap::from([("buildInputs", vec!["a b", "c d"])]))
                    .unwrap();

            let expected: Vec<Expr> = vec![
                Expr::parse("a").unwrap(),
                Expr::parse("b").unwrap(),
                Expr::parse("c").unwrap(),
                Expr::parse("d").unwrap(),
            ];

            assert_eq!(expected, directives.build_inputs);
        }

        #[test]
        fn only_one_interpreter_allowed() {
            let problem =
                Directives::from_directives(HashMap::from([("interpreter", vec!["a", "b"])]))
                    .unwrap_err();

            assert_eq!(
                String::from("I got multiple `interpreter` directives, and I don't know which to use. Remove all but one and try again!"),
                problem.to_string(),
            )
        }

        #[test]
        fn combines_runtime_inputs() {
            let directives =
                Directives::from_directives(HashMap::from([("runtimeInputs", vec!["a b", "c d"])]))
                    .unwrap();

            let expected: Vec<Expr> = vec![
                Expr::parse("a").unwrap(),
                Expr::parse("b").unwrap(),
                Expr::parse("c").unwrap(),
                Expr::parse("d").unwrap(),
            ];

            assert_eq!(expected, directives.runtime_inputs);
        }

        #[test]
        fn only_one_root_allowed() {
            let problem =
                Directives::from_directives(HashMap::from([("root", vec!["a", "b"])])).unwrap_err();

            assert_eq!(
                String::from("I got multiple `root` directives, and I don't know which to use. Remove all but one and try again!"),
                problem.to_string(),
            )
        }

        #[test]
        fn sets_root() {
            let directives =
                Directives::from_directives(HashMap::from([("root", vec!["."])])).unwrap();

            assert_eq!(Some(PathBuf::from(".")), directives.root)
        }
    }

    mod hash {
        use super::*;

        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        fn assert_have_different_hashes<H: Hash>(l: H, r: H) {
            let mut l_hasher = DefaultHasher::new();
            let mut r_hasher = DefaultHasher::new();

            l.hash(&mut l_hasher);
            r.hash(&mut r_hasher);

            println!("l: {}, r: {}", l_hasher.finish(), r_hasher.finish());
            assert!(l_hasher.finish() != r_hasher.finish())
        }

        #[test]
        fn build_command_changes_hash() {
            assert_have_different_hashes(
                Directives::from_directives(HashMap::from([("build", vec!["a"])])).unwrap(),
                Directives::from_directives(HashMap::from([("build", vec!["b"])])).unwrap(),
            )
        }

        #[test]
        fn build_inputs_changes_hash() {
            assert_have_different_hashes(
                Directives::from_directives(HashMap::from([("buildInputs", vec!["a"])])).unwrap(),
                Directives::from_directives(HashMap::from([("buildInputs", vec!["b"])])).unwrap(),
            )
        }

        #[test]
        fn interpreter_changes_hash() {
            assert_have_different_hashes(
                Directives::from_directives(HashMap::from([("interpreter", vec!["a"])])).unwrap(),
                Directives::from_directives(HashMap::from([("interpreter", vec!["b"])])).unwrap(),
            )
        }

        #[test]
        fn runtime_inputs_changes_hash() {
            assert_have_different_hashes(
                Directives::from_directives(HashMap::from([("runtimeInputs", vec!["a"])])).unwrap(),
                Directives::from_directives(HashMap::from([("runtimeInputs", vec!["b"])])).unwrap(),
            )
        }

        #[test]
        fn root_changes_hash() {
            assert_have_different_hashes(
                Directives::from_directives(HashMap::from([("root", vec!["a"])])).unwrap(),
                Directives::from_directives(HashMap::from([("root", vec!["b"])])).unwrap(),
            )
        }
    }
}
