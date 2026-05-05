//! 运行时子模块：`panic`。

use std::any::Any;
use std::fs::OpenOptions;
use std::io::Write;
use std::panic;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

const CRASH_LOG_PATH: &str = "crash.log";
const ISSUE_TEMPLATE: &str = "issues/new?template=bug_report.yml";

static CRASH_LOG: OnceLock<Option<Mutex<std::fs::File>>> = OnceLock::new();

#[derive(Debug, Clone)]
struct PanicContext {
    version: &'static str,
    platform: &'static str,
    repository: &'static str,
    timestamp: String,
    thread_name: String,
    location: String,
    message: String,
}

/// 安装自定义 panic hook。
///
/// crash.log 文件句柄在首次 panic 时惰性打开后复用（`OnceLock`），
/// 写入用 `try_lock()` 避免 panic storm 下的递归死锁或无限阻塞。
pub fn install_panic_hook() {
    panic::set_hook(Box::new(|info| {
        let thread = std::thread::current();
        let context = PanicContext {
            version: env!("CARGO_PKG_VERSION"),
            platform: std::env::consts::OS,
            repository: env!("CARGO_PKG_REPOSITORY"),
            timestamp: chrono::Local::now()
                .format("%Y-%m-%d %H:%M:%S%.3f")
                .to_string(),
            thread_name: thread.name().unwrap_or("<unnamed>").to_string(),
            location: info
                .location()
                .map(|loc| format!("{}:{}:{}", loc.file(), loc.line(), loc.column()))
                .unwrap_or_else(|| "<unknown>".to_string()),
            message: panic_payload_message(info.payload()),
        };

        let crash_log_path = crash_log_display_path();
        let crash_logged = write_crash_report(&context);

        eprintln!(
            "{}",
            render_user_panic_notice(&context, &crash_log_path, crash_logged)
        );
    }));
}

fn write_crash_report(context: &PanicContext) -> bool {
    let Some(file_mutex) = crash_log_file() else {
        return false;
    };

    let Ok(mut guard) = file_mutex.try_lock() else {
        return false;
    };

    // Backtrace::force_capture 是同步阻塞操作，在 panic storm 下会拖慢所有线程。
    // 只在实际持有 crash.log 写锁时 capture，stderr 行只打轻量信息。
    let backtrace = std::backtrace::Backtrace::force_capture().to_string();
    let crash_report = render_crash_report(context, &backtrace);
    guard.write_all(crash_report.as_bytes()).is_ok()
}

fn crash_log_file() -> Option<&'static Mutex<std::fs::File>> {
    CRASH_LOG
        .get_or_init(|| {
            OpenOptions::new()
                .create(true)
                .append(true)
                .open(CRASH_LOG_PATH)
                .map(Mutex::new)
                .ok()
        })
        .as_ref()
}

fn crash_log_display_path() -> PathBuf {
    std::env::current_dir()
        .map(|dir| dir.join(CRASH_LOG_PATH))
        .unwrap_or_else(|_| PathBuf::from(CRASH_LOG_PATH))
}

fn panic_payload_message(payload: &(dyn Any + Send)) -> String {
    if let Some(message) = payload.downcast_ref::<&str>() {
        (*message).to_string()
    } else if let Some(message) = payload.downcast_ref::<String>() {
        message.clone()
    } else {
        "<non-string panic payload>".to_string()
    }
}

fn issue_report_target(repository: &str) -> String {
    let repository = repository.trim_end_matches('/');
    if repository.is_empty() {
        "the project issue tracker".to_string()
    } else {
        format!("{repository}/{ISSUE_TEMPLATE}")
    }
}

fn render_crash_report(context: &PanicContext, backtrace: &str) -> String {
    let report_target = issue_report_target(context.repository);
    format!(
        "=== AsterDrive Panic Report ===\n\
         Version:   {}\n\
         Platform:  {}\n\
         Timestamp: {}\n\
         Thread:    {}\n\
         Location:  {}\n\
         Message:   {}\n\
         Report:    {}\n\
         Backtrace:\n{}\n\
         ===============================\n\n",
        context.version,
        context.platform,
        context.timestamp,
        context.thread_name,
        context.location,
        context.message,
        report_target,
        backtrace.trim_end()
    )
}

