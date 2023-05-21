use std::collections::HashMap;
use std::io::Read;
use std::net::{Ipv4Addr, UdpSocket};
use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

use http::header::HeaderValue;

// https://sites.google.com/a/dial-multiscreen.org/dial/dial-protocol-specification
// Used Version: DIAL-2ndScreenProtocol-2.2.1.pdf

const ROOT_DEVICE_UUID: &str = "170ba466-59ac-4039-a457-0fab725b60ff";

fn parse_request_to_string(request: http::request::Builder) -> String {
    let mut result: String = String::new();
    result.push_str(&format!(
        "{} {} {}\r\n",
        request.method_ref().unwrap().to_string(),
        request.uri_ref().unwrap().to_string(),
        match request.version_ref().unwrap() {
            &http::Version::HTTP_09 => "HTTP/0.9",
            &http::Version::HTTP_10 => "HTTP/1.0",
            &http::Version::HTTP_11 => "HTTP/1.1",
            &http::Version::HTTP_2 => "HTTP/2.0",
            &http::Version::HTTP_3 => "HTTP/3.0",
            _ => "HTTP/1.1",
        },
    ));
    request
        .headers_ref()
        .unwrap()
        .iter()
        .for_each(|(key, value)| {
            result.push_str(&format!(
                "{}: {}\r\n",
                key,
                value.to_str().expect("cant convert values to string")
            ))
        });
    result.push_str("\r\n");
    result
}

