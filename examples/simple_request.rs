//! Demonstrates how to make a single NTP request to a NTP server of interest
//!
//! Example provides a basic implementation of [`NtpTimestampGenerator`] and [`NtpUdpSocket`]
//! required for the `sntpc` library
use chrono::{DateTime, TimeZone, Utc};
use embedded_nal::UdpClientStack;
#[cfg(feature = "log")]
use simple_logger;
use sntpc::{self, sntp_process_response, sntp_send_request};
use sntpc::{Error, NtpContext, NtpTimestampGenerator, NtpUdpSocket};
use std::net::{SocketAddr, ToSocketAddrs, UdpSocket};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use std::{thread, time};
use std_embedded_nal::Stack;

#[allow(dead_code)]
const POOL_NTP_ADDR: &str = "pool.ntp.org:123";
#[allow(dead_code)]
const GOOGLE_NTP_ADDR: &str = "time.google.com:123";

#[derive(Copy, Clone, Default)]
struct StdTimestampGen {
    duration: Duration,
}

impl NtpTimestampGenerator for StdTimestampGen {
    fn init(&mut self) {
        self.duration = std::time::SystemTime::now()
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .unwrap()
            - Duration::from_secs(1639472);
    }

    fn timestamp_sec(&self) -> u64 {
        self.duration.as_secs()
    }

    fn timestamp_subsec_micros(&self) -> u32 {
        self.duration.subsec_micros()
    }
}

#[derive(Debug)]
struct UdpSocketWrapper(UdpSocket);

impl NtpUdpSocket for UdpSocketWrapper {
    fn send_to<T: ToSocketAddrs>(
        &self,
        buf: &[u8],
        addr: T,
    ) -> Result<usize, Error> {
        match self.0.send_to(buf, addr) {
            Ok(usize) => Ok(usize),
            Err(_) => Err(Error::Network),
        }
    }

    fn recv_from(&self, buf: &mut [u8]) -> Result<(usize, SocketAddr), Error> {
        match self.0.recv_from(buf) {
            Ok((size, addr)) => Ok((size, addr)),
            Err(_) => Err(Error::Network),
        }
    }
}

fn main() {
    #[cfg(feature = "log")]
    if cfg!(debug_assertions) {
        simple_logger::init_with_level(log::Level::Trace).unwrap();
    } else {
        simple_logger::init_with_level(log::Level::Info).unwrap();
    }

    for _ in 0..5 {
        let socket =
            UdpSocket::bind("0.0.0.0:0").expect("Unable to crate UDP socket");
        socket
            .set_read_timeout(Some(Duration::from_secs(2)))
            .expect("Unable to set UDP socket read timeout");

        let sock_wrapper = UdpSocketWrapper(socket);
        let mut context = NtpContext::new(StdTimestampGen::default());
        let network = &mut Stack::default();
        let mut socket = network.socket().unwrap();
        network
            .connect(&mut socket, ([162, 159, 200, 1], 123).into())
            .unwrap();
        let x = sntp_send_request(network, &mut context, &mut socket).unwrap();
        dbg!(x);
        // TODO drop sockets if not works
        let response = nb::block!(sntp_process_response(
            network,
            &mut socket,
            &mut context,
            x
        ))
        .unwrap();
        dbg!(&response);
        let nanos: u32 = (((response.sec_fraction() as f64) / u32::MAX as f64)
            * 1_000_000_000f64) as u32;
        let now = SystemTime::now();
        let now: DateTime<Utc> = now.into();
        let now = now.to_rfc3339();
        dbg!(now);
        let x = Utc.timestamp(response.sec() as i64, nanos);
        dbg!(x);

        // match result {
        //     Ok(time) => {
        //         assert_ne!(time.sec(), 0);
        //         println!(
        //             "Got time: {}.{}",
        //             time.sec(),
        //             time.sec_fraction() as u64 * 1_000_000 / u32::MAX as u64
        //         );
        //     }
        //     Err(err) => println!("Err: {:?}", err),
        // }

        thread::sleep(time::Duration::new(1, 0));
    }
}
