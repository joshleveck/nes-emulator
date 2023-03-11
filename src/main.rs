mod controller;

fn main() {
    println!("Hello, world!");
    let mut controller = controller::Controller::new();
    controller.master_loop();
}
