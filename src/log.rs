#[macro_export]
macro_rules! log_async {
    ($db:expr, $formatstr:expr, $($args:expr),*) => {{
        let source = format!("{}:{}", file!(), line!());
        let message = format!($formatstr, $($args),*);
        eprintln!("{}: {}", source, message);
        $db.insert_log_message(source, message).await?;
    }};
}

#[macro_export]
macro_rules! log_error_async {
    ($db:expr, $name:expr, $result:expr) => {{
        match $result {
            Ok(x) => Ok(x),
            Err(e) => {
                log_async!($db, "{}: {}", $name, e);
                Err(e)
            }
        }
    }};
}
