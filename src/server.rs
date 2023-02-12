use std::net::Ipv4Addr;

use anyhow::{anyhow, Result};
use cidr_utils::cidr::Ipv4Cidr;
use log::{debug, trace};

use super::{
    multicast::{Message, MulticastSocket},
    packet::{Name, Packet, ResourceRecord, ResourceRecordData, ResourceType},
    Service,
};

pub struct Server {
    services: Vec<Service>,
    hostname: String,
    // consider only ipv4 for now
    prefixes: Vec<(Ipv4Addr, Ipv4Cidr)>,
}

impl Server {
    pub fn new(services: Vec<Service>) -> Result<Self> {
        let mut hostname = hostname::get()?.into_string().unwrap();
        if !hostname.ends_with(".local") {
            hostname = format!("{}.local", hostname);
        }
        debug!("hostname: {}", hostname);

        let prefixes = if_addrs::get_if_addrs()?
            .into_iter()
            .filter_map(|if_addr| {
                use if_addrs::IfAddr;
                if let IfAddr::V4(addr) = if_addr.addr {
                    if addr.netmask != Ipv4Addr::new(0, 0, 0, 0) && addr.netmask != Ipv4Addr::new(255, 255, 255, 255) {
                        return Some((addr.ip, Ipv4Cidr::from_prefix_and_mask(addr.ip, addr.netmask).ok()?));
                    }
                }

                None
            })
            .collect::<Vec<_>>();

        for prefix in &prefixes {
            debug!("ip {:?}/{}", prefix.0, prefix.1.get_bits());
        }

        Ok(Self {
            services,
            hostname,
            prefixes,
        })
    }

    pub async fn serve(&self) -> Result<()> {
        let mdns_addr = Ipv4Addr::new(224, 0, 0, 251);
        let mut socket = MulticastSocket::new(mdns_addr, 5353).await?;

        loop {
            let message = socket.read().await?;
            trace!("receive from {}, raw {:?}", message.sender, message.data);

            if let Some((unicast_response, multicast_response)) = self.handle_packet(&message) {
                if let Some(unicast_response) = unicast_response {
                    let response = unicast_response.write();

                    trace!("sending response to {:?}, raw {:?}", message.sender, response);

                    socket.write_to(&response, message.interface, &message.sender).await?;
                }

                if let Some(multicast_response) = multicast_response {
                    let response = multicast_response.write();

                    trace!("sending response to {:?}, raw {:?}", message.sender, response);
                    socket.write(&response, message.interface).await?;
                }
            }
        }
    }

    fn handle_packet(&self, message: &Message) -> Option<(Option<Packet>, Option<Packet>)> {
        let packet = Packet::parse(&message.data).ok()?;

        if packet.header.is_query() {
            let mut unicast_response = (Vec::new(), Vec::new());
            let mut multicast_response = (Vec::new(), Vec::new());

            for question in &packet.questions {
                for service in &self.services {
                    if question.r#type == ResourceType::PTR && question.name.equals(&service.r#type) {
                        let (mut answers, mut additionals) = self.create_response(service, message.sender.ip()).ok()?;

                        if question.unicast {
                            unicast_response.0.append(&mut answers);
                            unicast_response.1.append(&mut additionals);
                        } else {
                            multicast_response.0.append(&mut answers);
                            multicast_response.1.append(&mut additionals);
                        }
                    }
                }
            }

            let unicast_response = (!unicast_response.0.is_empty() || !unicast_response.1.is_empty())
                .then(|| Packet::new_response(packet.header.id(), Vec::new(), unicast_response.0, Vec::new(), unicast_response.1));
            let multicast_response = (!multicast_response.0.is_empty() || !multicast_response.1.is_empty())
                .then(|| Packet::new_response(packet.header.id(), Vec::new(), multicast_response.0, Vec::new(), multicast_response.1));

            return Some((unicast_response, multicast_response));
        }

        None
    }

    fn create_response(&self, service: &Service, remote_addr: &Ipv4Addr) -> Result<(Vec<ResourceRecord>, Vec<ResourceRecord>)> {
        debug!("Creating response for {}", service.name);

        let ip = self.find_local_ip(remote_addr).ok_or_else(|| anyhow!("Can't find local ip address"))?;

        // PTR answer
        let answers = vec![ResourceRecord::new(
            &service.r#type,
            3600,
            ResourceRecordData::PTR(Name::new(&service.name)),
        )];

        // SRV record
        let mut additionals = vec![ResourceRecord::new(
            &service.name,
            3600,
            ResourceRecordData::SRV {
                priority: 0,
                weight: 0,
                port: service.port,
                target: Name::new(&self.hostname),
            },
        )];

        // TXT record
        if !service.txt.is_empty() {
            additionals.push(ResourceRecord::new(&service.name, 3600, ResourceRecordData::TXT(service.txt.clone())));
        }

        // A record
        additionals.push(ResourceRecord::new(&self.hostname, 3600, ResourceRecordData::A(ip)));

        Ok((answers, additionals))
    }

    fn find_local_ip(&self, remote_addr: &Ipv4Addr) -> Option<Ipv4Addr> {
        for prefix in &self.prefixes {
            if prefix.1.contains(remote_addr) {
                debug!("remote_addr: {:?}, interface ip: {:?}/{}", remote_addr, prefix.0, prefix.1.get_bits());
                return Some(prefix.0);
            }
        }

        None
    }
}
