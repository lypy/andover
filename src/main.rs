extern crate serde_json;
extern crate pnetlink;
use std::fs::File;
use std::env;
use std::io::{self, Read, Write};
use std::os::unix::io::AsRawFd;
use serde_json::Value;
use std::path::PathBuf;

use pnetlink::packet::netlink::NetlinkConnection;
use pnetlink::packet::route::link::Links;


use pnetlink::packet::netlink::{NLM_F_ACK, NLM_F_EXCL, NLM_F_CREATE};
use pnetlink::packet::route::link::{RTM_NEWLINK, IFLA_LINKINFO, IFLA_NET_NS_FD,
    IFLA_IFNAME, IFLA_INFO_DATA, UP, IFLA_INFO_KIND, IfInfoPacketBuilder};
use pnetlink::packet::netlink::{NetlinkReader, NetlinkRequestBuilder};

use pnetlink::packet::route::RtAttrPacket;
use pnetlink::packet::route::route::WithPayload;
use pnetlink::packet::route::route::Nested;

const VETH_INFO_PEER: u16 = 1;

enum CniError {
    VethVarError(std::env::VarError),
    VethJsonError(serde_json::Error),
    VethIoError(std::io::Error)
}

impl From<serde_json::Error> for CniError {
    fn from(error: serde_json::Error) -> Self {
        CniError::VethJsonError(error)
    }
}

impl From<std::io::Error> for CniError {
    fn from(error: std::io::Error) -> Self {
        CniError::VethIoError(error)
    }
}

impl From<std::env::VarError> for CniError {
    fn from(error: std::env::VarError) -> Self {
        CniError::VethVarError(error)
    }
}

fn main() {
    match create_veth() {
        Ok(_) => print!("{}", "{ \"cniVersion\": \"0.3.0\" }"),
        Err(_) => eprint!("error")
    }
}

fn create_veth() -> Result<(), CniError> {
    let command = env::var("CNI_COMMAND")?;

    if command != "ADD" {
        return Ok(());
    }

    let net_ns = env::var("CNI_NETNS")?;
    let if_name = env::var("CNI_IFNAME")?;

    let mut buffer = String::new();
    io::stdin().read_to_string(&mut buffer)?;

    let v: Value = serde_json::from_str(&buffer)?;
    let h_veth = str::replace(&v["andover"].to_string(), "\"", "");

    let p = PathBuf::from(net_ns);

    let fd = File::open(&p)?;

    let mut conn = NetlinkConnection::new();
    conn.new_veth_pair_link(&if_name, &h_veth, fd.as_raw_fd() as u32)?;
    let link = conn.get_link_by_name(h_veth.as_str())?;
    conn.link_set_up(link.unwrap().get_index())?;
    Ok(())

}

pub trait Veth where Self: Read + Write {
    fn new_veth_pair_link(&mut self, name: &str, hname: &str, fd: u32) -> io::Result<()>;
}

impl Veth for NetlinkConnection {
    fn new_veth_pair_link(&mut self, name: &str, hname: &str, fd: u32) -> io::Result<()> {
        let pifi = {
            IfInfoPacketBuilder::new().set_family(0 /* AF_UNSPEC */).build()
        };
        let n1 = Nested::InfoAttr(pifi);
        let n2 = Nested::RtAttr(RtAttrPacket::create_with_payload(IFLA_IFNAME, hname));

        let ifi = {
            let info_kind = RtAttrPacket::create_with_payload(IFLA_INFO_KIND, &b"veth"[..]);
            let mut link_info_data : Vec<&Nested> = Vec::new();
            link_info_data.push(&n1);
            link_info_data.push(&n2);

            let info_peer = RtAttrPacket::create_with_payload(VETH_INFO_PEER, &link_info_data[..]);
            let info_data = RtAttrPacket::create_with_payload(IFLA_INFO_DATA, info_peer);
            let mut link_info = Vec::new();
            link_info.push(&info_kind);
            link_info.push(&info_data);

            IfInfoPacketBuilder::new().
                    set_family(0 /* AF_UNSPEC */).
                    set_flags(UP).
                append(RtAttrPacket::create_with_payload(IFLA_IFNAME, name)).
                append(RtAttrPacket::create_with_payload(IFLA_NET_NS_FD, fd)).
                append(RtAttrPacket::create_with_payload(
                    IFLA_LINKINFO, &link_info[..])).
                build()
        };

        let req = NetlinkRequestBuilder::new(RTM_NEWLINK, NLM_F_CREATE | NLM_F_EXCL | NLM_F_ACK)
            .append(ifi).build();
        self.send(req);
        let reader = NetlinkReader::new(self);
        reader.read_to_end()
    }
}
