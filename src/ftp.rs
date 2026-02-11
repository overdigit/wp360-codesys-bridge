use serde::{Deserialize, Serialize};
use std::borrow::{Borrow, Cow};
use std::error::Error;
use std::fs::File;
use std::io::prelude::*;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use suppaftp::FtpError;
use suppaftp::rustls;
use suppaftp::rustls::ClientConfig;
use suppaftp::{RustlsConnector, RustlsFtpStream};

#[derive(Deserialize, Debug)]
#[serde(tag = "command")]
enum Command<'a> {
    NoOp,
    Upload {
        #[serde(borrow)]
        local: Cow<'a, Path>,
        remote: Option<&'a str>,
    },
    Download {
        local: Option<&'a Path>,
        remote: &'a str,
    },
    Rename {
        remote: &'a str,
        new_name: &'a str,
    },
    Delete {
        remote: &'a str,
    },
    GetFileSize {
        remote: &'a str,
    },
    SetDirectory {
        remote: &'a str,
    },
    GetDirectory,
    CreateDirectory {
        remote: &'a str,
    },
    DeleteDirectory {
        remote: &'a str,
    },
}

#[derive(Serialize, Debug)]
#[serde(untagged)]
enum FtpResult {
    GetFileSize {
        size: usize,
    },
    GetDirectory {
        path: String,
    },
    Generic {
        success: bool,
    },
    Error {
        success: bool,
        error: String,
        code: u32,
    },
}

impl From<OperationError> for FtpResult {
    fn from(e: OperationError) -> Self {
        match e {
            OperationError::Ftp(e) => match e {
                FtpError::ConnectionError(e) => e.into(),
                FtpError::UnexpectedResponse(r) => FtpResult::Error {
                    success: false,
                    code: 1000 + r.status.code(),
                    error: "Unexpected server response".to_string(),
                },
                FtpError::SecureError(s) => FtpResult::Error {
                    success: false,
                    code: 1001,
                    error: s,
                },
                FtpError::BadResponse => FtpResult::Error {
                    success: false,
                    code: 1002,
                    error: "Bad response".to_string(),
                },
                FtpError::InvalidAddress(_e) => {
                    unreachable!()
                }
                FtpError::DataConnectionAlreadyOpen => FtpResult::Error {
                    success: false,
                    code: 1003,
                    error: "Data connection already open".to_string(),
                },
            },
            OperationError::Io(e) => e.into(),
            OperationError::Serde(e) => e.into(),
        }
    }
}

impl From<std::io::Error> for FtpResult {
    fn from(error: std::io::Error) -> Self {
        match error.kind() {
            std::io::ErrorKind::InvalidFilename => FtpResult::Error {
                success: false,
                code: 1004,
                error: "Local path not in base folder".to_string(),
            },
            std::io::ErrorKind::InvalidInput => FtpResult::Error {
                success: false,
                code: 1005,
                error: "Invalid local path".to_string(),
            },
            std::io::ErrorKind::InvalidData => FtpResult::Error {
                success: false,
                code: 1006,
                error: "Invalid UTF-8 in remote path".to_string(),
            },
            _ => {
                eprintln!("{:?}", error);
                todo!()
            }
        }
    }
}

impl From<serde_json::Error> for FtpResult {
    fn from(error: serde_json::Error) -> Self {
        match error.classify() {
            serde_json::error::Category::Io => unreachable!(),
            serde_json::error::Category::Syntax => FtpResult::Error {
                success: false,
                code: 1007,
                error: "JSON syntax error".to_string(),
            },
            serde_json::error::Category::Data => FtpResult::Error {
                success: false,
                code: 1008,
                error: "Invalid JSON command".to_string(),
            },
            serde_json::error::Category::Eof => FtpResult::Error {
                success: false,
                code: 1009,
                error: "Unfinished JSON command".to_string(),
            },
        }
    }
}

#[derive(Deserialize, Debug)]
#[serde(default)]
struct Connection {
    hostname: String,
    port: u16,
    username: String,
    password: String,
    passive: bool,
    tls: bool,
}
impl Default for Connection {
    fn default() -> Self {
        Connection {
            hostname: "".to_string(),
            port: 21,
            username: "".to_string(),
            password: "".to_string(),
            passive: true,
            tls: false,
        }
    }
}

#[derive(Debug)]
enum OperationError {
    Ftp(FtpError),
    Io(std::io::Error),
    Serde(serde_json::Error),
}
impl From<std::io::Error> for OperationError {
    fn from(error: std::io::Error) -> Self {
        OperationError::Io(error)
    }
}
impl From<FtpError> for OperationError {
    fn from(error: FtpError) -> Self {
        OperationError::Ftp(error)
    }
}
impl From<serde_json::Error> for OperationError {
    fn from(error: serde_json::Error) -> Self {
        OperationError::Serde(error)
    }
}

