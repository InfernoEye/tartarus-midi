//! Windows 11 Tartarus-specific D-pad input via Raw Input (WM_INPUT).
//!
//! Unlike GetAsyncKeyState or a global keyboard hook, this identifies the
//! hardware source of each arrow event before changing the MIDI transpose.
//! Raw Input is OBSERVE-ONLY: it cannot suppress the original arrow key in
//! other Windows applications. Interception/kernel filtering would be needed
//! for reliable suppression; this module deliberately doesn't require it.
//!
//! This is a small Win32 FFI implementation, with no additional Rust crate.

use std::collections::HashMap;
use std::ffi::c_void;
use std::mem::{size_of, zeroed};
use std::ptr::{null, null_mut};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Left,
    Right,
    Up,
    Down,
}

impl Direction {
    pub fn name(self) -> &'static str {
        match self {
            Direction::Left => "LEWO",
            Direction::Right => "PRAWO",
            Direction::Up => "GORA",
            Direction::Down => "DOL",
        }
    }

    fn index(self) -> usize {
        match self {
            Direction::Left => 0,
            Direction::Right => 1,
            Direction::Up => 2,
            Direction::Down => 3,
        }
    }
}

type WinHandle = *mut c_void;

const WM_INPUT: u32 = 0x00FF;
const WM_INPUT_DEVICE_CHANGE: u32 = 0x00FE;
const RID_INPUT: u32 = 0x10000003;
const RIDI_DEVICENAME: u32 = 0x20000007;
const RIDEV_INPUTSINK: u32 = 0x00000100;
const RIDEV_DEVNOTIFY: u32 = 0x00002000;
const RIM_TYPEKEYBOARD: u32 = 1;
const RI_KEY_BREAK: u16 = 0x0001;
const VK_LEFT: u16 = 0x25;
const VK_UP: u16 = 0x26;
const VK_RIGHT: u16 = 0x27;
const VK_DOWN: u16 = 0x28;

#[repr(C)]
struct WinPoint {
    x: i32,
    y: i32,
}

#[repr(C)]
struct WinMsg {
    hwnd: WinHandle,
    message: u32,
    wparam: usize,
    lparam: isize,
    time: u32,
    point: WinPoint,
    lprivate: u32,
}

type WindowProc = unsafe extern "system" fn(WinHandle, u32, usize, isize) -> isize;

#[repr(C)]
struct WinClass {
    style: u32,
    wndproc: Option<WindowProc>,
    cls_extra: i32,
    wnd_extra: i32,
    instance: WinHandle,
    icon: WinHandle,
    cursor: WinHandle,
    background: WinHandle,
    menu_name: *const u16,
    class_name: *const u16,
}

