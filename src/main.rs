use std::hash::{BuildHasher, Hasher};
use std::io::{Error, Read, Write};
use std::net::{IpAddr, UdpSocket, TcpListener, TcpStream, SocketAddr, SocketAddrV4};
use std::sync::mpsc::SyncSender;
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

// listens on stdin for a number, which will correspond to a client to connect to
fn stdin_listener(channel: SyncSender<u64>) -> Result<(), Error> {
    loop {
        let stdin = std::io::stdin();

        let mut buf = [0; 256];
        println!("waiting for stdin");
        let size = std::io::stdin().read(&mut buf)?;
        println!("got stdin");

        let Ok(str) = std::str::from_utf8(&buf[..size]) else {
            continue;
        };
        let str = str.trim();
        let Ok(num) = str.parse() else {
            continue;
        };
        channel.send(num).unwrap();
    }
}

fn incoming_connection_listener() -> Result<(), Error> {
    let listener = TcpListener::bind(("0.0.0.0", PORT))?;

    let (stream, _address) = listener.accept()?;
    talk_with_client(stream)?;

    Ok(())
}

fn talk_with_client(mut stream: TcpStream) -> Result<(), Error> {
    println!("CONNECTION CREATED!");

    loop {
        let mut buf = [0; 256];

        std::io::stdin().read(&mut buf)?;
        stream.write_all(&buf)?;
        println!("got: "); 
        stream.read(&mut buf)?;
        std::io::stdout().write_all(&buf)?;
        stream.write_all(&buf)?;

    }
    
    // Ok(())
}

fn main() -> Result<(), Error> {
    let (client_sender, client_reciever) = std::sync::mpsc::sync_channel(0);
    let _listener = std::thread::spawn(move || {
        listen_and_broadcast_existance(client_sender, b"YOLOSwag").unwrap();
    });

    let (stdin_sender, stdin_reciever) = std::sync::mpsc::sync_channel(0);
    let _stdin_listener = std::thread::spawn(move || {
        stdin_listener(stdin_sender).unwrap();
    });

    let _connection_listener = std::thread::spawn(move || {
        incoming_connection_listener().unwrap();
    });

    let mut clients = std::collections::HashSet::new();

    loop {
        'block: {
            let Ok(client) = client_reciever.recv_timeout(Duration::from_millis(10)) else {
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
            let Ok(num) = stdin_reciever.recv_timeout(Duration::from_millis(10)) else {
                break 'block;
            };
            println!("found {}", num);
            let Some(client) = clients.iter().skip(num as _).next() else {
                println!("USER-ERROR: client `{}` does not exist", num); 
                break 'block;
            };
            println!("Connecting to {}", client);
            let Ok(tcp_stream) = TcpStream::connect_timeout(&SocketAddr::new(client.0, PORT), Duration::from_millis(1000)) else {
                println!("ERROR: Could not connect to {}", client);
                break 'block;
            };
            
            talk_with_client(tcp_stream)?;
            loop {}
        }
    }
    //
    //     loop {}
    //
    //     // let my_udp = UdpSocket::bind(("0.0.0.0", PORT))?;
    //
    //     // create two threads, one that listens for UDP and then creates TCP request
    //     // another that sends udp, and listens for TCP request
    //
    //     // broadcast who we are.
    //
    //     // my_udp.set_broadcast(true)?;
    //     // println!("starting to send message!");
    //     // my_udp.send_to(b"UDPtest from PORT", ("255.255.255.255", PORT))?;
    //
    //     let tcp_listener = TcpListener::bind(("0.0.0.0", PORT))?;
    //     let (mut stream, _other_socket) = tcp_listener.accept()?;
    //     let stream_clone = stream.try_clone().unwrap();
    //
    //     // reader
    //     let j1 = std::thread::spawn(move || loop {
    //         let mut buf = [0; 256];
    //         let size = stream.read(&mut buf).unwrap();
    //         if size == 0 {
    //             break;
    //         }
    //         println!("\nnew message recieved with len: {}", size);
    //         std::io::stdout().write_all(&buf[..size]).unwrap();
    //     });
    //     // writer
    //     let mut stream = stream_clone;
    //     let _j2 = std::thread::spawn(move || loop {
    //         let mut buf = [0; 256];
    //         let size = std::io::stdin().read(&mut buf).unwrap();
    //         stream.write_all(&buf[..size]).unwrap();
    //     });
    //
    //     j1.join().unwrap();
    //     Ok(())
    //
    //     //println!("waiting for UDP packet on: {:?}", my_udp);
    //     // let (_size, sender) = my_udp.recv_from(&mut buf)?;
    //     // println!("Received UDP packet withs size: {}, sender {:?}, msg: {:?}", _size, sender, &buf[.._size]);
    //     //
    //     // let stream = TcpStream::connect(sender)?;
    //     //
    //     // talk_to(stream)?;
    //
    //     // let loopback = Ipv4Addr::new(127, 0, 0, 1);
    //     // let socket = SocketAddrV4::new(loopback, 2000);
    //     // let listener = TcpListener::bind(socket)?;
    //     // let port = listener.local_addr()?;
    //     // println!("Listening on {}, access this port to end the program", port);
    //     // loop {
    //     //     let (mut stream, socket) = listener.accept()?;
    //     //     stream.write_all(b"hello guys")?;
    //     //     println!("socket: {}", socket);
    //     //     if socket.is_ipv6() {
    //     //         break;
    //     //     }
    //     //     let _ = std::thread::spawn(move || {
    //     //         let _ = talk_to(stream);
    //     //     });
    //     // }
    //
    //     //let (mut tcp_stream, addr) = listener.accept()?; //block  until requested
    //     // println!("Connection received! {:?} is sending data.", addr);
    //     // let mut input = String::new();
    //     // let _ = tcp_stream.read_to_string(&mut input)?;
    //     // println!("{:?} says {}", addr, input);
    //     // Ok(())
}
