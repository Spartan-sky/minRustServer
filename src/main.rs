use std::{
    io::{prelude::*, Read, Write, Error, BufReader, ErrorKind},
    net::{TcpStream, TcpListener},
    sync::{mpsc, Arc, Mutex},
    thread,
};

const LOCAL_HOST: &str = "127.0.0.1:8080";
const MESSAGE_SIZE: usize = 32;

pub struct ThreadPool {
    workers: Vec<Worker>,
    sender: Option<mpsc::Sender<Job>>,
}

type Job = Box<dyn FnOnce() + Send + 'static>;

impl ThreadPool {
    pub fn new(size: usize) -> ThreadPool {
        assert!(size > 0);

        let (sender, receiver) = mpsc::channel();

        let receiver = Arc::new(Mutex::new(receiver));

        let mut workers = Vec::with_capacity(size);

        for id in 0..size {
            workers.push(Worker::new(id, Arc::clone(&receiver)));
        }

        ThreadPool { 
            workers,
            sender: Some(sender),
        }
    }

    pub fn execute<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let job = Box::new(f);

        self.sender.as_ref().unwrap().send(job).unwrap();
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        drop(self.sender.take());

        for worker in &mut self.workers {
            println!("Shutting down worker {}", worker.id);

            if let Some(thread) = worker.thread.take() {
                thread.join().unwrap();
            }
        }
    }
}

struct Worker {
    id: usize,
    thread: Option<thread::JoinHandle<()>>,
}

impl Worker {
    fn new(id: usize, receiver: Arc<Mutex<mpsc::Receiver<Job>>>) -> Worker {
        let thread = thread::spawn(move || loop {
            let message = receiver.lock().unwrap().recv();

            match message {
                Ok(job) => {
                    println!("Worker {id} got a job; executing.");

                    job();
                }
                Err(_) => {
                    println!("Worker {id} disconnected; shutting down.");
                    break;
                }
            }
        });

        Worker {
            id,
            thread: Some(thread),
        }
    }
}

fn handle_client(mut stream: TcpStream) -> Result<(), Error> {
    println!("Connection from {}", stream.peer_addr()?);

    let mut buffer = [0; 240];
    

    let nbytes = stream.read(&mut buffer)?;
    if nbytes == 0 {
        return Ok(());
    }

    print!("Their username is:");
    for i in 0..nbytes {
        print!("{}", buffer[i] as char);
    }
    println!("");

    loop{
        let nbytes = stream.read(&mut buffer)?;
        if nbytes == 0 {
            return Ok(());
        }

        println!("The message recieved says:");
        for i in 0..nbytes {
            print!("{}", buffer[i] as char);
        }
        println!("");

        stream.write(&buffer)?;
        stream.flush();
    }
}

fn main() {
    let listener = TcpListener::bind(LOCAL_HOST).expect("Could not bind socket");

    listener.set_nonblocking(true).expect("Failed to initialize non-blocking");

    let mut clients = vec![];

    let (sender, receiver) = mpsc::channel::<String>();

    loop {
        if let Ok((mut socket, address)) = listener.accept() {
            println!("Client {}: CONNECTED", address);

            let sender = sender.clone();
            clients.push(socket.try_clone().expect("Failed to clone client"));

            thread::spawn(move || loop {
                let mut buffer = vec![0; MESSAGE_SIZE];

                match socket.read_exact(&mut buffer){
                    Ok(_) => {
                        let message = buffer.into_iter().take_while(|&x| x != 0).collect::<Vec<_>>();
                        let message = String::from_utf8(message).expect("Invalid utf8 message");
                        println!("{}: {:?}", address, message);
                        sender.send(message).expect("Failed to send message to receiver");
                    },
                    Err(ref err) if err.kind() == ErrorKind::WouldBlock => (),
                    Err(_) => {
                        println!("Closing connection with: {}", address);
                        break;
                    }
                }
                thread::sleep(::std::time::Duration::from_millis(100));
            });
        }
        if let Ok(message) = receiver.try_recv() {
            clients = clients.into_iter().filter_map(|mut client| {
                let mut buffer = message.clone().into_bytes();
                buffer.resize(MESSAGE_SIZE, 0);
                client.write_all(&buffer).map(|_| client).ok()
            }).collect::<Vec<_>>();
        }
        thread::sleep(::std::time::Duration::from_millis(100));
    }
}
