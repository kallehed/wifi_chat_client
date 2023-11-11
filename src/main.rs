use std::hash::{BuildHasher, Hasher};
use std::io::{Error, Read, Write};
use std::net::{IpAddr, SocketAddr, TcpListener, TcpStream, UdpSocket};
use std::sync::mpsc::{self, Receiver, SyncSender};
use std::sync::{Arc, Mutex};
use std::time::Duration;

const PORT: u16 = 2000;

// ms til next existance broadcast
const REBROADCAST_TIME: u64 = 1000;

const NAME_SIZE: u64 = 32;

#[derive(Hash, Eq, PartialEq, Debug)]
struct Client(IpAddr, [u8; NAME_SIZE as _]);

impl std::fmt::Display for Client {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Client(")?;
        self.0.fmt(f)?;
        write!(f, ", Name: ")?;
        for a in self.1 {
            write!(f, "{}", a as char)?;
        }
        write!(f, ")")?;
        std::fmt::Result::Ok(())
    }
}

fn listen_and_broadcast_existance(channel: SyncSender<Client>, name: &[u8]) -> Result<(), Error> {
    loop {
        // broadcast existance
        {
            let broadcaster = UdpSocket::bind(("0.0.0.0", PORT))?;
            broadcaster.set_broadcast(true)?;
            broadcaster.send_to(name, ("255.255.255.255", PORT))?;
        }
        // listen for other clients existance
        loop {
            let listen = UdpSocket::bind(("255.255.255.255", PORT))?;
            let mut buf = [0; NAME_SIZE as _];
            listen.set_read_timeout(Some(Duration::from_millis(REBROADCAST_TIME)))?;
            let Ok((_size, sender)) = listen.recv_from(&mut buf) else {
                break; // NO MESSAGE RECEIVED, now we shall broadcast again
            };
            channel // send to channel
                .send(Client(sender.ip(), buf))
                .map_err(|_| std::io::Error::last_os_error())?;
            // found client
            // std::io::stdout().write_all(&buf)?;
            std::thread::sleep(Duration::from_millis(
                std::collections::hash_map::RandomState::new()
                    .build_hasher()
                    .finish() as u64
                    % 100,
            ));
        }
    }
}

// listens on stdin and forwards it through channel
fn stdin_listener(channel: SyncSender<String>) -> Result<(), Error> {
    loop {
        let mut buf = [0; 256];
        println!("waiting for stdin");
        let size = std::io::stdin().read(&mut buf)?;
        println!("got stdin");

        let Ok(str) = std::str::from_utf8(&buf[..size]) else {
            continue;
        };
        channel.send(str.to_string()).unwrap();
    }
}

fn incoming_connection_listener(stdin_receiver: &Mutex<Receiver<String>>) -> Result<(), Error> {
    let listener = TcpListener::bind(("0.0.0.0", PORT))?;

    loop {
        let (stream, _address) = listener.accept()?;
        talk_with_client(stream, stdin_receiver)?;
    }
}

fn talk_with_client(
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
                // client has disconnected
                break 'main_loop;
            }
            print!("got: ");
            std::io::stdout().write_all(&buf[..size])?;
        }
    }
    println!("DISCONNECTED!");
    Ok(())
}

fn main() -> Result<(), Error> {
    let (client_sender, client_receiver) = mpsc::sync_channel(0);
    let _listener = std::thread::spawn(move || {
        listen_and_broadcast_existance(client_sender, b"YOLOSwag").unwrap();
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

        let mut clients = std::collections::HashSet::new();

        loop {
            'block: {
                let Ok(client) = client_receiver.recv_timeout(Duration::from_millis(10)) else {
                    break 'block;
                };
                //println!("found {}", client);
                clients.insert(client);
                // println!("all of them: {:?}", clients);
                println!("Neighbors:");
                for (idx, cl) in clients.iter().enumerate() {
                    println!("{idx}: {cl}");
                }
                println!();
            }
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
                println!("found {}", num);
                let Some(client) = clients.iter().skip(num as _).next() else {
                    eprintln!("USER-ERROR: client `{}` does not exist", num);
                    break 'block;
                };
                println!("Connecting to {}", client);
                let Ok(tcp_stream) = TcpStream::connect_timeout(
                    &SocketAddr::new(client.0, PORT),
                    Duration::from_millis(1000),
                ) else {
                    eprintln!("ERROR: Could not connect to {}", client);
                    break 'block;
                };

                talk_with_client(tcp_stream, &stdin_receiver)?;
            }
        }
    })?;
    Ok(())
}
