use crate::CLI;

pub mod logger;
pub mod time;

pub fn bootlog(cli: &CLI) {
    let name = env!("CARGO_PKG_NAME");
    let version = env!("CARGO_PKG_VERSION");
    let bits = std::mem::size_of::<usize>() * 8;
    let pid = std::process::id();
    let CLI { port, threads, .. } = cli;

    log::info!("{name} is starting");
    log::info!("version={version}, bits={bits}, pid={pid}, threads={threads}");

    println!(
        r"
    ⣿⣿⣿⡿⠋⠀⠉⠛⢿⣿⣿⣿⣿⣿⣿⣿⠟⠉⠀⡉⢻⣿⣿⣿⣿⣿
    ⣿⣿⢏⠞⡔⢠⣄⠀⠀⠙⢿⣿⣿⣿⣿⠃⠀⠀⠀⣿⠀⠻⣎⢿⣿⣿
    ⣿⡏⠞⣰⠃⣾⣿⣷⣄⠀⠀⠙⠿⠿⠃⠀⢀⣴⣷⢸⣆⠀⠹⣷⣻⣿
    ⡿⡰⠇⠀⣸⣿⣿⣿⠟⢁⡄⠀⢀⠀⠀⠀⠈⢻⣿⡞⣿⡄⠀⠹⣷⢿      {name} {version} (00000000/0) {bits} bit
    ⡇⠉⠀⣰⣿⣿⡿⠁⠔⠁⢡⠈⠀⠀⠀⠀⠀⠀⢹⣿⡘⣷⡀⠀⠹⣾
    ⣧⣀⣠⣿⣿⣿⢁⠀⢀⢀⣿⣧⣼⣷⠶⢀⠀⠀⠀⢿⣷⣜⠳⠀⢠⣿      Running in standalone mode
    ⣿⣿⣿⣿⣿⣿⢸⠀⠀⢨⣀⣽⣿⣿⣦⣴⠀⠂⠀⢸⣿⣿⣿⣿⣿⣿      Threads: {threads}
    ⣿⣿⣿⣿⣿⣿⡼⡇⠀⠘⣿⣿⣿⣿⣿⣇⠀⠀⠀⠘⣿⣿⣿⣿⣿⣿      Port: {port}
    ⣿⣿⣿⣿⣿⣿⣿⠃⡄⠀⠘⠻⠿⠿⣛⡅⠆⠀⠀⠀⢻⣿⣿⣿⣿⣿      PID: {pid}
    ⣿⣿⣿⣿⣿⣿⣿⠀⠁⠀⠀⢀⢬⣻⢿⣇⢀⣀⣀⡀⠀⢻⣿⣿⣿⣿
    ⣿⣿⣿⣿⣿⣿⡇⢀⣾⣿⣧⡜⣭⣿⠁⣀⢀⣼⣿⣿⡆⠈⢿⣿⣿⣿
    ⣿⣿⣿⣿⣿⡿⠀⢸⣿⣿⣿⢻⣿⣿⣷⣿⣿⣿⣿⣿⡻⠀⡘⣿⣿⣿
    ⣿⣿⣿⣿⣿⠃⠀⠸⣿⣿⣿⣿⠟⣙⡻⢿⣮⢿⣿⡟⡡⡄⠡⢹⣿⣿
    ⣿⣿⣿⣿⣿⢰⠀⡆⣿⣿⣿⡟⣸⣆⣿⣷⠍⠻⠏⠴⠿⠃⠀⠈⣿⣿
    ⣿⣿⣿⣿⣿⢸⡄⢁⢸⣿⣿⡇⠘⠛⠉⠁⠀⠀⠀⠀⠀⠀⠀⡇⣿⣿
    ⣿⣿⣿⣿⣿⣿⡗⠀⠀⢿⣿⡇⠀⠀⠀⠀⠀⠀⠀⢠⢠⠂⠀⣷⣿⣿
",
    );
}
