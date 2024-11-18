use std::net::Ipv4Addr;

use tracing::debug;

mod address;
mod arp;
mod ethernet;
mod net;

fn main() {
    tracing_subscriber::fmt::init();
    let args: Vec<String> = std::env::args().collect();

    match args[1].as_str() {
        "arp" => {
            debug!("arp test...");
            debug!(
                "src_network_interface_name={}, dst_ip_address={}",
                &args[2], &args[3]
            );
            let src_net_interface =
                net::NetworkInterface::new(&args[2]).expect("failed to find the network interface");
            debug!("src_network_interface: {:?}", &src_net_interface);
            let dst_ip_addr = args[3].parse::<Ipv4Addr>().expect("invalid ip address");
            let reply =
                arp::Arp::send(dst_ip_addr, src_net_interface).expect("missing reply arp frame");
            debug!("reply arp frame: {:?}", &reply);
        }
        _ => (),
    }
}
