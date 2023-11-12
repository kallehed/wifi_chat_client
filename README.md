# wifi_chat_client
Chat locally with people on the same wifi. Exercise in using UDP and TCP basically.

Written in Rust. Uses std::net a lot. 
TcpStream, TcpListener, UdpSocket....

Supports discovery of other participants
allows connecting to them, and sending arbitrary messages to them.
You can also disconnect from a peer by writing `exit` or `quit`.

Also, to make this work you have to allow traffic over the PORT (2000) port. 
