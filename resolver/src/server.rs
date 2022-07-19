use std::sync::Arc;

use anyhow::Result;
use tokio::net::UdpSocket;
use tracing::{error, info};

use protocol::byte_packet_buffer::BytePacketBuffer;
use protocol::packet::{Packet, Question};
use protocol::ser::Serialize;

use crate::resolver::Resolver;

pub struct Listener {
    // Reference to a bind UDP socket.
    //
    // This socket is the entry point for the
    // DNS resolver. Every request comes from this
    // socket and the socket might be cloned to be able
    // to spawn DNS resolver handlers.
    pub(crate) socket: Arc<UdpSocket>,

    pub(crate) resolver: Arc<Resolver>,
}

impl Listener {
    /// Run the listener to accept DNS requests.
    ///
    /// The listener creates handlers for every request.
    /// Each handler has to deal with the request, and perform
    /// the request resolution on its own.
    pub async fn run(&self) -> Result<()> {
        info!("accepting dns packets");

        loop {
            // Prepare a buffer which can accept only 512 bytes.
            //
            // In DNS protocol, 512 bytes is the maximum length,
            // if the length is bigger that this number, the rfc
            // suggests to use TCP along with the truncated DNS
            // header attributes.
            let mut buffer = [0u8; 512];

            // Clone the socket to have a safe reference to the handler.
            //
            // Cloning the reference allows the use of connect on
            // the socket to be able to send directly frames to the
            // client who initiates the DNS request.
            let socket = self.socket.clone();
            let (len, src) = socket.recv_from(&mut buffer).await?;
            socket.connect(src).await?;

            let handler = Handler {
                socket,
                resolver: self.resolver.clone(),
                request_data: (&buffer[..len]).to_vec(),
            };

            tokio::spawn(async move {
                if let Err(err) = handler.run().await {
                    error!(cause = ?err, "handler error")
                }
            });
        }
    }
}

struct Handler {
    socket: Arc<UdpSocket>,
    resolver: Arc<Resolver>,
    request_data: Vec<u8>,
}

impl Handler {
    async fn run(&self) -> Result<()> {
        // Parse the input raw data into a valid DNS packet.
        //
        // TODO: Handle error when available.
        let mut buffer = BytePacketBuffer::from_raw_data(self.request_data.as_slice());
        let mut request = Packet::from_buffer(&mut buffer);

        // Create an empty response to prepare the request answer.
        //
        // The response is empty if there is not question in the request
        // or if the resolver doesn't manage to get an answer.
        let mut response = Packet::new();

        // Taking the first question and resolve it.
        //
        // It overwrite the response in case of success.
        // TODO: Maybe considering looping over all the questions in the future.
        if let Some(question) = request.questions.pop() {
            let Question { name, qtype, .. } = question;
            response = self.resolver.resolve(name, qtype, request.header.recursion_desired)?;
        }

        // Re-overwriting the response header if it successfully found an answer.
        response.header.id = request.header.id;
        response.header.recursion_desired = request.header.recursion_desired;
        response.header.recursion_available = self.resolver.recursive;
        response.header.is_response = true;

        // Send back the response to the requester.
        let mut buffer = BytePacketBuffer::new();
        response.serialize(&mut buffer)?;
        self.socket.send(&buffer.bytes()).await?;

        Ok(())
    }
}