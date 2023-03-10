use blaze_pk::{
    codec::{Decodable, Encodable},
    error::DecodeResult,
    reader::TdfReader,
    tag::TdfType,
    writer::TdfWriter,
};
use blaze_pk::{types::Union, value_type};
use std::{
    fmt::{Debug, Display},
    net::Ipv4Addr,
};

/// Packet encoding for Redirector GetServerInstance packets
/// this contains basic information about the client session.
///
/// These details are extracted from an official game copy
pub struct InstanceRequest;

impl Encodable for InstanceRequest {
    fn encode(&self, writer: &mut TdfWriter) {
        writer.tag_str(b"BSDK", "3.15.6.0");
        writer.tag_str(b"BTIM", "Dec 21 2012 12:47:10");
        writer.tag_str(b"CLNT", "MassEffect3-pc");
        writer.tag_u8(b"CLTP", 0);
        writer.tag_str(b"CSKU", "134845");
        writer.tag_str(b"CVER", "05427.124");
        writer.tag_str(b"DSDK", "8.14.7.1");
        writer.tag_str(b"ENV", "prod");
        writer.tag_union_unset(b"FPID");
        writer.tag_u32(b"LOC", 0x656e4e5a);
        writer.tag_str(b"NAME", "masseffect-3-pc");
        writer.tag_str(b"PLAT", "Windows");
        writer.tag_str(b"PROF", "standardSecure_v3");
    }
}

/// Networking information for an instance. Contains the
/// host address and the port
pub struct InstanceNet {
    pub host: InstanceHost,
    pub port: Port,
}

impl From<(String, Port)> for InstanceNet {
    fn from((host, port): (String, Port)) -> Self {
        let host = InstanceHost::from(host);
        Self { host, port }
    }
}

impl Encodable for InstanceNet {
    fn encode(&self, writer: &mut TdfWriter) {
        self.host.encode(writer);
        writer.tag_u16(b"PORT", self.port);
        writer.tag_group_end();
    }
}

impl Decodable for InstanceNet {
    fn decode(reader: &mut TdfReader) -> DecodeResult<Self> {
        let host: InstanceHost = InstanceHost::decode(reader)?;
        let port: u16 = reader.tag("PORT")?;
        reader.read_byte()?;
        Ok(Self { host, port })
    }
}

value_type!(InstanceNet, TdfType::Group);

/// Type of instance details provided either hostname
/// encoded as string or IP address encoded as NetAddress
pub enum InstanceHost {
    Host(String),
    Address(NetAddress),
}

/// Attempts to convert the provided value into a instance type. If
/// the provided value is an IPv4 value then Address is used otherwise
/// Host is used.
impl From<String> for InstanceHost {
    fn from(value: String) -> Self {
        if let Ok(value) = value.parse::<Ipv4Addr>() {
            Self::Address(NetAddress(value))
        } else {
            Self::Host(value)
        }
    }
}

/// Function for converting an instance type into its address
/// string value for use in connections
impl From<InstanceHost> for String {
    fn from(value: InstanceHost) -> Self {
        match value {
            InstanceHost::Address(value) => value.to_string(),
            InstanceHost::Host(value) => value,
        }
    }
}

impl Encodable for InstanceHost {
    fn encode(&self, writer: &mut TdfWriter) {
        match self {
            InstanceHost::Host(value) => writer.tag_str(b"HOST", value),
            InstanceHost::Address(value) => writer.tag_value(b"IP", value),
        }
    }
}

impl Decodable for InstanceHost {
    fn decode(reader: &mut TdfReader) -> DecodeResult<Self> {
        let host: Option<String> = reader.try_tag("HOST")?;
        if let Some(host) = host {
            return Ok(Self::Host(host));
        }
        let ip: NetAddress = reader.tag("IP")?;
        Ok(Self::Address(ip))
    }
}

/// Details about an instance. This is used for the redirector system
/// to both encode for redirections and decode for the retriever system
pub struct InstanceDetails {
    /// The networking information for the instance
    pub net: InstanceNet,
    /// Whether the host requires a secure connection (SSLv3)
    pub secure: bool,
}

impl Encodable for InstanceDetails {
    fn encode(&self, writer: &mut TdfWriter) {
        writer.tag_union_start(b"ADDR", NetworkAddressType::Server.into());
        writer.tag_value(b"VALU", &self.net);

        writer.tag_bool(b"SECU", self.secure);
        writer.tag_bool(b"XDNS", false);
    }
}

impl Decodable for InstanceDetails {
    fn decode(reader: &mut TdfReader) -> DecodeResult<Self> {
        let net: InstanceNet = match reader.tag::<Union<InstanceNet>>("ADDR")? {
            Union::Set { value, .. } => value,
            Union::Unset => {
                return Err(blaze_pk::error::DecodeError::MissingTag {
                    tag: "ADDR".to_string(),
                    ty: TdfType::Union,
                })
            }
        };
        let secure: bool = reader.tag("SECU")?;
        Ok(InstanceDetails { net, secure })
    }
}

#[derive(Debug, Copy, Clone)]
pub enum NetworkAddressType {
    Server,
    Client,
    Pair,
    IpAddress,
    HostnameAddress,
    Unknown(u8),
}

impl NetworkAddressType {
    pub fn value(&self) -> u8 {
        match self {
            Self::Server => 0x0,
            Self::Client => 0x1,
            Self::Pair => 0x2,
            Self::IpAddress => 0x3,
            Self::HostnameAddress => 0x4,
            Self::Unknown(value) => *value,
        }
    }

    pub fn from_value(value: u8) -> Self {
        match value {
            0x0 => Self::Server,
            0x1 => Self::Client,
            0x2 => Self::Pair,
            0x3 => Self::IpAddress,
            0x4 => Self::HostnameAddress,
            value => Self::Unknown(value),
        }
    }
}

impl From<NetworkAddressType> for u8 {
    fn from(value: NetworkAddressType) -> Self {
        value.value()
    }
}

/// Type alias for ports which are always u16
pub type Port = u16;

/// Structure for wrapping a Blaze networking address
#[derive(Copy, Clone, Eq, PartialEq)]
pub struct NetAddress(pub Ipv4Addr);

impl Default for NetAddress {
    fn default() -> Self {
        Self(Ipv4Addr::LOCALHOST)
    }
}

impl Encodable for NetAddress {
    fn encode(&self, writer: &mut TdfWriter) {
        let bytes = self.0.octets();
        let value = u32::from_be_bytes(bytes);
        writer.write_u32(value);
    }
}

impl Decodable for NetAddress {
    fn decode(reader: &mut TdfReader) -> DecodeResult<Self> {
        let value = reader.read_u32()?;
        let bytes = value.to_be_bytes();
        let addr = Ipv4Addr::from(bytes);
        Ok(Self(addr))
    }
}

value_type!(NetAddress, TdfType::VarInt);

/// Debug trait implementation sample implementation as the Display
/// implementation so that is just called instead
impl Debug for NetAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self, f)
    }
}

/// Display trait implementation for NetAddress. If the value is valid
/// the value is translated into the IPv4 representation
impl Display for NetAddress {
    /// Converts the value stored in this NetAddress to an IPv4 string
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
