#![cfg_attr(feature = "client", allow(incomplete_features))]
#![cfg_attr(
    feature = "client",
    feature(associated_type_defaults, const_caller_location, specialization,)
)]

#[cfg(not(any(feature = "anyhow", feature = "color-eyre")))]
compile_error!("One of `anyhow` or `color-eyre` features must be enabled.");
#[cfg(all(feature = "anyhow", feature = "color-eyre"))]
compile_error!("Cannot compile with both `anyhow` and `color-eyre` features enabled.");

#[cfg(all(feature = "async-graphql-4", feature = "async-graphql-5"))]
compile_error!("More than one version of the subdependency `async-graphql` was enabled, please only enable one by only using one of the features: `async-graphql-4`, `async-graphql-5`, `async-graphql-6`.");
#[cfg(all(feature = "async-graphql-5", feature = "async-graphql-6"))]
compile_error!("More than one version of the subdependency `async-graphql` was enabled, please only enable one by only using one of the features: `async-graphql-4`, `async-graphql-5`, `async-graphql-6`.");
#[cfg(all(feature = "async-graphql-6", feature = "async-graphql-4"))]
compile_error!("More than one version of the subdependency `async-graphql` was enabled, please only enable one by only using one of the features: `async-graphql-4`, `async-graphql-5`, `async-graphql-6`.");

#[cfg(feature = "anyhow")]
pub(crate) use anyhow::Error as InternalError;

#[cfg(feature = "color-eyre")]
pub(crate) use color_eyre::Report as InternalError;

#[macro_use]
extern crate async_backtrace;
#[macro_use]
extern crate cfg_if;
#[macro_use]
extern crate derivative;

mod env;
mod error;

pub use env::*;
pub use error::*;

pub use lazy_static::lazy_static as service_util_lazy_static;
pub use paste::paste as service_util_paste;

cfg_if! {
    if #[cfg(feature = "client")] {
        mod client;
        pub use client::*;
    }
}
cfg_if! {
    if #[cfg(feature = "server")] {
        mod server;
        pub use server::*;

        pub use tokio as service_util_tokio;

        #[macro_export]
        macro_rules! try_join_safe {
            ($($expr:expr),+ $(,)?) => { $crate::try_join_safe!($($expr,)+ ~ @ _a ~ $($expr,)+) };
            ($($orig:expr,)* ~ $($final_ident:ident,)* @ $ident:ident ~ $expr:expr, $($remaining:expr,)*) => {
                $crate::service_util_paste! {
                    $crate::try_join_safe! { $($orig,)* ~ $($final_ident,)* $ident, @ [<_ $ident>] ~ $($remaining,)* }
                }
            };
            ( $($expr:expr,)* ~ $($final_ident:ident,)* @ $ident:ident ~) => {
                {
                    let ($($final_ident),*) = $crate::service_util_tokio::join!($($expr),+);
                    $crate::try_join_safe!( @ @ $($final_ident)* )
                }
            };
            (@ $($used:ident)* @ $ident:ident $($remaining:ident)*) => {
                $ident.and_then(move |$ident| $crate::try_join_safe!( @ $($used)* $ident @ $($remaining)* ))
            };
            (@ $($used:ident)* @) => {
                Ok(($($used),*))
            };
        }

        #[allow(clippy::manual_async_fn)]
        pub fn try_join_all_safe<I, T: Send, E: Send>(iter: I) -> impl futures::Future<Output = Result<Vec<T>, E>> + Send
        where
            I: IntoIterator + Send,
            <I as IntoIterator>::Item: futures::Future<Output = Result<T, E>> + Send,
        {
            async move {
                futures::future::join_all(iter).await.into_iter().collect()
            }
        }
    }
}
cfg_if! {
    if #[cfg(feature = "trace")] {
        mod trace;
        pub use trace::*;
    }
}
