# 🚫 Errors

> The greatest mistake is to imagine that we never err.
>
> — _Thomas Carlyle_.

Errors in Rust are a bit ambiguous:

- the infamous [`Result<T, E>`][`Result`] is a type that can either be `Ok(T)` or `Err(E)`, where `E` is
  the error type in case something went wrong.
- the [`std::error::Error`] trait is a trait that represents errors that can be displayed and
  have a source (cause).

The ambiguity comes from the fact that the [`std::error::Error`] trait is not required to be
implemented for the error type `E` in the [`Result<T, E>`][`Result`] type. This means that one can have
a [`Result<T, E>`][`Result`] where `E` is not an error type. A common example of something else it can be
is that it has the same type as the `T` type, which is not an error type. E.g. in case of a web
service middleware a firewall could return a 403 Http response as the `Err` variant of the
[`Result<T, Response>`][`Result`]. Where `T` is most likely also a `Response` type. In which
case you might as well have `Result<Response, Infallible>`.

Within Web Services we usually do not want an error type, as it does not make any sense.
This is because the server has to respond something (unless you simply want to kill the connection),
and so it makes much more sense to enforce the code type-wise to always return a response.

The most tricky scenario, if you can call it that, is what to do for middleware services.
These situations are tricky because they can wrap any generic `S` type, where `S` is the
service type. This means that the error type can be anything, and so it is not possible to
create values of that type for scenarios where the error comes from the middleware itself.

There are several possibilities here and we'll go over them next. But before we do that,
I do want to emphasise that while Rust's [`Result<T, E>`][`Result`] does not enforce that `E` is an error
type, it is still a good practice to use an error type for the `E` type. And that is also
that as a rule of thumb we do in Rama.

## Type Erasure

The [`BoxError`] type alias is a boxed Error trait object and can be used to represent any error that
implements the [`std::error::Error`] trait and is used for cases where it is usually not
that important what specific error type is returned, but rather that an error occurred.
Boxed errors do allow to _downcast_ to check for concrete error types, but this checks
only the top-level error and not the cause chain.

## Error Extension

The [`ErrorExt`] trait provides a set of methods to work with errors. These methods are
implemented for all types that implement the [`std::error::Error`] trait. The methods are
used to add context to an error, add a backtrace to an error, and to convert an error into
an opaque error.

The extension also allows one to iterate over the chain of errors ([`ErrorExt::chain`]),
which is useful for manually checking if certain errors happened,
and if so extract info out of them.

In case you only care about the most top-level error of a specific type you can use
[`ErrorExt::has_error`]. To get the root cause, regardless of the type you can use
[`ErrorExt::root_cause`].

### Error Extension Example

```rust
use rama::error::{BoxError, ErrorExt, OpaqueError};

#[derive(Debug)]
struct CustomError;

impl std::fmt::Display for CustomError {
   fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
    write!(f, "Custom error")
  }
}

impl std::error::Error for CustomError {}

#[derive(Debug)]
struct IoError(BoxError);

impl std::fmt::Display for IoError {
  fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
     write!(f, "IO error")
  }
}

impl std::error::Error for IoError {
  fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
     Some(self.0.as_ref())
  }
}

let error = IoError(CustomError.into()).context("whoops");
assert!(error.root_cause().downcast_ref::<CustomError>().is_some());
assert!(error.has_error::<IoError>().is_some());
assert!(error.has_error::<CustomError>().is_some());
```

## Opaque Error

The [`OpaqueError`] type is a type-erased error that can be used to represent any error
that implements the [`std::error::Error`] trait. Using the [`OpaqueError::from_display`]
you can even create errors from a displayable type.

The other advantage of [`OpaqueError`] over [`BoxError`]
is that it is Sized and can be used in places where a `Sized`` type is required,
while [`BoxError`] is `?Sized` and can give you a hard time in certain scenarios.

## `error` macro

The [`error`] macro is a convenient way to create an [`OpaqueError`]
from an error, format string or displayable type.

### `error` macro Example

```rust
use rama::error::{error, ErrorExt, OpaqueError};

let error = error!("error").context("foo");
assert_eq!(error.to_string(), "foo: error");

let error = error!("error {}", 404).context("foo");
assert_eq!(error.to_string(), "foo: error 404");

#[derive(Debug)]
struct CustomError;

impl std::fmt::Display for CustomError {
  fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
     write!(f, "entity not found")
  }
}

impl std::error::Error for CustomError {}

let error = error!(CustomError).context("foo");

assert_eq!(error.to_string(), "foo: entity not found");
assert!(error.root_cause().downcast_ref::<CustomError>().is_some());
```

## Error Context

The [`ErrorContext`] allows you to add a context to [`Result`]
and [`Option`] types:

- For [`Result`] types, the context is added to the error variant,
  turning `Result<T, E>` into `Result<T, OpaqueError>`;
- For [`Option`] types, the context is used as a DisplayError when
  the open is `None`, turning `Option<T>` into `Result<T, OpaqueError>`.

This is useful when you want to add custom context.
And can also be combined with other [`ErrorExt`] methods,
such as [`ErrorExt::backtrace`] to add even more info to the error case,
if there is one.

It is also an easy way to turn an option value into the inner value,
short-circuiting using `?` with the new context (Display) error
when the option was `None`.

### Error Context Example

Option Example:

```rust
use rama::error::{ErrorContext, ErrorExt};

let value = Some(42);
let value = match value.context("value is None") {
   Ok(value) => assert_eq!(value, 42),
   Err(error) => panic!("unexpected error: {error}"),
};

let value: Option<usize> = None;
let result = value.context("value is None");
assert!(result.is_err());
```

Result Example:

```rust
use rama::error::{ErrorContext, ErrorExt, OpaqueError};

let value: Result<_, OpaqueError> = Ok(42);
let value = match value.context("get the answer") {
  Ok(value) => assert_eq!(value, 42),
  Err(error) => panic!("unexpected error: {error}"),
};

let value: Result<usize, _> = Err(OpaqueError::from_display("error"));
let result = value.context("get the answer");
assert!(result.is_err());
```

## Error Composition

TODO: Add documentation for error composition.
Requires <https://github.com/plabayo/rama/issues/162>
to be implemented and resolved.

[`BoxError`]: https://ramaproxy.org/docs/rama/error/type.BoxError.html
[`OpaqueError`]: https://ramaproxy.org/docs/rama/error/type.OpaqueError.html
[`OpaqueError::from_display`]: https://ramaproxy.org/docs/rama/error/struct.OpaqueError.html#method.from_display
[`ErrorExt`]: https://ramaproxy.org/docs/rama/error/trait.ErrorExt.html
[`ErrorExt::chain`]: https://ramaproxy.org/docs/rama/error/trait.ErrorExt.html#tymethod.chain
[`ErrorExt::has_error`]: https://ramaproxy.org/docs/rama/error/trait.ErrorExt.html#tymethod.has_error
[`ErrorExt::root_cause`]: https://ramaproxy.org/docs/rama/error/trait.ErrorExt.html#method.root_cause
[`ErrorExt::backtrace`]: https://ramaproxy.org/docs/rama/error/trait.ErrorExt.html#tymethod.backtrace
[`ErrorContext`]: https://ramaproxy.org/docs/rama/error/trait.ErrorContext.html
[`Result`]: https://doc.rust-lang.org/stable/std/result/enum.Result.html
[`Option`]: https://doc.rust-lang.org/stable/std/option/enum.Option.html
[`error`]: https://ramaproxy.org/docs/rama/error/macro.error.html
[`std::error::Error`]: https://doc.rust-lang.org/stable/std/error/trait.Error.html