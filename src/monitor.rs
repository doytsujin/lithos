use std::rc::Rc;
use std::collections::TreeMap;
use std::collections::HashMap;
use std::collections::PriorityQueue;
use std::mem::swap;
use std::time::Duration;
use libc::pid_t;
use time::{Timespec, get_time};

use super::container::Command;
use super::signal;

pub enum MonitorResult {
    Killed,
    Reboot,
}

pub trait Executor {
    fn command(&self) -> Command;
}

pub struct Process<'a> {
    name: Rc<String>,
    current_pid: Option<pid_t>,
    start_time: Option<Timespec>,
    restart_timeout: Duration,
    executor: Box<Executor + 'a>,
}

pub struct Monitor<'a> {
    myname: String,
    processes: TreeMap<Rc<String>, Process<'a>>,
    start_queue: PriorityQueue<(i64, Rc<String>)>,
    pids: HashMap<pid_t, Rc<String>>,
    allow_reboot: bool,
}

fn _top_time(pq: &PriorityQueue<(i64, Rc<String>)>) -> Option<Timespec> {
    return pq.top().map(|&(ts, _)| Timespec::new(-ts, 0));
}

impl<'a> Monitor<'a> {
    pub fn new<'x>(name: String) -> Monitor<'x> {
        return Monitor {
            myname: name,
            processes: TreeMap::new(),
            pids: HashMap::new(),
            allow_reboot: false,
            start_queue: PriorityQueue::new(),
        };
    }
    pub fn allow_reboot(&mut self) {
        self.allow_reboot = true;
    }
    pub fn add(&mut self, name: Rc<String>, executor: Box<Executor>,
        timeout: Duration, current: Option<(pid_t, Timespec)>)
    {
        if current.is_some() {
            info!("[{:s}] Registered process pid: {} as name: {}",
                self.myname, current.map(|(pid, _)| pid).unwrap(), name);
        } else {
            self.start_queue.push((0, name.clone()));
        }
        self.processes.insert(name.clone(), Process {
            name: name,
            current_pid: current.map(|(pid, _)| pid),
            start_time: current.map(|(_, time)| time),
            restart_timeout: timeout,
            executor: executor});
    }
    pub fn has(&self, name: &Rc<String>) -> bool {
        return self.processes.contains_key(name);
    }
    fn _wait_signal(&self) -> signal::Signal {
        return signal::wait_next(
            self.allow_reboot,
            _top_time(&self.start_queue));
    }
    fn _start_processes(&mut self) {
        let time = get_time();
        loop {
            let name = match self.start_queue.top() {
                Some(&(ref ptime, ref name)) if Timespec::new(-ptime, 0) < time
                => name.clone(),
                _ => { break; }
            };
            self.start_queue.pop();
            let ref mut prc = self.processes.find_mut(&name).unwrap();
            match prc.executor.command().spawn() {
                Ok(pid) => {
                    info!("[{:s}] Process {} started with pid {}",
                        self.myname, prc.name, pid);
                    prc.current_pid = Some(pid);
                    prc.start_time = Some(get_time());
                    self.pids.insert(pid, prc.name.clone());
                }
                Err(e) => {
                    error!("Can't run container {}: {}", prc.name, e);
                    self.start_queue.push((
                        -(time + prc.restart_timeout).sec,
                        name,
                        ));
                }
            }
        }
    }
    pub fn run(&mut self) -> MonitorResult {
        debug!("[{:s}] Starting with {} processes",
            self.myname, self.processes.len());
        // Main loop
        loop {
            let sig = self._wait_signal();
            info!("[{:s}] Got signal {}", self.myname, sig);
            match sig {
                signal::Timeout => {
                    self._start_processes();
                }
                signal::Terminate(sig) => {
                    for (_name, prc) in self.processes.iter() {
                        match prc.current_pid {
                            Some(pid) => signal::send_signal(pid, sig),
                            None => {}
                        }
                    }
                    break;
                }
                signal::Child(pid, status) => {
                    let prc = match self.pids.find(&pid) {
                        Some(name) => &self.processes[*name],
                        None => {
                            warn!("[{:s}] Unknown process {} dead with {}",
                                self.myname, pid, status);
                            continue;
                        },
                    };
                    warn!("[{:s}] Child {}:{} exited with status {}",
                        self.myname, prc.name, pid, status);
                    self.start_queue.push((
                        -(prc.start_time.unwrap() + prc.restart_timeout).sec,
                        prc.name.clone(),
                        ));
                }
                signal::Reboot => {
                    return Reboot;
                }
            }
        }
        self.start_queue.clear();
        info!("[{:s}] Shutting down", self.myname);
        // Shut down loop
        let mut processes = TreeMap::new();
        swap(&mut processes, &mut self.processes);
        let mut left: TreeMap<pid_t, Process> = processes.into_iter()
            .filter(|&(_, ref prc)| prc.current_pid.is_some())
            .map(|(_, prc)| (prc.current_pid.unwrap(), prc))
            .collect();
        while left.len() > 0 {
            let sig = self._wait_signal();
            info!("[{:s}] Got signal {}", self.myname, sig);
            match sig {
                signal::Timeout => { unreachable!(); }
                signal::Terminate(sig) => {
                    for (_name, prc) in left.iter() {
                        match prc.current_pid {
                            Some(pid) => signal::send_signal(pid, sig),
                            None => {}
                        }
                    }
                }
                signal::Child(pid, status) => {
                    match left.pop(&pid) {
                        Some(prc) => {
                            info!("[{:s}] Child {}:{} exited with status {}",
                                self.myname, prc.name, pid, status);
                        }
                        None => {
                            warn!("[{:s}] Unknown process {} dead with {}",
                                self.myname, pid, status);
                        }
                    }
                }
                signal::Reboot => {
                    return Reboot;
                }
            }
        }
        return Killed;
    }
}
