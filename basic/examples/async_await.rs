use std::{
    net::{SocketAddr, UdpSocket},
    task::{Context, Poll, RawWaker, RawWakerVTable, Waker},
    future::Future,
    os::unix::prelude::{AsRawFd, RawFd},
    collections::{HashMap, VecDeque},
    rc::Rc, cell::RefCell,
    str,
};
use std::io;
use core::pin::Pin;

const READ_FLAGS: i32 = libc::EPOLLIN;
const WRITE_FLAGS: i32 = libc::EPOLLOUT;

#[allow(unused_macros)]
macro_rules! syscall {
    ($fn: ident ( $($arg: expr),* $(,)* ) ) => {{
        let res = unsafe { libc::$fn($($arg, )*) };
        if res == -1 {
            Err(std::io::Error::last_os_error())
        } else {
            Ok(res)
        }
    }};
}
struct Reactor {
    events: Vec<libc::epoll_event>,
    epoll_fd: RawFd,
}

pub type LocalFuture = dyn Future<Output = ()> + 'static;

pub struct Task where {
    future: Rc<LocalFuture>,
}

struct Excutor {
    tasks: VecDeque<Box<Task>>,
    reactor: Rc<RefCell<Reactor>>,
}

impl Excutor {
    fn new() -> Self {
        Excutor {
            tasks: VecDeque::new(),
            reactor: Reactor::new(),
        }

    }

    fn block_on<T>(&mut self, fut: T) where T:Future<Output=()> + 'static {
        //let mut cx = get_contexts();
        let mywaker = Rc::new(MyWaker {});

        let raw_waker = RawWaker::new(Rc::into_raw(mywaker) as *const (), &VTABLE);
        let waker = unsafe { Waker::from_raw(raw_waker) };

        let mut cx = Context::from_waker(&waker);

        let mut pinfut = Box::pin(fut);
        loop {
            println!("block_on loop");
            //let pinfut = Pin::new(&mut fut);
            //self.reactor.borrow_mut().wait();
            if let Poll::Ready(t) = pinfut.as_mut().poll(&mut cx) {
                println!("block_on break");
                break t;
            }
            println!("reactor wait");
            self.reactor.borrow_mut().wait();
        }
    }
}

impl Reactor {
    fn new() -> Rc<RefCell<Self>> {
        let epoll_fd = epoll_create().expect("can create epoll queue");

        Rc::new(RefCell::new(Reactor{
            events: Vec::with_capacity(1024),
            epoll_fd,
        }))
    }
    
    fn wait(&mut self) {
        self.events.clear();
        let res = match syscall!(epoll_wait(
            self.epoll_fd,
            self.events.as_mut_ptr() as *mut libc::epoll_event,
            1024,
            -1 as libc::c_int,
        )) {
            Ok(v) => v,
            Err(e) => panic!("error during epoll wait: {}", e), 
        };
        println!("epoll wait return:{}", res as usize);
        unsafe { self.events.set_len(res as usize) };
    }
}

async fn test_loop1() {
    println!("test loop1");
    std::thread::sleep(std::time::Duration::from_secs(2));
}

async fn test_loop() {
    loop{
    test_loop1().await ;
        println!("test loop")
    }
    
}

fn main() {
    let mut ex = Excutor::new();
    let mut udpfut = Rc::new(RefCell::new(UDP::new(&ex)));
    //ex.block_on(test_loop())
    ex.block_on(pp(Rc::clone(&udpfut)))
}

struct MyWaker {}

fn mywaker_wake(s: &MyWaker) {
    todo!()
}

fn mywaker_clone(s: &MyWaker) -> RawWaker {
    let rc = unsafe { Rc::from_raw(s) };
    std::mem::forget(rc.clone()); // increase ref count
    RawWaker::new(Rc::into_raw(rc) as *const (), &VTABLE)
}

const VTABLE: RawWakerVTable = unsafe {
    RawWakerVTable::new(
        |s| mywaker_clone(&*(s as *const MyWaker)),
        |s| mywaker_wake(&*(s as *const MyWaker)),  
        |s| mywaker_wake(&*(s as *const MyWaker)), 
        |s| drop(Rc::from_raw(s as *const MyWaker)),
    )
};

fn epoll_create() -> io::Result<RawFd> {
    let fd = syscall!(epoll_create1(0))?;
    if let Ok(flags) = syscall!(fcntl(fd, libc::F_GETFD)) {
        let _ = syscall!(fcntl(fd, libc::F_SETFD, flags | libc::FD_CLOEXEC));
    }

    Ok(fd)
}

fn add_interest(epoll_fd: RawFd, fd: RawFd, mut event: libc::epoll_event) -> io::Result<()> {
    syscall!(epoll_ctl(epoll_fd, libc::EPOLL_CTL_ADD, fd, &mut event))?;
    Ok(())
}

fn listener_read_event(key: u64) -> libc::epoll_event {
    libc::epoll_event {
        events: READ_FLAGS as u32,
        u64: key,
    }
}

struct UDP {
    stdudp: UdpSocket,
    reactor: Rc<RefCell<Reactor>>,
    buf: [u8; 1500],
}

impl UDP {
    fn new(ex: &Excutor) -> UDP {
        let sockfd = UdpSocket::bind("127.0.0.1:8080").unwrap();
        sockfd.set_nonblocking(true);
        let udprawfd = sockfd.as_raw_fd();
        add_interest(ex.reactor.borrow().epoll_fd, udprawfd, listener_read_event(1));

        UDP {
            stdudp: sockfd,
            reactor: Rc::clone(&ex.reactor),
            buf: [0u8; 1500],
        }
    }
    fn read(&mut self) -> io::Result<(usize, SocketAddr)> {
        println!("before recv_from");
        let result = self.stdudp.recv_from(&mut self.buf);
        if let Ok(r) = result {
            match str::from_utf8(&self.buf[0..r.0]) {
                Ok(s) => println!("after recv_from:{} {} {}", r.0, r.1, s),
                Err(e) => println!("read ok:{} {} err:{}", r.0, r.1, e),
            }
        } else {
            println!("after recv_from");
        }
        //std::thread::sleep(std::time::Duration::from_secs(1));
        return result;
    }
}

impl Future for UDP {
    type Output = io::Result<(usize, SocketAddr)>;
    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        //let result = cself.get_mut().read());
        let result = self.get_mut().read();

        if let Err(e) = result {
            println!("return pending:{}", e);
            return Poll::Pending;
        }
        return std::task::Poll::Ready(result);
    }
}

async fn pp(udpfut: Rc<RefCell<UDP>>) {
    println!("pp");
   loop {
        println!("=======");
        match  (&mut (*udpfut.borrow_mut())).await {
            Ok(r) => {
                if r.0 == 0 {
                    println!("000000");
                    return ()
                }
            }
            Err(e) => {
                println!("read err:{}", e);
                return ()
            }
        }
   }

}

