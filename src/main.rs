use bytecodec::{DecodeExt, Encode};
use httpcodec::{HttpVersion, ReasonPhrase, Request, RequestDecoder, Response, StatusCode, ResponseEncoder, BodyEncoder};
use bytecodec::bytes::BytesEncoder;
use bytecodec::io::IoEncodeExt;
use std::io::{Read, Write};
use std::fs;
use wasmedge_wasi_socket::{Shutdown, TcpListener, TcpStream};

fn handle_http(req: Request<String>) -> bytecodec::Result<Response<Vec<u8>>> {
    let t = req.request_target().as_ref().to_string();
    let target = if t == "/" {
        "/index.html".to_string()
    } else {
        t
    };
    let path = format!("files{}", target);
    let contents = fs::read(path)
        .unwrap();

    Ok(Response::new(
        HttpVersion::V1_0,
        StatusCode::new(200)?,
        ReasonPhrase::new("")?,
          contents,
    ))
}

fn handle_client(mut stream: TcpStream) -> std::io::Result<()> {
    let mut buff = [0u8; 1024];
    let mut data = Vec::new();

    loop {
        let n = stream.read(&mut buff)?;
        data.extend_from_slice(&buff[0..n]);
        if n < 1024 {
            break;
        }
    }

    let mut decoder =
        RequestDecoder::<httpcodec::BodyDecoder<bytecodec::bytes::Utf8Decoder>>::default();

    let req = match decoder.decode_from_bytes(data.as_slice()) {
        Ok(req) => handle_http(req),
        Err(e) => Err(e),
    };

    let r = match req {
        Ok(r) => r,
        Err(e) => {
            let err = format!("{:?}", e);
            Response::new(
                HttpVersion::V1_0,
                StatusCode::new(500).unwrap(),
                ReasonPhrase::new(err.clone().as_str()).unwrap(),
                err.clone().as_bytes().to_vec(),
            )
        }
    };

    let mut encoder = ResponseEncoder::new(BodyEncoder::new(BytesEncoder::new()));
    encoder.start_encoding(r).unwrap();
    let mut buf = Vec::new();
    encoder.encode_all(&mut buf).unwrap();

    stream.write(buf.as_slice())?;
    stream.shutdown(Shutdown::Both)?;
    Ok(())
}

fn main() -> std::io::Result<()> {
    let port = "1234";
    println!("listening at {}", port);
    let listener = TcpListener::bind(format!("0.0.0.0:{}", port), false)?;
    loop {
        let _ = handle_client(listener.accept(false)?.0);
    }
}
