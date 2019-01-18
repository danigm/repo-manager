use jsonwebtoken as jwt;
#[macro_use]
extern crate serde_derive;
use base64;



use serde_json;

use std::io;
use std::io::prelude::*;
use std::fs;
use std::process;
use crate::jwt::{encode, Header};
use chrono::{Utc, Duration};

use argparse::{ArgumentParser, StoreTrue, Store, StoreOption, List};

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String,
    scope: Vec<String>,
    name: String,
    prefix: Option<Vec<String>>,
    exp: i64,
}

fn read_secret(filename: String) -> io::Result<String> {
    let mut contents = String::new();
    if filename == "-" {
        io::stdin().read_to_string(&mut contents)?;
    } else {
        let mut file = fs::File::open(filename)?;
        file.read_to_string(&mut contents)?;
    }
    Ok(contents)
}

fn main() {
    let mut verbose = false;
    let mut base64 = false;
    let mut name = "default".to_string();
    let mut sub = "build".to_string();
    let mut secret: Option<String> = None;
    let mut secret_file: Option<String> = None;
    let mut duration: i64 = Duration::days(365).num_seconds();
    let mut scope: Vec<String> = vec![];
    let mut prefix: Vec<String> = vec![];

    {
        let mut ap = ArgumentParser::new();
        ap.set_description("Generate token for repo-manager.");
        ap.refer(&mut verbose)
            .add_option(&["-v", "--verbose"], StoreTrue,
                        "Be verbose");
        ap.refer(&mut name)
            .add_option(&["--name"], Store,
                        "Name for the token");
        ap.refer(&mut sub)
            .add_option(&["--sub"], Store,
                        "Subject (default: build)");
        ap.refer(&mut scope)
            .add_option(&["--scope"], List,
                        "Add scope (default if none: [build, upload, publish, jobs]");
        ap.refer(&mut prefix)
            .add_option(&["--prefix"], List,
                        "Add ref prefix (default if none: ['']");
        ap.refer(&mut base64)
            .add_option(&["--base64"], StoreTrue,
                        "The secret is base64 encoded");
        ap.refer(&mut secret)
            .add_option(&["--secret"], StoreOption,
                        "Secret used to encode the token");
        ap.refer(&mut secret_file)
            .add_option(&["--secret-file"], StoreOption,
                        "Load secret from file (or - for stdin)");
        ap.refer(&mut duration)
            .add_option(&["--duration"], Store,
                        "Duration for key in seconds (default 1 year)");
        ap.parse_args_or_exit();
    }

    let secret_contents;

    if scope.len() == 0 {
        scope = vec!["build".to_string(), "upload".to_string(), "publish".to_string(), "jobs".to_string()];
    }

    if prefix.len() == 0 {
        prefix = vec!["".to_string()];
    }

    if let Some(s) = secret {
        secret_contents = s.clone();
    } else if let Some(filename) = secret_file {
        match read_secret(filename) {
            Ok(contents) => secret_contents = contents,
            Err(e) => {
                eprintln!("Error reading secrets: {}", e);
                process::exit(1)
            }
        }
    } else {
        eprintln!("No secret specified, use --secret or --secret-file");
        process::exit(1)
    }

    let bytes: Vec<u8> = if base64 {
        base64::decode(secret_contents.trim()).unwrap()
    } else {
        secret_contents.trim().as_bytes().to_vec()
    };

    let claims = Claims {
        sub: sub,
        scope: scope,
        prefix: Some(prefix),
        name: name.clone(),
        exp: Utc::now().timestamp() + Duration::days(365).num_seconds(),
    };

    if verbose {
        println!("Token: {}", serde_json::to_string(&claims).unwrap());
    }

    let token = encode(&Header::default(), &claims, &bytes).unwrap();
    println!("{}", token);
}
