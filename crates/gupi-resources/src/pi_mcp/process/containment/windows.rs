use std::{
    mem::size_of,
    os::windows::io::{AsRawHandle as _, FromRawHandle as _, OwnedHandle},
};
use windows_sys::Win32::{
    Foundation::{ERROR_NO_MORE_FILES, GetLastError, INVALID_HANDLE_VALUE},
    System::{
        Diagnostics::ToolHelp::{
            CreateToolhelp32Snapshot, TH32CS_SNAPTHREAD, THREADENTRY32, Thread32First, Thread32Next,
        },
        JobObjects::{
            AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
            JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectExtendedLimitInformation,
            SetInformationJobObject,
        },
        Threading::{
            GetProcessIdOfThread, OpenThread, ResumeThread, THREAD_QUERY_LIMITED_INFORMATION,
            THREAD_SUSPEND_RESUME,
        },
    },
};
pub(super) struct Job(OwnedHandle);
impl Job {
    pub(super) fn new() -> Result<Self, ()> {
        let handle = unsafe { CreateJobObjectW(std::ptr::null(), std::ptr::null()) };
        if handle.is_null() {
            return Err(());
        }
        let job = Self(unsafe { OwnedHandle::from_raw_handle(handle) });
        let mut limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
        limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        if unsafe {
            SetInformationJobObject(
                handle,
                JobObjectExtendedLimitInformation,
                (&limits as *const JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast(),
                size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            )
        } == 0
        {
            return Err(());
        }
        Ok(job)
    }
    pub(super) fn attach_and_resume(&self, child: &tokio::process::Child) -> Result<(), ()> {
        let process = child.raw_handle().ok_or(())?;
        if unsafe { AssignProcessToJobObject(self.0.as_raw_handle(), process) } == 0 {
            return Err(());
        }
        // The child is suspended before any CLI/helper code runs. std/Tokio
        // exposes no primary-thread handle; find that owned thread publicly.
        let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0) };
        if snapshot == INVALID_HANDLE_VALUE {
            return Err(());
        }
        let snapshot = unsafe { OwnedHandle::from_raw_handle(snapshot) };
        let mut entry = THREADENTRY32 {
            dwSize: size_of::<THREADENTRY32>() as u32,
            ..Default::default()
        };
        if unsafe { Thread32First(snapshot.as_raw_handle(), &mut entry) } == 0 {
            return Err(());
        }
        let mut thread = None;
        loop {
            if entry.th32OwnerProcessID == child.id().ok_or(())?
                && thread.replace(entry.th32ThreadID).is_some()
            {
                return Err(());
            }
            if unsafe { Thread32Next(snapshot.as_raw_handle(), &mut entry) } == 0 {
                if unsafe { GetLastError() } != ERROR_NO_MORE_FILES {
                    return Err(());
                }
                break;
            }
        }
        let handle = unsafe {
            OpenThread(
                THREAD_SUSPEND_RESUME | THREAD_QUERY_LIMITED_INFORMATION,
                0,
                thread.ok_or(())?,
            )
        };
        if handle.is_null() {
            return Err(());
        }
        let handle = unsafe { OwnedHandle::from_raw_handle(handle) };
        if unsafe { GetProcessIdOfThread(handle.as_raw_handle()) } != child.id().ok_or(())? {
            return Err(());
        }
        if unsafe { ResumeThread(handle.as_raw_handle()) } != 1 {
            return Err(());
        }
        Ok(())
    }
}