pub fn init() -> Result<(), Box<dyn Error>> {
    let root_store =
        rustls::RootCertStore::from_iter(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());

    let config = ClientConfig::builder()
        .with_root_certificates(root_store)
        .with_no_client_auth();

    let mut buf: String = "".to_string();
    std::io::stdin().read_line(&mut buf)?;
    let connection_params: Connection = serde_json::from_str(&buf)?;

    let mut ftp_stream = RustlsFtpStream::connect(format!(
        "{}:{}",
        connection_params.hostname, connection_params.port
    ))?;
    ftp_stream.set_mode(if connection_params.passive {
        suppaftp::types::Mode::Passive
    } else {
        suppaftp::types::Mode::Active
    });
    if connection_params.tls {
        ftp_stream = ftp_stream.into_secure(
            RustlsConnector::from(Arc::new(config)),
            &connection_params.hostname,
        )?;
    }
    ftp_stream.login(connection_params.username, connection_params.password)?;
    ftp_stream.transfer_type(suppaftp::types::FileType::Binary)?;
    println!(
        "{}",
        serde_json::to_string(&FtpResult::Generic { success: true }).unwrap()
    );

    buf.clear();
    loop {
        let Ok(n) = std::io::stdin().read_line(&mut buf) else {
            let _ = ftp_stream.quit();
            return Err("Couldn't read from input".into());
        };
        if n == 0 {
            break;
        }
        match perform_operation(&mut ftp_stream, &buf) {
            Ok(t) => println!("{}", serde_json::to_string(&t).unwrap()),
            Err(e) => {
                let err = FtpResult::from(e);
                eprintln!("{:?}", err);
                println!("{}", serde_json::to_string(&err).unwrap());
            }
        }
        buf.clear();
    }

    Ok(())
}

fn perform_operation<'a>(
    ftp: &'a mut RustlsFtpStream,
    buf: &'a str,
) -> Result<FtpResult, OperationError> {
    let command: Command = serde_json::from_str(buf)?;
    match command {
        Command::NoOp {} => {
            ftp.noop()?;
        }

        Command::Upload { local, remote } => {
            let path = ftp_path(local.borrow())?;
            let mut file = File::open(&path)?;
            let filename;
            match remote {
                Some(t) => filename = t,
                None => match &path.file_name().unwrap().to_str() {
                    Some(p) => filename = p,
                    None => {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            "Invalid UTF8 sequence in remote filename",
                        )
                        .into());
                    }
                },
            }
            ftp.put_file(remote.unwrap_or(filename), &mut file)?;
        }

        Command::Download { local, remote } => {
            let path;
            if remote.ends_with("/") {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::IsADirectory,
                    "Remote path is a directory",
                )
                .into());
            }
            match local {
                Some(p) => path = ftp_path(p)?,
                None => match Path::new(remote).file_name().unwrap().to_str() {
                    Some(p) => path = ftp_path(Path::new(p))?,
                    None => {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            "Invalid UTF8 sequence in remote filename",
                        )
                        .into());
                    }
                },
            }
            let mut file = File::create(&path)?;
            let buf = ftp.retr_as_buffer(remote)?;
            file.write_all(buf.get_ref())?;
        }

        Command::Rename { remote, new_name } => {
            ftp.rename(remote, new_name)?;
        }

        Command::Delete { remote } => {
            ftp.rm(remote)?;
        }

        Command::GetFileSize { remote } => {
            let size = ftp.size(remote)?;
            return Ok(FtpResult::GetFileSize { size });
        }

        Command::SetDirectory { remote } => {
            ftp.cwd(remote)?;
        }

        Command::GetDirectory {} => {
            let path = ftp.pwd()?;
            return Ok(FtpResult::GetDirectory { path });
        }

        Command::CreateDirectory { remote } => {
            ftp.mkdir(remote)?;
        }

        Command::DeleteDirectory { remote } => {
            ftp.rmdir(remote)?;
        }
    }
    Ok(FtpResult::Generic { success: true })
}

fn ftp_path(path: &Path) -> Result<PathBuf, std::io::Error> {
    let base_path = Path::new("/home/nix/build/wp360-codesys-bridge-rs/");
    let complete_path = base_path.join(path);

    if complete_path == base_path {
        return Ok(complete_path);
    }

    let Some(parent) = complete_path.parent() else {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "Invalid local path",
        ));
    };
    let Some(filename) = complete_path.file_name() else {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "Invalid local path",
        ));
    };

    let true_parent = parent.canonicalize()?;
    if !true_parent.starts_with(base_path) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidFilename,
            "Local path not in base folder",
        ));
    }
    let true_path = parent.join(filename);
    Ok(true_path)
}
