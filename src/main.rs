use std::io::{Error, Read, Write};
use std::net::{TcpListener, UdpSocket};

const PORT: u16 = 2000;

fn main() -> Result<(), Error> {
    let my_udp = UdpSocket::bind(("0.0.0.0", PORT))?;
    println!("my udp: {:?}", my_udp);

    my_udp.set_broadcast(true)?;
    println!("starting to send message!");
    my_udp.send_to(b"UDPtest from PORT", ("255.255.255.255", PORT))?;

    let tcp_listener = TcpListener::bind(("0.0.0.0", PORT))?;
    let (mut stream, _other_socket) = tcp_listener.accept()?;
    let stream_clone = stream.try_clone().unwrap();

    // reader
    let j1 = std::thread::spawn(move || loop {
        let mut buf = [0; 256];
        let size = stream.read(&mut buf).unwrap();
        if size == 0 {
            break;
        }
        println!("\nnew message recieved with len: {}", size);
        std::io::stdout().write_all(&buf[..size]).unwrap();
    });
    // writer
    let mut stream = stream_clone;
    let _j2 = std::thread::spawn(move || loop {
        let mut buf = [0; 256];
        let size = std::io::stdin().read(&mut buf).unwrap();
        stream.write_all(&buf[..size]).unwrap();
    });

    j1.join().unwrap();
    Ok(())

    //println!("waiting for UDP packet on: {:?}", my_udp);
    // let (_size, sender) = my_udp.recv_from(&mut buf)?;
    // println!("Received UDP packet withs size: {}, sender {:?}, msg: {:?}", _size, sender, &buf[.._size]);
    //
    // let stream = TcpStream::connect(sender)?;
    //
    // talk_to(stream)?;

    // let loopback = Ipv4Addr::new(127, 0, 0, 1);
    // let socket = SocketAddrV4::new(loopback, 2000);
    // let listener = TcpListener::bind(socket)?;
    // let port = listener.local_addr()?;
    // println!("Listening on {}, access this port to end the program", port);
    // loop {
    //     let (mut stream, socket) = listener.accept()?;
    //     stream.write_all(b"hello guys")?;
    //     println!("socket: {}", socket);
    //     if socket.is_ipv6() {
    //         break;
    //     }
    //     let _ = std::thread::spawn(move || {
    //         let _ = talk_to(stream);
    //     });
    // }

    //let (mut tcp_stream, addr) = listener.accept()?; //block  until requested
    // println!("Connection received! {:?} is sending data.", addr);
    // let mut input = String::new();
    // let _ = tcp_stream.read_to_string(&mut input)?;
    // println!("{:?} says {}", addr, input);
    // Ok(())
}
