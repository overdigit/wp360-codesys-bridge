use serde::{Deserialize, Serialize};
use std::borrow::{Borrow, Cow};
use std::error::Error;
use std::fs::File;
use std::io::prelude::*;
use std::path::{Path, PathBuf};
use suppaftp::native_tls::{TlsConnector, TlsStream};
use suppaftp::{FtpError, NativeTlsConnector, NativeTlsFtpStream};

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
enum FtpResult<'a> {
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
        error: &'a str,
        code: u32,
    },
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
enum OperationError<'a> {
    FtpError(FtpError),
    IoError(std::io::Error),
    SerdeError(serde_json::Error),
    InvalidPathError(&'a str),
}
impl From<std::io::Error> for OperationError<'_> {
    fn from(error: std::io::Error) -> Self {
        OperationError::IoError(error)
    }
}
impl From<FtpError> for OperationError<'_> {
    fn from(error: FtpError) -> Self {
        OperationError::FtpError(error)
    }
}
impl From<serde_json::Error> for OperationError<'_> {
    fn from(error: serde_json::Error) -> Self {
        OperationError::SerdeError(error)
    }
}

pub fn init() -> Result<(), Box<dyn Error>> {
    let mut buf: String = "".to_string();
    std::io::stdin().read_line(&mut buf)?;
    let connection_params: Connection = serde_json::from_str(&buf)?;

    let mut ftp_stream = NativeTlsFtpStream::connect(format!(
        "{}:{}",
        connection_params.hostname, connection_params.port
    ))?;
    ftp_stream.set_mode(if connection_params.passive { suppaftp::types::Mode::Passive } else { suppaftp::types::Mode::Active } );
    if connection_params.tls {
        eprintln!("Activating FTPS");
        ftp_stream = ftp_stream.into_secure(
            NativeTlsConnector::from(TlsConnector::new().unwrap()),
            &connection_params.hostname,
        )?;
        eprintln!("FTPS activated");
    }
    ftp_stream.login(connection_params.username, connection_params.password)?;
    ftp_stream.transfer_type(suppaftp::types::FileType::Binary);

    buf.clear();
    while let Ok(n) = std::io::stdin().read_line(&mut buf) {
        if n == 0 {
            break;
        }
        match perform_operation(&mut ftp_stream, &buf) {
            Ok(t) => println!("{}", serde_json::to_string(&t).unwrap()),
            Err(e) => {
                let err = get_ftp_error(e);
                eprintln!("{:?}", err);
                println!("{}", serde_json::to_string(&err).unwrap());
            }
        }
        buf.clear();
    }

    Ok(())
}

fn get_ftp_error(e: OperationError) -> FtpResult {
    match e {
        OperationError::FtpError(e) => match e {
            suppaftp::FtpError::ConnectionError(e) => {
                todo!();
            },
            suppaftp::FtpError::UnexpectedResponse(r) => {
                let Ok(response) = std::str::from_utf8(&r.body) else {
                    return FtpResult::Error {
                        success: false,
                        code: 1999,
                        error: "Invalid UTF-8 in server response",
                    };
                };
                return FtpResult::Error {
                    success: false,
                    code: 1000 + r.status.code(),
                    error: "Unexpected server response",
                };
            },
            _ => todo!(),
        },
        OperationError::IoError(_) | OperationError::SerdeError(_) | OperationError::InvalidPathError(_) => todo!()
    }
}

fn perform_operation<'a>(
    ftp: &'a mut NativeTlsFtpStream,
    buf: &'a String,
) -> Result<FtpResult<'a>, OperationError<'a>> {
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
                        return Err(OperationError::InvalidPathError(
                            "Invalid UTF8 sequence in filename",
                        ));
                    }
                },
            }
            ftp.put_file(remote.unwrap_or(filename), &mut file)?;
        }

        Command::Download { local, remote } => {
            let path;
            if remote.ends_with("/") {
                return Err(OperationError::InvalidPathError(
                    "Remote path is a directory",
                ));
            }
            match local {
                Some(p) => path = ftp_path(p)?,
                None => match Path::new(remote).file_name().unwrap().to_str() {
                    Some(p) => path = ftp_path(Path::new(p))?,
                    None => return Err(OperationError::InvalidPathError("Invalid remote path")),
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
            return Ok(FtpResult::GetFileSize { size: size });
        }

        Command::SetDirectory { remote } => {
            ftp.cwd(remote)?;
        }

        Command::GetDirectory {} => {
            let path = ftp.pwd()?;
            return Ok(FtpResult::GetDirectory { path: path });
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
    let BASE_PATH = Path::new("/home/nix/build/wp360-codesys-bridge-rs/");
    let complete_path = BASE_PATH.join(path);
    let Some(parent) = complete_path.parent() else {
        return Err(std::io::Error::other("Invalid local path"));
    };
    let Some(filename) = complete_path.file_name() else {
        return Err(std::io::Error::other("Invalid local path"));
    };
    let true_parent = parent.canonicalize()?;
    if !true_parent.starts_with(BASE_PATH) {
        return Err(std::io::Error::other("Invalid local path"));
    }
    let true_path = parent.join(filename);
    Ok(true_path)
}
