use crate::db::Database;

pub struct Logger {
    db: Database,
}

impl Logger {
    pub async fn log(&self, source: String, message: String) -> anyhow::Result<()> {
        eprintln!("{}: {}", source, message);
        self.db.insert_log_message(source, message).await
    }
}

macro_rules! log {
    ($logger:expr, $formatstr:expr, $($args:tt),*) => {
        $logger.log(format!("{}:{}", file!(), line!()), format!($formatstr, $($args),*))
    };
}
