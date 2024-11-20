use crate::log_data::{LogRepresentable, LogWriter};

use std::io::Write;

pub struct BufferLogWriter<W> {
    buf: W,
}

impl<W, LRO, LRA> LogWriter<LRO, LRA> for BufferLogWriter<W>
where
    W: Write,
    LRO: LogRepresentable,
    LRA: LogRepresentable,
{
    fn add_log_data(
        &mut self,
        object: LRO,
        action: LRA,
        time: crate::gametime::GameTime,
        duration: crate::gametime::GameTime,
    ) {
        // data goes straight to buffer
        if let Err(e) = writeln!(
            self.buf,
            "{}\t{}\t{}\t{}",
            object.to_log_repr(),
            action.to_log_repr(),
            time,
            duration
        ) {
            eprintln!("error writing log: {}", e);
        };
        if let Err(e) = self.buf.flush() { // flush every line!
            eprintln!("error flushing log: {}", e);
        };
    }
}

impl<W> BufferLogWriter<W>
where
    W: Write,
{
    pub fn new(buffer: W) -> BufferLogWriter<W> {
        BufferLogWriter {
            buf: buffer,
        }
    }
}
