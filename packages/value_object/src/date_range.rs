use chrono::{DateTime, Utc};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DateRange {
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
}

impl DateRange {
    pub fn new(
        from: Option<DateTime<Utc>>,
        to: Option<DateTime<Utc>>,
    ) -> Self {
        Self { from, to }
    }

    pub fn contains(&self, instant: &DateTime<Utc>) -> bool {
        if let Some(from) = self.from {
            if instant < &from {
                return false;
            }
        }

        if let Some(to) = self.to {
            if instant > &to {
                return false;
            }
        }

        true
    }

    pub fn effective_instant(&self) -> DateTime<Utc> {
        self.to.unwrap_or_else(Utc::now)
    }
}