async fn broadcast_root_device_to_network(
    socket: &UdpSocket,
    root_device_url: &str,
) -> tokio::io::Result<()> {
    // http://www.upnp.org/specs/arch/UPnP-arch-DeviceArchitecture-v1.0.pdf
    // see http://www.upnp.org/specs/basic/UPnP-basic-Basic-v1-Device.pdf
    /*
    NOTIFY * HTTP/1.1
    HOST: 239.255.255.250:1900
    CACHE-CONTROL: max-age = seconds until advertisement expires
    LOCATION: URL for UPnP description for root device
    NT: search target
    NTS: ssdp:alive
    SERVER: OS/version UPnP/1.0 product/version
    USN: advertisement UUID
    */
    // 3 messages for root device
    // Message 1: NT: upnp:rootdevice  ->USN:  uuid:device-UUID::upnp:rootdevice
    let uuid_nt = "upnp:rootdevice";
    let uuid_usn = format!("uuid::{}::{}", ROOT_DEVICE_UUID, "upnp:rootdevice").to_string();
    let request1 = http::Request::builder()
        .method("NOTIFY")
        .uri("*")
        .version(http::Version::HTTP_11)
        .header("HOST", HeaderValue::from_static("239.255.255.250:1900"))
        .header("cache-control", HeaderValue::from_static("max-age = 900"))
        .header(
            "LOCATION",
            HeaderValue::from_str(&root_device_url).expect("Invalid url"),
        )
        .header(
            "NT",
            HeaderValue::from_str(&uuid_nt).expect("This should never be invalid utf-8"),
        )
        .header(
            "USN",
            HeaderValue::from_str(&uuid_usn).expect("This should never be invalid utf-8"),
        )
        .header("NTS", HeaderValue::from_static("ssdp:alive"))
        .header(
            "SERVER",
            HeaderValue::from_static("Linus/Arch UPnP/1.0 Linus_Listener/1.0"),
        );

    // Message 2: NT: uuid:device-UUID   ->USN: uuid:device-UUID (for root device UUID)
    let uuid_nt = format!("uuid::{}", ROOT_DEVICE_UUID).to_string();
    let uuid_usn = format!("uuid::{}", ROOT_DEVICE_UUID).to_string();
    let request2 = http::Request::builder()
        .method("NOTIFY")
        .uri("*")
        .version(http::Version::HTTP_11)
        .header("HOST", HeaderValue::from_static("239.255.255.250:1900"))
        .header("cache-control", HeaderValue::from_static("max-age = 900"))
        .header(
            "LOCATION",
            HeaderValue::from_str(&root_device_url).expect("Invalid url"),
        )
        .header(
            "NT",
            HeaderValue::from_str(&uuid_nt).expect("This should never be invalid utf-8"),
        )
        .header("NTS", HeaderValue::from_static("ssdp:alive"))
        .header(
            "SERVER",
            HeaderValue::from_static("Linus/Arch UPnP/1.0 Linus_Listener/1.0"),
        )
        .header(
            "USN",
            HeaderValue::from_str(&uuid_usn).expect("This should never be invalid utf-8"),
        );

    // Message 3: NT: uuid:device-UUID   ->USN: uuid:device-UUID (for root device UUID)
    /*
       NR :
       urn:schemas-upnp-org:device:deviceType:v or
       urn:domain-name:device:deviceType:v

       USN:
       uuid:device-UUID::urn:schemas-upnp-org:device:deviceType:v (of root device) or
       uuid:device-UUID::urn:domain-name:device:deviceType:v
    */
    let uuid_nt = format!("urn:schemas-upnp-org:device:{}:{}", "Basic", "1").to_string();
    let uuid_usn = format!(
        "uuid:{}::urn:schemas-upnp-org:device:{}:{}",
        ROOT_DEVICE_UUID, "Basic", "1"
    )
    .to_string();
    let request3 = http::Request::builder()
        .method("NOTIFY")
        .uri("*")
        .version(http::Version::HTTP_11)
        .header("HOST", HeaderValue::from_static("239.255.255.250:1900"))
        .header("cache-control", HeaderValue::from_static("max-age = 900"))
        .header(
            "LOCATION",
            HeaderValue::from_str(&root_device_url).expect("Invalid url"),
        )
        .header(
            "NT",
            HeaderValue::from_str(&uuid_nt).expect("This should never be invalid utf-8"),
        )
        .header("NTS", HeaderValue::from_static("ssdp:alive"))
        .header(
            "SERVER",
            HeaderValue::from_static("Linus/Arch UPnP/1.0 Linus_Listener/1.0"),
        )
        //uuid:device-UUID::upnp:rootdevice
        .header(
            "USN",
            HeaderValue::from_str(&uuid_usn).expect("This should never be invalid utf-8"),
        );

    println!("Sending broadcast messages");
    socket.send(parse_request_to_string(request1).as_bytes())?;
    println!("Sent message 1");
    tokio::time::sleep(Duration::from_millis(100)).await;
    socket.send(parse_request_to_string(request2).as_bytes())?;
    println!("Sent message 2");
    tokio::time::sleep(Duration::from_millis(100)).await;
    socket.send(parse_request_to_string(request3).as_bytes())?;
    println!("Sent message 3");
    Ok(())
}

