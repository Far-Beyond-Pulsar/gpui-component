pub struct PhysicsEngine {
    // Physics thread logic here
}

impl PhysicsEngine {
    pub fn new() -> Self {
        PhysicsEngine {}
    }

    pub async fn start(&self) {
        loop {
            std::thread::sleep(std::time::Duration::from_millis(1000));
            println!("Physics engine running...");
        }
    }
}
