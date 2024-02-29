#[macro_export]
macro_rules! timer_start {
    ($name:ident) => {
        timer_start!($name, "");
    };
    ($name:ident, $arg:expr) => {
        #[allow(non_snake_case)]
        let $name = std::time::Instant::now();
        debug!("{}    start >>> {}{}{}", "\x1b[2m", stringify!($name), $arg, "\x1b[37;0m");
    };
}

#[macro_export]
macro_rules! timer_stop {
    ($name:ident) => {
        #[allow(non_snake_case)]
        let $name = std::time::Instant::now() - $name;
        debug!("{}     stop <<< {}{}", "\x1b[2m", stringify!($name), "\x1b[37;0m");
    };
}

#[macro_export]
macro_rules! timer_stop_step {
    ($name:ident, $step:expr) => {
        #[allow(non_snake_case)]
        let $name = std::time::Instant::now() - $name;
        debug!("{}     stop <<< {}{}{}", "\x1b[2m", stringify!($name), $step, "\x1b[37;0m");
    };
}

#[macro_export]
macro_rules! timer_log {
    ($name:ident) => {
        debug!("{} duration --- {} {}ms{}", "\x1b[2m", stringify!($name), $name.as_millis(), "\x1b[37;0m");
    };
}

#[macro_export]
macro_rules! timer_stop_and_log {
    ($name:ident) => {
        timer_stop_and_log!($name, "");
    };
    ($name:ident, $arg:expr) => {
        #[allow(non_snake_case)]
        let $name = std::time::Instant::now() - $name;
        debug!("{}     stop <<< {}{} {}ms{}", "\x1b[2m", stringify!($name), $arg, $name.as_millis(), "\x1b[37;0m");
    };
}