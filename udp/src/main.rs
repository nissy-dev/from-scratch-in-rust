use std::net::Ipv4Addr;
use tracing::info;

mod address;
mod arp;
mod checksum;
mod ethernet;
mod icmp;
mod ip;
mod net;
mod socket;
mod udp;

fn main() {
    tracing_subscriber::fmt::init();
    let args: Vec<String> = std::env::args().collect();

    match args[1].as_str() {
        "arp" => {
            info!("arp test...");
            info!(
                "src_network_interface_name='{}', dst_ip_address='{}'",
                &args[2], &args[3]
            );
            let src_net_interface =
                net::NetworkInterface::new(&args[2]).expect("failed to find the network interface");
            info!("{:?}", &src_net_interface);
            let dst_ip_addr = args[3].parse::<Ipv4Addr>().expect("invalid ip address");
            let reply =
                arp::Arp::send(dst_ip_addr, &src_net_interface).expect("missing reply arp frame");
            info!("reply arp frame: {:?}", &reply);
            info!("arp test is successful!!")
        }
        "icmp" => {
            info!("icmp ping test...");
            info!(
                "src_network_interface_name='{}', dst_ip_address='{}'",
                &args[2], &args[3]
            );
            let src_net_interface =
                net::NetworkInterface::new(&args[2]).expect("failed to find the network interface");
            let dst_ip_addr = args[3].parse::<Ipv4Addr>().expect("invalid ip address");
            let reply =
                arp::Arp::send(dst_ip_addr, &src_net_interface).expect("missing reply arp frame");
            let reply = icmp::Icmp::send(dst_ip_addr, reply.sender_mac_addr, &src_net_interface)
                .expect("missing reply icmp frame");
            info!("reply icmp frame: {:?}", &reply);
            info!("icmp ping test is successful!!")
        }
        "udp" => {
            info!("udp test...");
            info!(
                "src_network_interface_name='{}', dst_ip_address='{}'",
                &args[2], &args[3]
            );
            let src_net_interface =
                net::NetworkInterface::new(&args[2]).expect("failed to find the network interface");
            let dst_ip_addr = args[3].parse::<Ipv4Addr>().expect("invalid ip address");
            let dst_port = args[4].parse::<u16>().expect("invalid port number");
            let reply =
                arp::Arp::send(dst_ip_addr, &src_net_interface).expect("missing reply arp frame");
            udp::Udp::send(
                dst_port,
                dst_ip_addr,
                reply.sender_mac_addr,
                &src_net_interface,
            );
            info!("udp test is successful!!")
        }
        _ => {}
    }
}
