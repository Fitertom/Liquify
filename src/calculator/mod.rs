// Calculator module - empty for now

pub struct Calculator {
    pub display: String,
}

impl Calculator {
    pub fn new() -> Self {
        Calculator {
            display: String::new(),
        }
    }
}