async fn broadcast_device_to_network(
    socket: &UdpSocket,
    root_device_url: &str,
) -> tokio::io::Result<()> {
    /*
    NOTIFY * HTTP/1.1
    USN: uuid:aadda81b-614f-3719-b247-c7545f302b6d::urn:dial-multiscreen-org:device:dial:1
    CACHE-CONTROL: max-age=1800
    NT: urn:dial-multiscreen-org:device:dial:1
    HOST: 239.255.255.250:1900
    LOCATION: http://192.168.178.38:60000/upnp/dev/aadda81b-614f-3719-b247-c7545f302b6d/desc
    SERVER: Linux/4.4.120 UPnP/1.0 Cling/2.0
    NTS: ssdp:alive
    */

    // 2 messages for each embedded device
    // NT: uuid:device-UUID -> USN: uuid:device-UUID
    // Message 2: NT: uuid:device-UUID   ->USN: uuid:device-UUID (for root device UUID)
    let uuid_nt = format!("uuid::{}", ROOT_DEVICE_UUID).to_string();
    let uuid_usn = format!("uuid::{}", ROOT_DEVICE_UUID).to_string();
    let request1 = http::Request::builder()
        .method("NOTIFY")
        .uri("*")
        .version(http::Version::HTTP_11)
        .header("HOST", HeaderValue::from_static("239.255.255.250:1900"))
        .header("cache-control", HeaderValue::from_static("max-age = 900"))
        .header(
            "LOCATION",
            HeaderValue::from_str(&root_device_url).expect("Invalid url"),
        )
        .header(
            "NT",
            HeaderValue::from_str(&uuid_nt).expect("This should never be invalid utf-8"),
        )
        .header("NTS", HeaderValue::from_static("ssdp:alive"))
        .header(
            "SERVER",
            HeaderValue::from_static("Linus/Arch UPnP/1.0 Linus_Listener/1.0"),
        )
        //uuid:device-UUID::upnp:rootdevice
        .header(
            "USN",
            HeaderValue::from_str(&uuid_usn).expect("This should never be invalid utf-8"),
        );

    // Message 3: NT: uuid:device-UUID   ->USN: uuid:device-UUID (for root device UUID)
    /*
       NR :
       urn:schemas-upnp-org:device:deviceType:v or
       urn:domain-name:device:deviceType:v

       USN:
       uuid:device-UUID::urn:schemas-upnp-org:device:deviceType:v (of root device) or
       uuid:device-UUID::urn:domain-name:device:deviceType:v
    */
    let uuid_nt = format!("urn:schemas-upnp-org:device:{}:{}", "Basic", "1").to_string();
    let uuid_usn = format!(
        "uuid:{}::urn:schemas-upnp-org:device:{}:{}",
        ROOT_DEVICE_UUID, "Basic", "1"
    )
    .to_string();
    let request2 = http::Request::builder()
        .method("NOTIFY")
        .uri("*")
        .version(http::Version::HTTP_11)
        .header("HOST", HeaderValue::from_static("239.255.255.250:1900"))
        .header("cache-control", HeaderValue::from_static("max-age = 900"))
        .header(
            "LOCATION",
            HeaderValue::from_str(&root_device_url).expect("Invalid url"),
        )
        .header(
            "NT",
            HeaderValue::from_str(&uuid_nt).expect("This should never be invalid utf-8"),
        )
        .header("NTS", HeaderValue::from_static("ssdp:alive"))
        .header(
            "SERVER",
            HeaderValue::from_static("Linus/Arch UPnP/1.0 Linus_Listener/1.0"),
        )
        .header(
            "USN",
            HeaderValue::from_str(&uuid_usn).expect("This should never be invalid utf-8"),
        );

    println!("Sending device messages");
    socket.send(parse_request_to_string(request1).as_bytes())?;
    println!("Sent message 1");
    tokio::time::sleep(Duration::from_millis(100)).await;
    socket.send(parse_request_to_string(request2).as_bytes())?;
    println!("Sent message 2");
    Ok(())
}

