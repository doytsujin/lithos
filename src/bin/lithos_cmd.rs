#![feature(phase, macro_rules)]

extern crate serialize;
extern crate libc;
#[phase(plugin, link)] extern crate log;

extern crate argparse;
extern crate quire;
#[phase(plugin, link)] extern crate lithos;
#[phase(plugin)] extern crate regex_macros;
extern crate regex;


use std::rc::Rc;
use std::os::{set_exit_status, getenv};
use std::io::stderr;
use std::time::Duration;
use std::default::Default;
use libc::funcs::posix88::unistd::getpid;

use argparse::{ArgumentParser, Store, List};
use quire::parse_config;

use lithos::tree_config::TreeConfig;
use lithos::container_config::{ContainerConfig, Command};
use lithos::child_config::ChildConfig;
use lithos::container::{Command};
use lithos::monitor::{Monitor, Executor};
use lithos::signal;
use lithos::setup::{read_local_config, setup_filesystem, prepare_state_dir};


struct Target {
    name: Rc<String>,
    global: TreeConfig,
    local: ContainerConfig,
    args: Vec<String>,
}

impl Executor for Target {
    fn command(&self) -> Command
    {
        let mut cmd = Command::new((*self.name).clone(),
            self.local.executable.as_slice());
        cmd.set_user_id(self.local.user_id);
        cmd.chroot(&self.global.mount_dir);
        cmd.set_workdir(&self.local.workdir);

        // Should we propagate TERM?
        cmd.set_env("TERM".to_string(),
                    getenv("TERM").unwrap_or("dumb".to_string()));
        cmd.update_env(self.local.environ.iter());
        cmd.set_env("LITHOS_COMMAND".to_string(), (*self.name).clone());

        cmd.args(self.local.arguments.as_slice());
        cmd.args(self.args.as_slice());

        return cmd;
    }
    fn finish(&self) -> bool {
        return false;  // Do not restart
    }
}

fn run(global_cfg: Path, name: String, args: Vec<String>)
    -> Result<(), String>
{
    let global: TreeConfig = try_str!(parse_config(&global_cfg,
        &*TreeConfig::validator(), Default::default()));

    assert!(regex!("^[a-zA-Z0-9][a-zA-Z0-9_.-]+$").is_match(name.as_slice()));
    let child_fn = global.config_dir.join(name + ".yaml".to_string());
    let child_cfg: ChildConfig = try_str!(parse_config(&child_fn,
        &*ChildConfig::validator(), Default::default()));

    if child_cfg.kind != Command {
        return Err(format!("The target container is: {}", child_cfg.kind));
    }

    // TODO(tailhook) clarify it: root is mounted in read_local_config
    let local: ContainerConfig = try!(read_local_config(
        &global, &child_cfg));

    info!("[{:s}] Running command with args {}", name, args);

    let state_dir = &global.state_dir.join(
        format!(".cmd.{}.{}", name, unsafe { getpid() }));
    try!(prepare_state_dir(state_dir, &global, &local));
    try!(setup_filesystem(&global, &local, state_dir));

    let mut mon = Monitor::new(name.clone());
    let name = Rc::new(name + ".cmd");
    let timeo = Duration::milliseconds(0);
    mon.add(name.clone(), box Target {
        name: name,
        global: global,
        local: local,
        args: args,
    }, timeo, None);
    mon.run();

    return Ok(());
}

fn main() {

    signal::block_all();

    let mut global_config = Path::new("/etc/lithos.yaml");
    let mut command_name = "".to_string();
    let mut args = vec!();
    {
        let mut ap = ArgumentParser::new();
        ap.set_description("Runs tree of processes");
        ap.refer(&mut global_config)
          .add_option(["--global-config"], box Store::<Path>,
            "Name of the global configuration file (default /etc/lithos.yaml)")
          .metavar("FILE");
        ap.refer(&mut command_name)
          .add_argument("name", box Store::<String>,
            "Name of the command to run")
          .required();
        ap.refer(&mut args)
          .add_argument("argument", box List::<String>,
            "Arguments for the command");
        match ap.parse_args() {
            Ok(()) => {}
            Err(x) => {
                set_exit_status(x);
                return;
            }
        }
    }
    match run(global_config, command_name, args) {
        Ok(()) => {
            set_exit_status(0);
        }
        Err(e) => {
            (write!(stderr(), "Fatal error: {}\n", e)).ok();
            set_exit_status(1);
        }
    }
}
