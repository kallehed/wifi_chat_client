use std::hash::{BuildHasher, Hasher};
use std::io::{Error, Read, Write};
use std::net::{IpAddr, SocketAddr, TcpListener, TcpStream, UdpSocket};
use std::sync::mpsc::{self, Receiver, SyncSender};
use std::sync::Mutex;
use std::time::Duration;

const PORT: u16 = 2000;

// ms til next existance broadcast
const REBROADCAST_TIME: u64 = 1500;

const NAME_SIZE: u64 = 32;

#[derive(Hash, Eq, PartialEq, Debug)]
struct Peer(IpAddr, [u8; NAME_SIZE as _]);

fn main() -> Result<(), Error> {
    println!("INSTRUCTIONS:\n when looking at neighbors, write a number to connect to them\n disconnect from peer by writing `exit` or `quit`");
    println!("What is your name? : ");
    let mut buf = [0; 256];
    std::io::stdin().read(&mut buf)?;
    let (peer_sender, peer_receiver) = mpsc::sync_channel(0);
    let _listener = std::thread::spawn(move || {
        listen_and_broadcast_existance(peer_sender, &buf).unwrap();
    });

    let (stdin_sender, stdin_receiver) = mpsc::sync_channel(0);
    let stdin_receiver = Mutex::new(stdin_receiver);
    std::thread::scope(|s| -> Result<(), Error> {
        let _stdin_listener = s.spawn(move || {
            stdin_listener(stdin_sender).unwrap();
        });

        let _connection_listener = s.spawn(|| {
            incoming_connection_listener(&stdin_receiver).unwrap();
        });

        let mut peers = std::collections::HashSet::new();

        loop {
            // print neighbors block
            'block: {
                let Ok(peer) = peer_receiver.recv_timeout(Duration::from_millis(10)) else {
                    break 'block;
                };
                //println!("found {}", peer);
                peers.insert(peer);
                // println!("all of them: {:?}", peers);
                println!("Neighbors:");
                for (idx, cl) in peers.iter().enumerate() {
                    println!("{idx}: {cl}");
                }
                println!();
            }
            // connect to other user block
            'block: {
                let Ok(input) = stdin_receiver
                    .lock()
                    .unwrap()
                    .recv_timeout(Duration::from_millis(10))
                else {
                    break 'block;
                };
                let input = input.trim(); // parse input
                let Ok(num) = input.parse::<u64>() else {
                    eprintln!("USER-ERROR: not a number: `{input}`");
                    break 'block;
                };
                let Some(peer) = peers.iter().skip(num as _).next() else {
                    eprintln!("USER-ERROR: peer `{}` does not exist", num);
                    break 'block;
                };
                println!("Connecting to {}", peer);
                let Ok(tcp_stream) = TcpStream::connect_timeout(
                    &SocketAddr::new(peer.0, PORT),
                    Duration::from_millis(1000),
                ) else {
                    eprintln!("ERROR: Could not connect to {}", peer);
                    break 'block;
                };

                talk_with_peer(tcp_stream, &stdin_receiver)?;
            }
        }
    })?;
    Ok(())
}

/// read from stdin -> write to peer
/// read from tcp_stream -> write to stdout
/// take turns with each.
fn talk_with_peer(
    mut stream: TcpStream,
    stdin_receiver: &Mutex<Receiver<String>>,
) -> Result<(), Error> {
    let stdin_receiver = stdin_receiver.lock().unwrap();
    println!("CONNECTION CREATED!");
    stream.set_read_timeout(Some(Duration::from_millis(10)))?;
    'main_loop: loop {
        'block: {
            let Ok(input) = stdin_receiver.recv_timeout(Duration::from_millis(10)) else {
                break 'block;
            };
            if input.trim() == "exit" || input.trim() == "quit" {
                break 'main_loop;
            }
            stream.write_all(input.as_str().as_bytes())?;
        }
        'block: {
            let mut buf = [0; 256];
            let Ok(size) = stream.read(&mut buf) else {
                break 'block;
            };
            if size == 0 {
                // peer has disconnected
                break 'main_loop;
            }
            print!("got: ");
            std::io::stdout().write_all(&buf[..size])?;
        }
    }
    println!("DISCONNECTED!");
    Ok(())
}

/// take turns broadcasting existance through udp
/// + reading from udp broadcast to see if anyone
/// is here.
/// Send found peers down a mpsc channel
fn listen_and_broadcast_existance(channel: SyncSender<Peer>, name: &[u8]) -> Result<(), Error> {
    loop {
        // broadcast existance
        {
            let broadcaster = UdpSocket::bind(("0.0.0.0", PORT))?;
            broadcaster.set_broadcast(true)?;
            broadcaster.send_to(name, ("255.255.255.255", PORT))?;
        }
        // listen for other peers existance
        'block: {
            let listen = UdpSocket::bind(("255.255.255.255", PORT))?;
            let mut buf = [0; NAME_SIZE as _];
            listen.set_read_timeout(Some(Duration::from_millis(
                REBROADCAST_TIME
                    + std::collections::hash_map::RandomState::new()
                        .build_hasher()
                        .finish() as u64
                        % 200,
            )))?;
            let Ok((_size, sender)) = listen.recv_from(&mut buf) else {
                break 'block; // NO MESSAGE RECEIVED, now we shall broadcast again
            };
            channel.send(Peer(sender.ip(), buf)).unwrap(); // send to channel
            std::thread::sleep(Duration::from_millis(500));
        }
    }
}

/// listens on stdin and forwards it through mpsc channel
fn stdin_listener(channel: SyncSender<String>) -> Result<(), Error> {
    loop {
        let mut buf = [0; 256];
        let size = std::io::stdin().read(&mut buf)?;

        let Ok(str) = std::str::from_utf8(&buf[..size]) else {
            continue;
        };
        channel.send(str.to_string()).unwrap();
    }
}

/// listens for anyone trying to connect to us through tcp
/// subsequently we start a chat
fn incoming_connection_listener(stdin_receiver: &Mutex<Receiver<String>>) -> Result<(), Error> {
    let listener = TcpListener::bind(("0.0.0.0", PORT))?;

    loop {
        let (stream, _address) = listener.accept()?;
        talk_with_peer(stream, stdin_receiver)?;
    }
}

/// pretty print Peer
impl std::fmt::Display for Peer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Peer(")?;
        self.0.fmt(f)?;
        write!(f, ", Name: ")?;
        for a in self.1 {
            write!(f, "{}", a as char)?;
        }
        write!(f, ")")?;
        std::fmt::Result::Ok(())
    }
}
