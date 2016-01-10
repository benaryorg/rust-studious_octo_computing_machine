#[macro_use]
extern crate log;
extern crate daemonize;
extern crate unix_socket;
extern crate flexi_logger;
extern crate threadpool;
extern crate atom;

use unix_socket::UnixStream;
use unix_socket::UnixListener;
use daemonize::Daemonize;
use threadpool::ThreadPool;
use atom::AtomSetOnce;

use std::sync::Arc;
use std::io::BufReader;
use std::io::BufWriter;
use std::io::BufRead;
use std::io::Write;

const SOCKFILE: &'static str = "/var/tmp/unixsock.sock";
const PIDFILE: &'static str = "/var/tmp/unixsock.pid";
const LOGDIR: &'static str = "/var/log/unixsock/";

fn main()
{
	flexi_logger::init(flexi_logger::LogConfig
	{
		log_to_file: true,
		directory: Some(LOGDIR.into()),
		..  flexi_logger::LogConfig::new()
	},None).unwrap();

	debug!("socket: {}.",&SOCKFILE);
	debug!("pid: {}.",&PIDFILE);
	debug!("logs: {}.",&LOGDIR);

	let daemon = Daemonize::new()
		.pid_file(&PIDFILE)
		.privileged_action(
		{
			||UnixListener::bind(&SOCKFILE)
		});

	let server = daemon.start().unwrap().unwrap();
	info!("socket bound");
	info!("daemonized");

	let pool = ThreadPool::new(16);
	info!("thread pool initialized");

	let running = Arc::new(AtomSetOnce::empty());

	info!("listening");
	for stream in server.incoming().take_while(|_|running.is_none())
	{
		if let Ok(stream) = stream 
		{
			info!("new connection");
			let running = running.clone();
			pool.execute(move||
			{
				let read = BufReader::new(&stream);
				let mut write = BufWriter::new(&stream);

				for line in read.lines()
					.map(|l|l.unwrap())
				{
					debug!("read line: {}",line);
					if line == "shutdown"
					{
						info!("received shutdown");
						running.clone().set_if_none(Box::new(()));
						info!("last-connect");
						UnixStream::connect(&SOCKFILE).unwrap();
						break;
					}
					writeln!(write,"{}",line).unwrap();
					debug!("wrote line: {}",line);
					write.flush().unwrap();
					debug!("flushed");
				}
			});
		}
	}

	info!("shuting down server");
	drop(server);
	info!("deleting file");
	std::fs::remove_file(&SOCKFILE).unwrap();
	info!("socket removed");
}

