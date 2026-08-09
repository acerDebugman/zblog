//! Print an Argon2 PHC hash for `ZBLOG_PASSWORD_HASH`.
//!
//! Usage: `cargo run --example hash_password -- 'your-password'`

use argon2::{
    password_hash::{rand_core::OsRng, SaltString},
    Argon2, PasswordHasher,
};

fn main() {
    let Some(password) = std::env::args().nth(1) else {
        eprintln!("usage: hash_password <password>");
        std::process::exit(2);
    };
    let salt = SaltString::generate(&mut OsRng);
    match Argon2::default().hash_password(password.as_bytes(), &salt) {
        Ok(hash) => println!("{hash}"),
        Err(e) => {
            eprintln!("failed to hash password: {e}");
            std::process::exit(1);
        }
    }
}
