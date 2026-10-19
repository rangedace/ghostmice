use std::{io, ptr::null, time::Duration};
use windows_sys::Win32::{
    Foundation::{
        CloseHandle, ERROR_ALREADY_EXISTS, ERROR_FILE_NOT_FOUND, GetLastError, HANDLE,
        SetLastError, WAIT_ABANDONED, WAIT_OBJECT_0, WAIT_TIMEOUT,
    },
    Security::{GetLengthSid, GetTokenInformation, TOKEN_QUERY, TOKEN_USER, TokenUser},
    System::Threading::{
        CreateEventW, CreateMutexW, EVENT_MODIFY_STATE, GetCurrentProcess, OpenEventW, OpenMutexW,
        OpenProcessToken, ReleaseMutex, SYNCHRONIZATION_SYNCHRONIZE, SetEvent, WaitForSingleObject,
    },
    UI::{
        Input::KeyboardAndMouse::{
            GetLastInputInfo, INPUT, INPUT_0, INPUT_MOUSE, LASTINPUTINFO, MOUSEEVENTF_MOVE,
            MOUSEINPUT, SendInput,
        },
        WindowsAndMessaging::GetCursorPos,
    },
};

pub const INPUT_TAG: usize = 0x47484F53;

struct Handle(HANDLE);
impl Drop for Handle {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.0);
        }
    }
}

fn object_name(suffix: &str) -> io::Result<Vec<u16>> {
    use std::fmt::Write;
    let mut token = std::ptr::null_mut();
    if unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) } == 0 {
        return Err(io::Error::last_os_error());
    }
    let token = Handle(token);
    // TOKEN_USER plus the maximum Windows SID (68 bytes), with pointer alignment.
    let mut buffer = [0usize; 32];
    let mut size = 0;
    if unsafe {
        GetTokenInformation(
            token.0,
            TokenUser,
            buffer.as_mut_ptr().cast(),
            size_of_val(&buffer) as u32,
            &mut size,
        )
    } == 0
    {
        return Err(io::Error::last_os_error());
    }
    let sid = unsafe { (*(buffer.as_ptr().cast::<TOKEN_USER>())).User.Sid };
    let bytes = unsafe { std::slice::from_raw_parts(sid.cast::<u8>(), GetLengthSid(sid) as usize) };
    let mut name = String::from("Global\\GhostMice-2DC53565-");
    for byte in bytes {
        write!(name, "{byte:02x}").expect("writing to a String");
    }
    name.push_str(suffix);
    Ok(name.encode_utf16().chain(Some(0)).collect())
}

pub struct Instance(Handle);
impl Instance {
    pub fn acquire() -> io::Result<Option<Self>> {
        // User SID isolates accounts; Global covers all sessions without elevation.
        let name = object_name("-instance")?;
        let handle = unsafe { CreateMutexW(null(), 1, name.as_ptr()) };
        if handle.is_null() {
            return Err(io::Error::last_os_error());
        }
        let exists = unsafe { GetLastError() } == ERROR_ALREADY_EXISTS;
        let handle = Handle(handle);
        if exists {
            Ok(None)
        } else {
            Ok(Some(Self(handle)))
        }
    }
}
impl Drop for Instance {
    fn drop(&mut self) {
        unsafe {
            ReleaseMutex(self.0.0);
        }
    }
}

pub struct StopEvent(Handle);
impl StopEvent {
    pub fn create() -> io::Result<Self> {
        let name = object_name("-stop")?;
        let handle = unsafe { CreateEventW(null(), 1, 0, name.as_ptr()) };
        if handle.is_null() {
            Err(io::Error::last_os_error())
        } else {
            Ok(Self(Handle(handle)))
        }
    }
    pub fn wait(&self, duration: Duration) -> io::Result<bool> {
        let ms = duration
            .as_nanos()
            .div_ceil(1_000_000)
            .min(u32::MAX as u128 - 1) as u32;
        match unsafe { WaitForSingleObject(self.0.0, ms) } {
            WAIT_OBJECT_0 => Ok(true),
            WAIT_TIMEOUT => Ok(false),
            _ => Err(io::Error::last_os_error()),
        }
    }
}