#[repr(C)]
struct RawInputDevice {
    usage_page: u16,
    usage: u16,
    flags: u32,
    target_window: WinHandle,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct RawInputHeader {
    kind: u32,
    total_size: u32,
    source_device: WinHandle,
    wparam: usize,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct RawKeyboard {
    make_code: u16,
    flags: u16,
    reserved: u16,
    vkey: u16,
    message: u32,
    extra_info: u32,
}

#[link(name = "user32")]
extern "system" {
    fn DefWindowProcW(hwnd: WinHandle, msg: u32, wparam: usize, lparam: isize) -> isize;
    fn RegisterClassW(class: *const WinClass) -> u16;
    fn CreateWindowExW(
        extended_style: u32,
        class_name: *const u16,
        window_name: *const u16,
        style: u32,
        x: i32, y: i32, width: i32, height: i32,
        parent: WinHandle, menu: WinHandle,
        instance: WinHandle, create_param: WinHandle,
    ) -> WinHandle;
    fn DestroyWindow(hwnd: WinHandle) -> i32;
    fn RegisterRawInputDevices(devices: *const RawInputDevice, count: u32, size: u32) -> i32;
    fn GetRawInputData(
        raw_input: WinHandle, command: u32, buffer: *mut c_void,
        size: *mut u32, header_size: u32,
    ) -> u32;
    fn GetRawInputDeviceInfoW(
        device: WinHandle, command: u32,
        buffer: *mut c_void, size: *mut u32,
    ) -> u32;
    fn GetMessageW(msg: *mut WinMsg, window: WinHandle, min: u32, max: u32) -> i32;
    fn TranslateMessage(msg: *const WinMsg) -> i32;
    fn DispatchMessageW(msg: *const WinMsg) -> isize;
}

#[link(name = "kernel32")]
extern "system" {
    fn GetModuleHandleW(module_name: *const u16) -> WinHandle;
}

unsafe extern "system" fn window_proc(
    hwnd: WinHandle, msg: u32, wparam: usize, lparam: isize,
) -> isize {
    // All messages including WM_INPUT go through DefWindowProcW so Windows
    // can perform the documented WM_INPUT cleanup.
    DefWindowProcW(hwnd, msg, wparam, lparam)
}

fn utf16z(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(Some(0)).collect()
}

fn device_is_tartarus(device: WinHandle, cache: &mut HashMap<usize, bool>) -> bool {
    let id = device as usize;
    if id == 0 { return false; }
    if let Some(found) = cache.get(&id) { return *found; }

    // RIDI_DEVICENAME reports the device interface path, e.g.:
    // \\?\HID#VID_1532&PID_0244&MI_00#...
    let mut length: u32 = 0;
    let size_result = unsafe {
        GetRawInputDeviceInfoW(device, RIDI_DEVICENAME, null_mut(), &mut length)
    };
    if size_result == u32::MAX || length == 0 || length > 4096 { return false; }
    let mut name = vec![0u16; length as usize];
    let copied = unsafe {
        GetRawInputDeviceInfoW(device, RIDI_DEVICENAME,
            name.as_mut_ptr().cast(), &mut length)
    };
    if copied == u32::MAX { return false; }
    let actual = (copied as usize).min(name.len());
    let s = String::from_utf16_lossy(&name[..actual]).to_ascii_uppercase();
    let found = s.contains("VID_1532") && s.contains("PID_0244");
    cache.insert(id, found);
    if found {
        println!("D-pad Raw Input: rozpoznano klawiaturę Tartarus Pro (VID_1532 PID_0244)");
    }
    found
}

fn direction_for_vkey(vkey: u16) -> Option<Direction> {
    match vkey {
        VK_LEFT => Some(Direction::Left),
        VK_RIGHT => Some(Direction::Right),
        VK_UP => Some(Direction::Up),
        VK_DOWN => Some(Direction::Down),
        _ => None,
    }
}

fn handle_input(
    raw: WinHandle,
    cache: &mut HashMap<usize, bool>,
    down: &mut [bool; 4],
    tx: &Sender<Direction>,
) {
    let mut data_size = 0u32;
    // GetRawInputData returns bytes; allocate u64s for 8-byte alignment.
    let probe = unsafe {
        GetRawInputData(raw, RID_INPUT, null_mut(),
            &mut data_size, size_of::<RawInputHeader>() as u32)
    };
    if probe == u32::MAX || data_size < (size_of::<RawInputHeader>() + size_of::<RawKeyboard>()) as u32 {
        return;
    }
    // Ignore unusually large input messages; a keyboard record is small.
    if data_size > 4096 { return; }
    let mut storage = vec![0u64; (data_size as usize + 7) / 8];
    let actual = unsafe {
        GetRawInputData(raw, RID_INPUT,
            storage.as_mut_ptr().cast(),
            &mut data_size, size_of::<RawInputHeader>() as u32)
    };
    if actual == u32::MAX || actual < (size_of::<RawInputHeader>() + size_of::<RawKeyboard>()) as u32 {
        return;
    }

    // SAFETY: storage is 8-byte aligned; size was checked above. The
    // RAWINPUT header is followed by its raw keyboard union payload.
    let header = unsafe { (storage.as_ptr() as *const RawInputHeader).read() };
    if header.kind != RIM_TYPEKEYBOARD || !device_is_tartarus(header.source_device, cache) {
        return;
    }
    let keyboard = unsafe {
        (storage.as_ptr().cast::<u8>().wrapping_add(size_of::<RawInputHeader>())
            as *const RawKeyboard).read_unaligned()
    };
    let Some(direction) = direction_for_vkey(keyboard.vkey) else { return; };
    let index = direction.index();
    let is_release = (keyboard.flags & RI_KEY_BREAK) != 0;

    if is_release {
        down[index] = false;
    } else if !down[index] {
        down[index] = true;
        let _ = tx.send(direction);
    }
    // Auto-repeat events are ignored until a real key release is received.
}

fn listener(tx: Sender<Direction>) -> Result<(), Box<dyn std::error::Error>> {
    // Raw Input messages require a window and a running Windows message loop.
    // The window is never shown; only its OS message queue is used.
    let class_name = utf16z("TartarusMidiV2RawInputSink");
    let instance = unsafe { GetModuleHandleW(null()) };
    if instance.is_null() {
        return Err(std::io::Error::last_os_error().into());
    }
    let class = WinClass {
        style: 0, wndproc: Some(window_proc), cls_extra: 0, wnd_extra: 0,
        instance, icon: null_mut(), cursor: null_mut(), background: null_mut(),
        menu_name: null(), class_name: class_name.as_ptr(),
    };
    if unsafe { RegisterClassW(&class) } == 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    let hwnd = unsafe {
        CreateWindowExW(0, class_name.as_ptr(), class_name.as_ptr(),
            0, 0, 0, 0, 0, null_mut(), null_mut(), instance, null_mut())
    };
    if hwnd.is_null() {
        return Err(std::io::Error::last_os_error().into());
    }
    let device = RawInputDevice {
        usage_page: 0x01, usage: 0x06, // generic desktop / keyboard
        flags: RIDEV_INPUTSINK | RIDEV_DEVNOTIFY,
        target_window: hwnd,
    };
    if unsafe { RegisterRawInputDevices(&device, 1, size_of::<RawInputDevice>() as u32) } == 0 {
        let err = std::io::Error::last_os_error();
        unsafe { DestroyWindow(hwnd); }
        return Err(err.into());
    }
    println!("D-pad: Raw Input aktywny (rozpoznawane są tylko strzałki Tartarusa).");
    let mut cache = HashMap::<usize, bool>::new();
    let mut down = [false; 4];
    loop {
        let mut msg: WinMsg = unsafe { zeroed() };
        let rc = unsafe { GetMessageW(&mut msg, null_mut(), 0, 0) };
        if rc <= 0 { break; }
        match msg.message {
            WM_INPUT => handle_input(
                msg.lparam as WinHandle, &mut cache, &mut down, &tx,
            ),
            WM_INPUT_DEVICE_CHANGE => {
                cache.clear();
                down = [false; 4];
            }
            _ => (),
        }
        unsafe {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
    unsafe { DestroyWindow(hwnd); }
    Ok(())
}

pub fn start_listener() -> Receiver<Direction> {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        if let Err(err) = listener(tx) {
            eprintln!("D-pad Raw Input niedostępny: {err} (nuty i Aftertouch nadal działają)");
        }
    });
    rx
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn arrow_keys_only() {
        assert_eq!(direction_for_vkey(0x25), Some(Direction::Left));
        assert_eq!(direction_for_vkey(0x26), Some(Direction::Up));
        assert_eq!(direction_for_vkey(0x27), Some(Direction::Right));
        assert_eq!(direction_for_vkey(0x28), Some(Direction::Down));
        assert_eq!(direction_for_vkey(0x41), None);
    }
}