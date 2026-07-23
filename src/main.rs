#![cfg_attr(windows, feature(windows_process_extensions_raw_attribute))]

use std::{env, fs, io};
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
use std::os::windows::process::{CommandExt, ProcThreadAttributeList};
use std::path::Path;
use std::process::{Child, Command};
use windows::Win32::Foundation::HANDLE;
use windows::Win32::System::Threading::{OpenProcess, PROCESS_ALL_ACCESS};

const PROC_THREAD_ATTRIBUTE_PARENT_PROCESS: usize = 0x00020000;

fn open_parent_process(pid: u32) -> io::Result<OwnedHandle> {
    let handle: HANDLE = unsafe { OpenProcess(PROCESS_ALL_ACCESS, false, pid)? };
    let handle = unsafe { OwnedHandle::from_raw_handle(handle.0) };
    Ok(handle)
}

fn launch_child_process(child: &mut Command, parent: &OwnedHandle) -> io::Result<Child> {
    let parent_handle = parent.as_raw_handle();
    let attribute_list = ProcThreadAttributeList::build()
        .attribute(
            PROC_THREAD_ATTRIBUTE_PARENT_PROCESS,
            &parent_handle,
        )
        .finish()
        .unwrap();

    let child_process = child.spawn_with_attributes(&attribute_list)?;
    Ok(child_process)
}

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();
    let proc_name = Path::new(&args[0])
        .file_name()
        .unwrap()
        .to_str()
        .unwrap();

    if args.len() < 3 {
        println!("Usage: {} <parent pid> <child program> [<child program arg> ...]", proc_name);
        return Ok(());
    }

    let ppid: u32 = args[1].parse().unwrap();
    let parent = open_parent_process(ppid)?;

    let mut child = Command::new(fs::canonicalize(&args[2])?);
    args[3..].iter().for_each(|arg| {
        child.raw_arg(arg);
    });

    launch_child_process(&mut child, &parent)?;
    Ok(())
}