pub fn stop() -> io::Result<()> {
    let name = object_name("-instance")?;
    let mutex = unsafe { OpenMutexW(SYNCHRONIZATION_SYNCHRONIZE, 0, name.as_ptr()) };
    if mutex.is_null() {
        return if unsafe { GetLastError() } == ERROR_FILE_NOT_FOUND {
            Ok(())
        } else {
            Err(io::Error::last_os_error())
        };
    }
    let mutex = Handle(mutex);
    let name = object_name("-stop")?;
    // A concurrent startup may own the mutex before creating the stop event.
    let mut event = std::ptr::null_mut();
    for _ in 0..100 {
        event = unsafe { OpenEventW(EVENT_MODIFY_STATE, 0, name.as_ptr()) };
        if !event.is_null() {
            break;
        }
        if unsafe { WaitForSingleObject(mutex.0, 10) } != WAIT_TIMEOUT {
            return Ok(());
        }
    }
    if event.is_null() {
        return Err(io::Error::other("stop event unavailable"));
    }
    let event = Handle(event);
    if unsafe { SetEvent(event.0) } == 0 {
        return Err(io::Error::last_os_error());
    }
    match unsafe { WaitForSingleObject(mutex.0, 5000) } {
        WAIT_OBJECT_0 | WAIT_ABANDONED => Ok(()),
        _ => Err(io::Error::other("GhostMice did not stop within 5 seconds")),
    }
}

fn zero_input() -> INPUT {
    INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT {
                dx: 0,
                dy: 0,
                mouseData: 0,
                dwFlags: MOUSEEVENTF_MOVE,
                time: 0,
                dwExtraInfo: INPUT_TAG,
            },
        },
    }
}

pub fn send_zero_delta() -> io::Result<()> {
    let input = zero_input();
    unsafe {
        SetLastError(0);
    }
    let count = unsafe { SendInput(1, &input, size_of::<INPUT>() as i32) };
    if count == 1 {
        Ok(())
    } else {
        Err(io::Error::other(format!(
            "SendInput accepted {count}/1 events; Win32={} (UIPI may not set an error)",
            unsafe { GetLastError() }
        )))
    }
}

pub fn cursor() -> io::Result<(i32, i32)> {
    let mut point = windows_sys::Win32::Foundation::POINT { x: 0, y: 0 };
    if unsafe { GetCursorPos(&mut point) } == 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok((point.x, point.y))
    }
}
pub fn last_input() -> io::Result<u32> {
    let mut info = LASTINPUTINFO {
        cbSize: size_of::<LASTINPUTINFO>() as u32,
        dwTime: 0,
    };
    if unsafe { GetLastInputInfo(&mut info) } == 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(info.dwTime)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_zero_relative_movement_no_buttons() {
        let input = zero_input();
        assert_eq!(input.r#type, INPUT_MOUSE);
        let mouse = unsafe { input.Anonymous.mi };
        assert_eq!(
            (mouse.dx, mouse.dy, mouse.mouseData, mouse.time),
            (0, 0, 0, 0)
        );
        assert_eq!(mouse.dwFlags, MOUSEEVENTF_MOVE);
    }
    #[test]
    fn singleton() {
        let first = Instance::acquire()
            .unwrap()
            .expect("stop GhostMice before testing");
        assert!(Instance::acquire().unwrap().is_none());
        drop(first);
        assert!(Instance::acquire().unwrap().is_some());
    }
    #[test]
    #[ignore = "injects one real zero-delta event into the interactive desktop"]
    fn native_zero_delta_probe() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use windows_sys::Win32::UI::WindowsAndMessaging::*;
        static SEEN: AtomicUsize = AtomicUsize::new(0);
        unsafe extern "system" fn hook(code: i32, w: usize, l: isize) -> isize {
            if code >= 0 {
                let data = unsafe { &*(l as *const MSLLHOOKSTRUCT) };
                if data.dwExtraInfo == INPUT_TAG && data.flags & LLMHF_INJECTED != 0 {
                    SEEN.fetch_add(1, Ordering::Relaxed);
                }
            }
            unsafe { CallNextHookEx(std::ptr::null_mut(), code, w, l) }
        }
        let h = unsafe { SetWindowsHookExW(WH_MOUSE_LL, Some(hook), std::ptr::null_mut(), 0) };
        assert!(!h.is_null(), "{}", io::Error::last_os_error());
        let before = cursor().unwrap();
        let idle_before = last_input().unwrap();
        let result = send_zero_delta();
        let start = std::time::Instant::now();
        while start.elapsed() < Duration::from_millis(300) {
            let mut msg = unsafe { std::mem::zeroed() };
            unsafe {
                while PeekMessageW(&mut msg, std::ptr::null_mut(), 0, 0, PM_REMOVE) != 0 {
                    TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        unsafe {
            UnhookWindowsHookEx(h);
        }
        result.unwrap();
        let after = cursor().unwrap();
        println!(
            "hook_events={} before={before:?} after={after:?} last_input_before={idle_before} last_input_after={}",
            SEEN.load(Ordering::Relaxed),
            last_input().unwrap()
        );
        // A zero-delta MOVE may be accepted but omitted from low-level hook delivery.
        // Record zero as a valid experimental result, never as proof of delivery.
        assert_eq!(
            before, after,
            "physical input during test invalidates the stationary check"
        );
    }
}
