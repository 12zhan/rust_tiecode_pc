use std::panic;
use std::fs::OpenOptions;
use std::io::Write;
use std::process::Command;
use log::error;

pub fn init() {
    panic::set_hook(Box::new(|info| {
        let location = info.location().unwrap();
        let msg = match info.payload().downcast_ref::<&'static str>() {
            Some(s) => *s,
            None => match info.payload().downcast_ref::<String>() {
                Some(s) => &s[..],
                None => "Box<Any>",
            },
        };

        let error_msg = format!("Application Panic at {}: {}", location, msg);
        error!("{}", error_msg);
        eprintln!("{}", error_msg);

        // Write to crash.log (append mode)
        if let Ok(mut file) = OpenOptions::new().create(true).append(true).open("crash.log") {
            let _ = writeln!(file, "----------------------------------------");
            let _ = writeln!(file, "Timestamp: {:?}", std::time::SystemTime::now());
            let _ = writeln!(file, "{}", error_msg);
            // Capture backtrace if possible
            let backtrace = std::backtrace::Backtrace::capture();
            if backtrace.status() == std::backtrace::BacktraceStatus::Captured {
                 let _ = writeln!(file, "Backtrace:\n{}", backtrace);
            }
        }

        // Show Message Box on Windows
        #[cfg(target_os = "windows")]
        {
            // Limit message length to avoid command line issues
            let display_msg = if error_msg.len() > 500 {
                format!("{}... (Check crash.log for details)", &error_msg[..500])
            } else {
                error_msg.clone()
            };
            
            // Escape quotes for PowerShell
            let safe_msg = display_msg.replace("\"", "`\"").replace("'", "''");
            
            // Use specific command to show message box. 
            // Using Start-Process to ensure it detaches or runs properly even if parent is crashing? 
            // No, panic hook blocks unwinding, so we can run synchronous command.
            let _ = Command::new("powershell")
                .arg("-NoProfile")
                .arg("-Command")
                .arg(format!(
                    "Add-Type -AssemblyName System.Windows.Forms; [System.Windows.Forms.MessageBox]::Show('{}', 'TieCode Crash', 'OK', 'Error')", 
                    safe_msg
                ))
                .status();
        }
    }));
}