async fn broadcast_service_type_to_network(
    socket: &UdpSocket,
    root_device_url: &str,
) -> tokio::io::Result<()> {
    /*
    Probably need the following services:
    RenderingControl: http://upnp.org/specs/av/UPnP-av-RenderingControl-v1-Service.pdf
    ConnectionManager: http://upnp.org/specs/av/UPnP-av-ConnectionManager-v1-Service.pdf
    AVTransport: http://upnp.org/specs/av/UPnP-av-AVTransport-v1-Service.pdf
    */

    // Message 1:
    /*
    NOTIFY * HTTP/1.1
    HOST: 239.255.255.250:1900
    CACHE-CONTROL: max-age=1800
    LOCATION: http://192.168.178.35:8080/MediaRenderer/desc.xml
    NT: urn:schemas-upnp-org:service:RenderingControl:1
    NTS: ssdp:alive
    SERVER: KnOS/3.2 UPnP/1.0 DMP/3.5
    USN: uuid:5f9ec1b3-ed59-1900-4530-00a0dea81946::urn:schemas-upnp-org:service:RenderingControl:1
        */
    let uuid_nt = format!(
        "urn:schemas-upnp-org:service:{}:{}",
        "RenderingControl", "1"
    )
    .to_string();
    let uuid_usn = format!(
        "uuid:{}::urn:schemas-upnp-org:service:{}:{}",
        ROOT_DEVICE_UUID, "RenderingControl", "1"
    )
    .to_string();
    let request1 = http::Request::builder()
        .method("NOTIFY")
        .uri("*")
        .version(http::Version::HTTP_11)
        .header("HOST", HeaderValue::from_static("239.255.255.250:1900"))
        .header("cache-control", HeaderValue::from_static("max-age = 900"))
        .header(
            "LOCATION",
            HeaderValue::from_str(&root_device_url).expect("Invalid url"),
        )
        .header(
            "NT",
            HeaderValue::from_str(&uuid_nt).expect("This should never be invalid utf-8"),
        )
        .header("NTS", HeaderValue::from_static("ssdp:alive"))
        .header(
            "SERVER",
            HeaderValue::from_static("Linus/Arch UPnP/1.0 Linus_Listener/1.0"),
        )
        .header(
            "USN",
            HeaderValue::from_str(&uuid_usn).expect("This should never be invalid utf-8"),
        );

    println!("Sending device messages");
    socket.send(parse_request_to_string(request1).as_bytes())?;
    println!("Sent message 1");

    Ok(())
}