fn render_user_panic_notice(
    context: &PanicContext,
    crash_log_path: &std::path::Path,
    crash_logged: bool,
) -> String {
    let report_target = issue_report_target(context.repository);
    let diagnostic_line = if crash_logged {
        format!(
            "A diagnostic report was written to {}.",
            crash_log_path.display()
        )
    } else {
        format!(
            "A diagnostic report could not be written to {}.",
            crash_log_path.display()
        )
    };

    format!(
        "AsterDrive encountered an unexpected internal error.\n\
         {diagnostic_line}\n\
         Timestamp: {}\n\
         If the process exits, restart AsterDrive and report the diagnostic report at:\n\
         {report_target}",
        context.timestamp
    )
}

#[cfg(test)]
mod tests {
    use super::{
        PanicContext, issue_report_target, panic_payload_message, render_crash_report,
        render_user_panic_notice,
    };

    fn test_context() -> PanicContext {
        PanicContext {
            version: "0.1.0-test",
            platform: "test-os",
            repository: "https://example.test/asterdrive/",
            timestamp: "2026-05-05 12:34:56.789".to_string(),
            thread_name: "test-thread".to_string(),
            location: "src/main.rs:42:9".to_string(),
            message: "secret panic payload".to_string(),
        }
    }

    #[test]
    fn user_notice_is_short_and_omits_developer_diagnostics() {
        let context = test_context();
        let notice = render_user_panic_notice(
            &context,
            std::path::Path::new("/tmp/asterdrive/crash.log"),
            true,
        );

        assert!(notice.contains("AsterDrive encountered an unexpected internal error."));
        assert!(notice.contains("/tmp/asterdrive/crash.log"));
        assert!(notice.contains("2026-05-05 12:34:56.789"));
        assert!(
            notice.contains("https://example.test/asterdrive/issues/new?template=bug_report.yml")
        );
        assert!(!notice.contains("src/main.rs:42:9"));
        assert!(!notice.contains("secret panic payload"));
        assert!(!notice.contains("Backtrace"));
    }

    #[test]
    fn user_notice_reports_when_crash_log_could_not_be_written() {
        let context = test_context();
        let notice = render_user_panic_notice(&context, std::path::Path::new("crash.log"), false);

        assert!(notice.contains("could not be written"));
        assert!(notice.contains("crash.log"));
    }

    #[test]
    fn crash_report_keeps_developer_diagnostics() {
        let context = test_context();
        let report = render_crash_report(&context, "frame 1\nframe 2\n");

        assert!(report.contains("=== AsterDrive Panic Report ==="));
        assert!(report.contains("Version:   0.1.0-test"));
        assert!(report.contains("Platform:  test-os"));
        assert!(report.contains("Thread:    test-thread"));
        assert!(report.contains("Location:  src/main.rs:42:9"));
        assert!(report.contains("Message:   secret panic payload"));
        assert!(report.contains(
            "Report:    https://example.test/asterdrive/issues/new?template=bug_report.yml"
        ));
        assert!(report.contains("Backtrace:\nframe 1\nframe 2"));
    }

    #[test]
    fn panic_payload_message_handles_common_payload_types() {
        let owned = "owned panic".to_string();

        assert_eq!(panic_payload_message(&"static panic"), "static panic");
        assert_eq!(panic_payload_message(&owned), "owned panic");
        assert_eq!(
            panic_payload_message(&123_i32),
            "<non-string panic payload>"
        );
    }

    #[test]
    fn issue_report_target_tolerates_empty_repository() {
        assert_eq!(
            issue_report_target("https://example.test/project/"),
            "https://example.test/project/issues/new?template=bug_report.yml"
        );
        assert_eq!(issue_report_target(""), "the project issue tracker");
    }
}
