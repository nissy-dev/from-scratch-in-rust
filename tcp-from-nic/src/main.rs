use tracing::info;

mod nic;

fn main() {
    tracing_subscriber::fmt::init();
    let args: Vec<String> = std::env::args().collect();

    match args[1].as_str() {
        "nic" => {
            let nic = nic::NetDevice::new();
            nic.bind();
            loop {
                let packet = nic.read();
                info!("received packet: {:?}", packet);
                nic.write(packet);
            }
        }
        _ => (),
    }
}
