use crate::timing;
use serde::Serialize;
use std::cell::RefCell;
use std::fmt;
use std::mem;
use std::time::Duration;

thread_local! {
    static OUTPUT_TIME: RefCell<Duration> = RefCell::new(Duration::default());
}

pub(crate) fn record_output_time(duration: Duration) {
    OUTPUT_TIME.with(|rc| *rc.borrow_mut() = duration);
}

pub fn take_output_time() -> Duration {
    OUTPUT_TIME.with(|rc| mem::replace(&mut *rc.borrow_mut(), Duration::default()))
}

#[derive(Serialize)]
pub struct TimingInfo {
    pass_times: Vec<String>,
}

impl TimingInfo {
    pub fn collect() -> Self {
        // `cranelift_codegen::timing::PassTimes` has hidden members at the moment
        // so the best we can do consistently without deep sins like transmutes is to just split
        // some strings.
        let mut pass_times: Vec<String> = vec![];
        let cranelift_time_text = cranelift_codegen::timing::take_current().to_string();
        // skip the header text from Cranelift's `Display`, then take until we hit the end (also
        // "======= ======= ==...")
        for pass in cranelift_time_text
            .split("\n")
            .skip(3)
            .take_while(|line| !line.starts_with("========"))
        {
            pass_times.push(pass.to_string());
        }

        // and now add our own recording of how long it took to write output
        let output_time = timing::take_output_time();
        if output_time != Duration::default() {
            // Round to nearest ms by adding 500us (copied from cranelift-codegen)
            let output_time = output_time + Duration::new(0, 500_000);
            let ms = output_time.subsec_millis();
            let secs = output_time.as_secs();
            // match cranelift pass timing format
            pass_times.push(format!(
                "{:4}.{:03} {:4}.{:03}  Emit output",
                secs, ms, secs, ms
            ));
        }

        Self { pass_times }
    }
}

impl fmt::Display for TimingInfo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for pass in self.pass_times.iter() {
            writeln!(f, "{}", pass)?;
        }

        Ok(())
    }
}
