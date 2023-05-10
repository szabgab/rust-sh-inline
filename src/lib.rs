//! # This crate is deprecated
//!
//! This crate is considered deprecated!  Please use [xshell](https://crates.io/crates/xshell) instead.
//!
//! # Macros to run bash scripts inline in Rust
//!
//! In many cases it's convenient to run child processes,
//! particularly via Unix shell script.  Writing the
//! Rust code to use `std::process::Command` directly
//! will get very verbose quickly.  You can generate
//! a script "manually" by using e.g. `format!()` but
//! there are some important yet subtle things to get right,
//! such as dealing with quoting issues.
//!
//! This macro takes Rust variable names at the start
//! that are converted to a string (quoting as necessary)
//! and bound into the script as bash variables.
//!
//! Further, the generated scripts use "bash strict mode"
//! by default, i.e. `set -euo pipefail`.
//!
//! ```
//! use sh_inline::*;
//! let foo = "variable with spaces";
//! bash!(r#"test "${foo}" = 'variable with spaces'"#, foo)?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! This generates and executes bash script as follows:
//! ```sh
//! set -euo pipefail
//! foo="variable with spaces"
//! test ${foo} = 'variable with spaces'
//! ```
//!
//! # Related crates
//!
//! [`xshell`] is a crate that does not depend on bash, and also supports
//! inline formatting.
//!
//! [`xshell`]: https://docs.rs/xshell/latest/xshell/

#[doc(hidden)]
pub mod internals;

#[cfg(feature = "cap-std-ext")]
pub use cap_std_ext;
#[cfg(feature = "cap-std-ext")]
pub use cap_std_ext::cap_std;

/// Create a [`Command`] object that will execute a fragment of (Bash) shell script
/// in "strict mode", i.e. with `set -euo pipefail`.  The first argument is the
/// script, and additional arguments should be Rust variable identifiers.  The
/// provided Rust variables will become shell script variables with their values
/// quoted.
///
/// This macro will allocate a temporary file for the script; this can (in very
/// unusual cases such as file descriptior exhaustion) fail.
///
/// ```
/// use sh_inline::*;
/// let a = "foo";
/// let b = std::path::Path::new("bar");
/// let c = 42;
/// let d: String = "baz".into();
/// let r = bash_command!(r#"test "${a} ${b} ${c}" = "foo bar 42""#, a, b, c).expect("creating script").status()?;
/// assert!(r.success());
/// let r = bash_command!(r#"test "${a}" = "2""#, a = 1 + 1).expect("creating script").status()?;
/// assert!(r.success());
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// [`Command`]: https://doc.rust-lang.org/std/process/struct.Command.html
#[macro_export]
macro_rules! bash_command {
    ($s:expr) => { $crate::bash_command!($s,) };
    ($s:expr, $( $id:ident = $v:expr),*) => {
        {
            use std::fmt::Write;
            let mut script: String = "set -euo pipefail\n".into();
            $(
                write!(&mut script, "{}={}\n", stringify!($id), $crate::internals::CommandArg::from(&$v)).unwrap();
            )*
            $crate::internals::render(&$s, script)
        }
    };
    ($s:expr, $( $id:ident ),*) => { $crate::bash_command!($s, $($id = $id),*) };
}

/// Execute a fragment of Bash shell script, returning an error if the subprocess exits unsuccessfully.
/// This is intended as a convenience macro for the common case of wanting to just propagate
/// errors.  The returned error type is [std::io::Error](https://doc.rust-lang.org/std/io/struct.Error.html).
///
/// For more details on usage, see the [`bash_command`](./macro.bash_command.html) macro.
///
/// ```
/// use sh_inline::*;
/// let a = "foo";
/// let b = std::path::Path::new("bar");
/// bash!(r#"test "${a} ${b} ${c}" = "foo bar 42""#, a = a, b = b, c = 42)?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[macro_export]
macro_rules! bash {
    ($s:expr) => { $crate::bash!($s,) };
    ($s:expr, $( $id:ident = $v:expr ),*) => {
        $crate::internals::execute($crate::bash_command!($s, $( $id = $v ),*).expect("failed to create temporary script")
    )
    };
    ($s:expr, $( $id:ident ),*) => {
        $crate::internals::execute($crate::bash_command!($s, $( $id ),*).expect("failed to create temporary script")
    )
    };
}

/// Execute a fragment of Bash shell script with the specified working directory.
///
/// Otherwise this is equivalent to the [`bash`] macro.
///
/// ```
/// use sh_inline::*;
/// use std::sync::Arc;
/// let td = Arc::new(cap_tempfile::tempdir(cap_std::ambient_authority())?);
/// td.write("sometestfile", "test file contents")?;
/// bash_in!(td, r#"grep -qF "test file contents" sometestfile"#)?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[macro_export]
#[cfg(feature = "cap-std-ext")]
macro_rules! bash_in {
    ($cwd:expr, $s:expr) => { $crate::bash_in!($cwd, $s,) };
    ($cwd:expr, $s:expr, $( $id:ident = $v:expr ),*) => {
        { use cap_std_ext::cmdext::CapStdExtCommandExt;
            let mut cmd = $crate::bash_command!($s, $( $id = $v ),*).expect("failed to create temporary script");
            cmd.cwd_dir($cwd.try_clone().expect("cloning dir"));
            $crate::internals::execute(cmd)
    }
    };
    ($cwd: expr, $s:expr, $( $id:ident ),*) => { $crate::bash_in!($cwd, $s, $($id = $id),*) };
}
