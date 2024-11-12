use std::{
    io::{prelude::*, Read, Write, Error, BufReader},
    net::{TcpStream, TcpListener},
    sync::{mpsc, Arc, Mutex},
    thread,
};

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

fn main() -> std::io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:8080").expect("TcpListener for 127.0.0.1:8080 failed");
    let pool = ThreadPool::new(4);
    
    // accept connections and process them serially
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                pool.execute(|| {
                    let _ = handle_client(stream);
                });
            }
            Err(e) => {
                println!("There was a connection errer with error {e}!");
            }
        }
    }

    Ok(())
}
