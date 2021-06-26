#[macro_export]
macro_rules! step {
    ($name:expr $(, $arg:expr)+ $(,)?) => {{
        eprint!("\x1b[1m{:>8}\x1b[m ", $name);
        eprint!( $($arg),+ );
    }};
}

#[macro_export]
macro_rules! stepln {
    ($name:expr, $msg:literal $(, $arg:expr)* $(,)?) => {{
        step!($name, concat!($msg, "\n"), $($arg),* );
    }};
}
