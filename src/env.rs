use std::str::FromStr;
use thiserror::Error;

// env produces for each input:
// - a pub const with the same name as the provided identifier
// - a fn which attempts to extract that environment variable and parse it into the specified type
#[macro_export]
macro_rules! env {
    () => {};
    ($var:ident: $($tt:tt)*) => { $crate::service_util_paste! {
        pub const $var: &str = stringify!($var);
        fn [<has_set_ $var:lower>]() -> bool { [<$var _VALUE>].read().unwrap().is_some() }

        $crate::env! { @ $var $($tt)* }
    } };
    (@ use_env $var:ident $ty:ty | $setter:expr) => {
        $crate::service_util_paste! {
            pub fn [<use_ $var:lower>]<'a, T: 'a, F>(f: F) -> Result<T, $crate::EnvError>
            where
                F: FnOnce(&$ty) -> T + 'a,
            {
                if [<has_set_ $var:lower>]() {
                    let lock = [<$var _VALUE>].read().unwrap();
                    let res = lock.as_ref().unwrap();
                    match res {
                        Ok(ok) => Ok(f(ok)),
                        Err(err) => Err(err.clone()),
                    }
                } else {
                    let res = $setter;
                    let output = match &res {
                        Ok(ok) => Ok(f(&ok)),
                        Err(err) => Err(err.clone()),
                    };
                    let mut write_lock = [<$var _VALUE>].write().unwrap();
                    *write_lock = Some(res);
                    output
                }
            }
        }
    };
    (@ $var:ident Option<$ty:ty> $(, $($tt:tt)*)?) => { $crate::service_util_paste! {
        $crate::service_util_lazy_static! {
            static ref [<$var _VALUE>]: std::sync::Arc<std::sync::RwLock<Option<Result<Option<$ty>, $crate::EnvError>>>> = std::sync::Arc::new(std::sync::RwLock::new(None));
        }

        pub fn [<$var:lower>]() -> Result<Option<$ty>, $crate::EnvError> {
            if [<has_set_ $var:lower>]() {
                [<$var _VALUE>].read().unwrap().as_ref().unwrap().clone()
            } else {
                let res = $crate::service_util_opt_env($var);
                let mut write_lock = [<$var _VALUE>].write().unwrap();
                *write_lock = Some(res.clone());
                res
            }
        }

        $crate::env! { @ use_env $var Option<$ty> | $crate::service_util_opt_env($var) }

        $($crate::env! { $($tt)* })?
    } };
    (@ $var:ident $ty:ty $(, $($tt:tt)*)?) => { $crate::service_util_paste! {
        $crate::service_util_lazy_static! {
            static ref [<$var _VALUE>]: std::sync::Arc<std::sync::RwLock<Option<Result<$ty, $crate::EnvError>>>> = std::sync::Arc::new(std::sync::RwLock::new(None));
        }

        pub fn [<$var:lower>]() -> Result<$ty, $crate::EnvError> {
            if [<has_set_ $var:lower>]() {
                [<$var _VALUE>].read().unwrap().as_ref().unwrap().clone()
            } else {
                let res = $crate::service_util_env($var);
                let mut write_lock = [<$var _VALUE>].write().unwrap();
                *write_lock = Some(res.clone());
                res
            }
        }

        $crate::env! { @ use_env $var $ty | $crate::service_util_env($var) }

        $($crate::env! { $($tt)* })?
    } };
    (@ $var:ident $ty:ty = $expr:expr $(, $($tt:tt)*)?) => { $crate::service_util_paste! {
        $crate::service_util_lazy_static! {
            static ref [<$var _VALUE>]: std::sync::Arc<std::sync::RwLock<Option<Result<$ty, $crate::EnvError>>>> = std::sync::Arc::new(std::sync::RwLock::new(None));
        }

        pub fn [<$var:lower>]() -> Result<$ty, $crate::EnvError> {
            if [<has_set_ $var:lower>]() {
                [<$var _VALUE>].read().unwrap().as_ref().unwrap().clone()
            } else {
                let res = $crate::service_util_opt_env($var).map(|x| x.unwrap_or_else(|| { $expr }.into()));
                let mut write_lock = [<$var _VALUE>].write().unwrap();
                *write_lock = Some(res.clone());
                res
            }
        }

        $crate::env! { @ use_env $var $ty | $crate::service_util_env($var) }

        $($crate::env! { $($tt)* })?
    } };
    (@ $var:ident Option<$ty:ty> | $map_fn:path $(, $($tt:tt)*)?) => { $crate::service_util_paste! {
        $crate::service_util_lazy_static! {
            static ref [<$var _VALUE>]: std::sync::Arc<std::sync::RwLock<Option<Result<Option<$ty>, $crate::EnvError>>>> = std::sync::Arc::new(std::sync::RwLock::new(None));
        }

        pub fn [<$var:lower>]() -> Result<Option<$ty>, $crate::EnvError> {
            if [<has_set_ $var:lower>]() {
                [<$var _VALUE>].read().unwrap().as_ref().unwrap().clone()
            } else {
                let res = $crate::service_util_opt_env($var).map(|x| x.map($map_fn));
                let mut write_lock = [<$var _VALUE>].write().unwrap();
                *write_lock = Some(res.clone());
                res
            }
        }

        $crate::env! { @ use_env $var Option<$ty> | $crate::service_util_opt_env($var) }

        $($crate::env! { $($tt)* })?
    } };
    (@ $var:ident $ty:ty | $map_fn:path $(, $($tt:tt)*)?) => { $crate::service_util_paste! {
        $crate::service_util_lazy_static! {
            static ref [<$var _VALUE>]: std::sync::Arc<std::sync::RwLock<Option<Result<$ty, $crate::EnvError>>>> = std::sync::Arc::new(std::sync::RwLock::new(None));
        }

        pub fn [<$var:lower>]() -> Result<$ty, $crate::EnvError> {
            if [<has_set_ $var:lower>]() {
                [<$var _VALUE>].read().unwrap().as_ref().unwrap().clone()
            } else {
                let res = $crate::service_util_env($var).map($map_fn);
                let mut write_lock = [<$var _VALUE>].write().unwrap();
                *write_lock = Some(res.clone());
                res
            }
        }

        $crate::env! { @ use_env $var $ty | $crate::service_util_env($var).map($map_fn) }

        $($crate::env! { $($tt)* })?
    } };
}

#[derive(Clone, Debug, Error)]
pub enum EnvError {
    #[error("invalid value for environment variable `{0}`")]
    InvalidValue(&'static str),
    #[error("missing required environment variable `{0}`")]
    Missing(&'static str),
}

pub fn service_util_env<T: FromStr>(var_name: &'static str) -> Result<T, EnvError> {
    service_util_opt_env(var_name)?.ok_or(EnvError::Missing(var_name))
}

pub fn service_util_opt_env<T: FromStr>(var_name: &'static str) -> Result<Option<T>, EnvError> {
    if let Ok(value) = std::env::var(var_name) {
        value.parse().map(Some).or(Err(EnvError::InvalidValue(var_name)))
    } else {
        Ok(None)
    }
}

pub fn parse_allowed_origins(allowed_origins: String) -> Vec<hyper::http::HeaderValue> {
    allowed_origins
        .split(',')
        .map(TryFrom::try_from)
        .collect::<Result<_, _>>()
        .unwrap()
}