async fn broadcast_creation(socket: &UdpSocket, root_device_url: &str) -> tokio::io::Result<()> {
    socket
        .set_broadcast(true)
        .expect("set_broadcast call failed ");
    socket.connect("239.255.255.250:1900")?;

    broadcast_root_device_to_network(&socket, root_device_url).await?;
    broadcast_device_to_network(&socket, root_device_url).await?;
    broadcast_service_type_to_network(&socket, root_device_url).await?;

    socket
        .set_broadcast(false)
        .expect("set_broadcast(false) call failed ");
    tokio::io::Result::Ok(())
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    //239.255.255.250
    let address = "0.0.0.0";
    let port = 1900;
    println!("Opening UDP socket and listening on {}:{}", &address, &port);
    let socket = UdpSocket::bind(format!("{}:{}", &address, &port))?;
    socket
        .join_multicast_v4(
            &Ipv4Addr::new(239, 255, 255, 250),
            &Ipv4Addr::new(0, 0, 0, 0),
        )
        .expect("Failed to join multicast ");
    socket.set_multicast_loop_v4(true)?;
    // println!(
    //     "{}",
    //     socket
    //         .multicast_loop_v4()
    //         .expect("Failed to retrieve multicast loop ")
    // :;

    let tcplistener = TcpListener::bind("0.0.0.0:8081").await?;
    let local_ip = "192.168.178.9";
    println!("Opening TCP socket and listening on 0.0.0.0:8081");

    tokio::spawn(async move {
        let mut buf = [0; 8 * 1024];
        loop {
            let (mut socket, socket_addr) = tcplistener
                .accept()
                .await
                .expect("Failed to listen for tcp connection ");

            tokio::spawn(async move {
                loop {
                    let n = match socket.read(&mut buf).await {
                        // socket closed
                        Ok(n) if n == 0 => return,
                        Ok(n) => n,
                        Err(e) => {
                            println!("failed to read bytes: {}", e);
                            return;
                        }
                    };

                    println!("Received {} bytes", n);

                    let mut buf = &mut buf[..n];

                    println!("{}", socket_addr);
                    let text = match std::str::from_utf8(&mut buf) {
                        Ok(text) => {
                            println!("{}", text);
                            text
                        }
                        Err(e) => {
                            println!("Received invalid utf-8 text: {}", e);
                            return;
                        }
                    };

                    let mut headers: HashMap<&str, &str> = HashMap::new();
                    let method: &str;
                    let path: &str;
                    let protocol: &str;

                    let mut lines = text.lines();
                    if let Some(first_line) = lines.next() {
                        let words: Vec<&str> = first_line.split(" ").collect();

                        method = words[0];
                        path = words[1];
                        protocol = words[2];
                        println!("method: {} path: {} protocol: {}", method, path, protocol);
                    } else {
                        println!("Invalid request");
                        return;
                    }

                    lines.for_each(|line| {
                        if line.is_empty() {
                            return;
                        }

                        let words: Vec<&str> = line.split(": ").collect();
                        if words.len() == 2 {
                            headers.insert(words[0], words[1]);
                        } else {
                            println!("Invalid header line: {}", line);
                        }
                    });

                    println!("Headers: {:#?}", headers);

                    let xml;
                    let resp: &[u8] = if path == "/" && method == "GET" {
                        b"HTTP/1.1 200 OK
Connection: Keep-Alive
Access-Control-Allow-Origin: *
Content-Type: text/html; charset=utf-8

<html>
<body>TEST</body>
</html>"
                    } else if path == "/upnp_device_descriptor.xml" && method == "GET" {
                        let xml_content = &mut String::new();
                        println!(
                            "current filepath: {}",
                            std::env::current_dir().unwrap().display()
                        );
                        std::fs::File::open("./src/desc.xml")
                            .expect("Failed to open file: desc.xml")
                            .read_to_string(xml_content)
                            .expect("Failed to read file: desc.xml");
                        xml = format!(
                            "HTTP/1.1 200 OK
content-type: application/xml

{}",
                            xml_content
                        );
                        xml.as_bytes()
                    } else {
                        b"HTTP/1.1 404 Not Found"
                    };

                    println!("Waiting to become writeable");
                    socket.writable().await.expect("Failed to become writeable");
                    println!("Became writeable");

                    if let Err(e) = socket.write_all(resp).await {
                        println!("failed to write response: {}", e);
                        return;
                    }

                    socket.flush().await.expect("Failed to flush");
                    println!("Send response");
                    socket.shutdown().await.expect("Failed to shutdown");
                    println!("Shutdown connection");
                }
            });
        }
    });

    let root_device_url = format!("http://{}/", local_ip);
    broadcast_creation(&socket, &root_device_url).await?;

    // support up to 4KB, go for 8 just to be sure
    let mut buf = [0; 8 * 1024];
    loop {
        let (amt, src_addr) = socket.recv_from(&mut buf).expect("didn't receive data");

        println!("{:?}", src_addr);
        let mut data = &mut buf[..amt];
        let text = match std::str::from_utf8(&mut data) {
            Ok(msg) => Some(msg),
            Err(e) => {
                println!("Invalid utf-8 bytes {:?}", e);
                None
            }
        };

        if let Some(msg) = text {
            let mut header_found = false;
            for line in msg.lines() {
                if line == "ST: urn:dial-multiscreen-org:service:dial:1" {
                    header_found = true
                }
            }

            if header_found {
                println!("{}", msg);
                println!("DIAL ueader found :)");

                let response = format!(
                    "HTTP/1.1 200 OK
LOCATION: http://{}:8081/upnp_device_descriptor.xml
ST: urn:dial-multiscreen-org:service:dial:1
USN: testing-laptop
",
                    &local_ip
                );
                println!("Sendign LOCATION Resonse: {}", &response);
                socket
                    .send_to(&response.as_bytes(), src_addr)
                    .expect(&format!("failed to respond to {}", &src_addr));
            } else {
                println!("Timestamp: {:?}", std::time::SystemTime::now().elapsed());
                // println!("{}", msg);
                println!("No DIAL header found :(\n\n");
            }
        }
    }
}
