use std::mem::{size_of, zeroed};

use windows::{
    Win32::{
        Foundation::{CloseHandle, ERROR_INSUFFICIENT_BUFFER, HANDLE},
        System::{
            Memory::{GetProcessHeap, HEAP_NONE, HEAP_ZERO_MEMORY, HeapAlloc, HeapFree},
            Threading::{
                CREATE_UNICODE_ENVIRONMENT, CreateProcessW, DeleteProcThreadAttributeList,
                EXTENDED_STARTUPINFO_PRESENT, InitializeProcThreadAttributeList,
                LPPROC_THREAD_ATTRIBUTE_LIST, OpenProcess, PROC_THREAD_ATTRIBUTE_PARENT_PROCESS,
                PROCESS_ALL_ACCESS, PROCESS_INFORMATION, STARTF_USESHOWWINDOW, STARTUPINFOEXW,
                UpdateProcThreadAttribute,
            },
        },
        UI::WindowsAndMessaging::SW_SHOW,
    },
    core::{Error, PWSTR},
};

pub struct ScopedHandle(HANDLE);

impl Drop for ScopedHandle {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseHandle(self.0);
        }
    }
}

pub fn create_process_with_handle(
    parent: ScopedHandle,
    args: &[String],
) -> Result<u32, windows::core::Error> {
    let mut si: STARTUPINFOEXW = unsafe { zeroed() };
    let mut pi: PROCESS_INFORMATION = unsafe { zeroed() };
    let mut size: usize = 0x30;

    loop {
        if size > 1024 {
            return Err(windows::core::Error::from_thread());
        }

        si.StartupInfo.cb = size_of::<STARTUPINFOEXW>() as u32;
        si.lpAttributeList = LPPROC_THREAD_ATTRIBUTE_LIST(unsafe {
            HeapAlloc(GetProcessHeap().unwrap(), HEAP_ZERO_MEMORY, size)
        });

        if si.lpAttributeList.is_invalid() {
            return Err(windows::core::Error::from_thread());
        }
        let ret = match unsafe {
            InitializeProcThreadAttributeList(Some(si.lpAttributeList), 1, Some(0), &mut size)
        } {
            Ok(()) => {
                unsafe {
                    UpdateProcThreadAttribute(
                        si.lpAttributeList,
                        0,
                        PROC_THREAD_ATTRIBUTE_PARENT_PROCESS as usize,
                        Some(&parent.0 as *const _ as *mut _),
                        size_of::<HANDLE>(),
                        None,
                        None,
                    )?;
                }

                si.StartupInfo.dwFlags = STARTF_USESHOWWINDOW;
                si.StartupInfo.wShowWindow = SW_SHOW.0 as _;

                let mut cmdline: Vec<_> = args.join(" ").encode_utf16().collect();
                cmdline.push(0x0);

                // println!("CMD len {}", cmdline.len());

                unsafe {
                    CreateProcessW(
                        None,
                        Some(PWSTR::from_raw(cmdline.as_mut_ptr())),
                        None,
                        None,
                        false,
                        CREATE_UNICODE_ENVIRONMENT | EXTENDED_STARTUPINFO_PRESENT,
                        None,
                        None,
                        &si.StartupInfo,
                        &mut pi,
                    )?;
                }

                let _ = ScopedHandle(pi.hThread);
                let _ = ScopedHandle(pi.hProcess);

                Ok(pi.dwProcessId)
            }
            // Err(windows::core::Error::from(ERROR_INSUFFICIENT_BUFFER)) => {}
            Err(e) => {
                if e != windows::core::Error::from(ERROR_INSUFFICIENT_BUFFER) {
                    Err(e)
                } else {
                    Ok(0)
                }
            }
        };

        if !si.lpAttributeList.is_invalid() {
            unsafe {
                DeleteProcThreadAttributeList(si.lpAttributeList);
            }
        }
        unsafe {
            HeapFree(
                GetProcessHeap().unwrap(),
                HEAP_NONE,
                Some(si.lpAttributeList.0),
            )?;
        }

        match ret {
            Ok(0) => {
                continue;
            }
            Ok(pid) => {
                return Ok(pid);
            }
            Err(e) => {
                return Err(e);
            }
        }
    }
}

pub fn open_parent_process(ppid: u32) -> Result<ScopedHandle, Error> {
    let handle = unsafe { OpenProcess(PROCESS_ALL_ACCESS, false, ppid)? };
    let handle = ScopedHandle(handle);
    return Ok(handle);
}

fn main() -> Result<(), Error> {
    // parse args
    let args: Vec<String> = std::env::args().collect();
    let procname = std::path::Path::new(args[0].as_str())
        .file_name()
        .unwrap()
        .to_str()
        .unwrap();

    if args.len() < 3 {
        println!("Usage: {} <ppid> <commandline>", procname);
        return Ok(());
    }

    let ppid: u32 = args[1].parse().unwrap();

    // parent process spoofing
    let parent: ScopedHandle = open_parent_process(ppid)?;
    create_process_with_handle(parent, &args[2..])?;

    Ok(())
}
