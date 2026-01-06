pub struct Time {
    hour: u16,
    minute: u16,
}

impl Time {
    pub fn to_minutes(&self) -> u16 {
        self.minute * 60 + self.hour
    }
}

pub struct Availability {
    pub from: Time,
    pub to: Time,
}

pub struct Aircraft {
    pub id: String,
    pub disruptions: Vec<Availability>
}