//! The module of functions that the checks of a workflow's services call as the program
//! compiles: the first service a step requests that no provider serves, and the message saying
//! so.

use proc_macro2::TokenStream;
use quote::quote;

use super::super::checks;

/// The module, which also holds the functions comparing the steps' names.
pub(super) fn functions() -> TokenStream {
    let steps = checks::steps();
    quote! {
        mod __itinera_services {
            #steps

            pub(super) type Provided = (&'static str, &'static [&'static str]);

            /// The first key the step requests that no provider serves, if the providers are
            /// known; the step is not named for a rescue step, which only a provider attached
            /// to no step serves.
            pub(super) const fn unprovided(
                providers: ::core::option::Option<&[Provided]>,
                step: ::core::option::Option<&str>,
                keys: &[&'static str],
            ) -> ::core::option::Option<&'static str> {
                match providers {
                    ::core::option::Option::Some(providers) => first_unserved(providers, step, keys),
                    ::core::option::Option::None => ::core::option::Option::None,
                }
            }

            const fn first_unserved(
                providers: &[Provided],
                step: ::core::option::Option<&str>,
                keys: &[&'static str],
            ) -> ::core::option::Option<&'static str> {
                match keys {
                    [] => ::core::option::Option::None,
                    [key, rest @ ..] => {
                        if served(providers, step, key) {
                            first_unserved(providers, step, rest)
                        } else {
                            ::core::option::Option::Some(*key)
                        }
                    }
                }
            }

            const fn served(
                providers: &[Provided],
                step: ::core::option::Option<&str>,
                key: &str,
            ) -> bool {
                match providers {
                    [] => false,
                    [(provided, steps), rest @ ..] => {
                        (__itinera_steps::same(provided.as_bytes(), key.as_bytes())
                            && (steps.is_empty() || attached(steps, step)))
                            || served(rest, step, key)
                    }
                }
            }

            const fn attached(steps: &[&str], step: ::core::option::Option<&str>) -> bool {
                match step {
                    ::core::option::Option::Some(step) => __itinera_steps::listed(steps, step),
                    ::core::option::Option::None => false,
                }
            }

            /// A message made of parts, up to the length of its buffer.
            pub(super) struct Message {
                bytes: [u8; 512],
                length: usize,
            }

            pub(super) const fn message(parts: &[&str]) -> Message {
                let mut bytes = [0; 512];
                let length = written(&mut bytes, parts);
                Message { bytes, length }
            }

            const fn written(buffer: &mut [u8], parts: &[&str]) -> usize {
                match parts {
                    [] => 0,
                    [part, rest @ ..] => {
                        let length = copied(buffer, part.as_bytes());
                        let (_, buffer) = buffer.split_at_mut(length);
                        length + written(buffer, rest)
                    }
                }
            }

            const fn copied(buffer: &mut [u8], part: &[u8]) -> usize {
                match (buffer.split_first_mut(), part) {
                    (::core::option::Option::Some((first, buffer)), [byte, part @ ..]) => {
                        *first = *byte;
                        1 + copied(buffer, part)
                    }
                    _ => 0,
                }
            }

            pub(super) const fn text(message: &Message) -> &str {
                let (bytes, _) = message.bytes.split_at(message.length);
                match ::core::str::from_utf8(bytes) {
                    ::core::result::Result::Ok(text) => text,
                    ::core::result::Result::Err(_) => "service not provided",
                }
            }
        }
    }
}
