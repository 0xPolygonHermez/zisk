#[macro_export]
macro_rules! timer_start {
    ($name:ident) => {
        #[allow(non_snake_case)]
        let $name = std::time::Instant::now();
        debug!("{}    start >>> {}{}", "\x1b[2m", stringify!($name), "\x1b[37;0m");
    };
}

#[macro_export]
macro_rules! timer_start_step {
    ($name:ident, $step:expr) => {
        #[allow(non_snake_case)]
        let $name = std::time::Instant::now();
        debug!("{}    start >>> {}{}{}", "\x1b[2m", stringify!($name), $step, "\x1b[37;0m");
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
        #[allow(non_snake_case)]
        let $name = std::time::Instant::now() - $name;
        debug!("{}     stop <<< {} {}ms{}", "\x1b[2m", stringify!($name), $name.as_millis(), "\x1b[37;0m");
    };
}
