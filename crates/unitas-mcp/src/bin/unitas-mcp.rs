use std::io;
use std::process::ExitCode;
use unitas_mcp::StdioServer;

fn main() -> ExitCode {
    let mut server = match StdioServer::with_defaults() {
        Ok(server) => server,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::FAILURE;
        }
    };
    let stdin = io::stdin();
    let stdout = io::stdout();
    if let Err(error) = server.serve(stdin.lock(), stdout.lock()) {
        eprintln!("{error}");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}
