use std::{
    io::{BufRead, BufReader},
    process::{Child, Command, Stdio},
    sync::mpsc,
    time::Duration,
};

struct Server(Child);

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[test]
#[ignore = "requires the Hurl CLI"]
fn todo_api_over_http() {
    let db_dir = tempfile::tempdir().unwrap();
    let mut server = Server(
        Command::new(env!("CARGO_BIN_EXE_server"))
            .current_dir(db_dir.path())
            .env("DATABASE_URL", "sqlite://todos.db")
            .env("BIND_ADDR", "127.0.0.1:0")
            .stderr(Stdio::piped())
            .spawn()
            .expect("start the server"),
    );

    let stderr = server.0.stderr.take().unwrap();
    let (send, receive) = mpsc::channel();
    std::thread::spawn(move || {
        for line in BufReader::new(stderr).lines().map_while(Result::ok) {
            eprintln!("{line}");
            if let Some(url) = line.strip_prefix("Listening on ") {
                let _ = send.send(url.to_owned());
            }
        }
    });
    let url = receive
        .recv_timeout(Duration::from_secs(15))
        .expect("server did not announce its listener");

    let status = Command::new("hurl")
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .args([
            "--test",
            "--max-time",
            "10",
            "--variable",
            &format!("base={url}"),
            "tests/e2e/todos.hurl",
        ])
        .status()
        .expect("install Hurl and make it available on PATH");
    assert!(status.success(), "Hurl failed: {status}");
}
