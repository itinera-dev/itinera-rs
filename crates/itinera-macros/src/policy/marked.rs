//! The functions of a policy's block that its attribute reads: its hooks and its factory.

use syn::{Attribute, ImplItemFn, ItemImpl};

use super::factory::Factory;
use super::hook::{Hook, HookName};
use super::{Kind, MARKS};
use crate::{block, marker};

/// The hooks the block defines, in order, and its factory, if it has one.
#[derive(Default)]
pub(super) struct Marked {
    pub(super) hooks: Vec<Hook>,
    pub(super) factory: Option<Factory>,
}

/// A function of the block marked as a hook, not read yet, or as the factory, and its mark.
enum Function<'a> {
    Hook(HookName, &'a mut ImplItemFn, Attribute),
    Factory(Factory, Attribute),
}

impl Marked {
    /// Reads the marked functions of the block, and removes their marks.
    pub(super) fn read(block: &mut ItemImpl, kind: Kind) -> syn::Result<Self> {
        let functions = block::functions(block)
            .map(|function| marked(function, kind))
            .collect::<syn::Result<Vec<_>>>()?;
        functions
            .into_iter()
            .flatten()
            .try_fold(Self::default(), Self::with)
    }

    /// Adds a marked function, unless the block already holds one marked the same way.
    fn with(mut self, function: Function<'_>) -> syn::Result<Self> {
        match function {
            Function::Hook(name, function, mark) => {
                if self.hooks.iter().any(|hook| hook.defines(name)) {
                    return Err(syn::Error::new_spanned(
                        mark,
                        format!(
                            "{} is defined once, and another function of the block defines it",
                            name.spoken()
                        ),
                    ));
                }
                self.hooks.push(Hook::read(name, function)?);
            }
            Function::Factory(factory, mark) => {
                if self.factory.is_some() {
                    return Err(syn::Error::new_spanned(mark, "a policy has one factory"));
                }
                self.factory = Some(factory);
            }
        }
        Ok(self)
    }
}

/// What the function is marked as, if anything, read for a policy of this kind.
fn marked(function: &mut ImplItemFn, kind: Kind) -> syn::Result<Option<Function<'_>>> {
    let marks = marker::take(&mut function.attrs, &MARKS);
    let Some(mark) = marker::at_most_one(marks, "a function is one hook, or the factory")? else {
        return Ok(None);
    };
    mark.meta.require_path_only()?;
    let Ok(name) = marker::name(&mark).parse::<HookName>() else {
        let factory = Factory::read(function)?;
        return Ok(Some(Function::Factory(factory, mark)));
    };
    if !name.is_of(kind) {
        return Err(syn::Error::new_spanned(
            mark,
            format!(
                "{} is not a hook of a policy described by `{}`",
                name.spoken(),
                kind.attribute()
            ),
        ));
    }
    Ok(Some(Function::Hook(name, function, mark)))
}
