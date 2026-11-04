use crate::achievements::AchievementNotification;
use crate::config::{self, load_config};
use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::ptr::{null, null_mut};
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex};
use std::thread;
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::Shell::{
    Shell_NotifyIconW, NIF_ICON, NIF_INFO, NIF_MESSAGE, NIF_TIP, NIIF_INFO, NIM_ADD, NIM_DELETE,
    NIM_MODIFY, NOTIFYICONDATAW,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, CreateWindowExW, DefWindowProcW, DestroyMenu, DispatchMessageW,
    GetCursorPos, GetMessageW, LoadIconW, PostQuitMessage, RegisterClassW, SetForegroundWindow,
    TrackPopupMenu, TranslateMessage, IDI_APPLICATION, MF_GRAYED, MF_SEPARATOR, MF_STRING,
    TPM_BOTTOMALIGN, TPM_RIGHTBUTTON, WM_APP, WM_COMMAND, WM_DESTROY, WM_LBUTTONUP, WM_RBUTTONUP,
    WNDCLASSW, WS_OVERLAPPEDWINDOW,
};

#[repr(C)]
struct IconInfo {
    f_icon: i32,
    x_hotspot: u32,
    y_hotspot: u32,
    hbm_mask: isize,
    hbm_color: isize,
}

#[link(name = "user32")]
extern "system" {
    fn CreateIconIndirect(piconinfo: *const IconInfo) -> isize;
    fn DestroyIcon(hicon: isize) -> i32;
}

#[link(name = "gdi32")]
extern "system" {
    fn CreateBitmap(
        nWidth: i32,
        nHeight: i32,
        nPlanes: u32,
        nBitCount: u32,
        lpBits: *const core::ffi::c_void,
    ) -> isize;
    fn DeleteObject(ho: isize) -> i32;
}

pub const WM_TRAY_CALLBACK: u32 = WM_APP + 1;

pub const ID_STATUS: usize = 1001;
pub const ID_OPEN_DASHBOARD: usize = 1002;
pub const ID_BACKUP_ALL: usize = 1003;
pub const ID_EXIT: usize = 1004;

pub fn to_wide_chars(s: &str) -> Vec<u16> {
    OsStr::new(s).encode_wide().chain(Some(0)).collect()
}

/// Renders the Sager badge icon (rounded square, gold star emblem) as a sharp 32x32 tray icon.
fn create_sager_icon() -> isize {
    unsafe {
        let mut pixels = [0u32; 32 * 32];

        for y in 0..32 {
            for x in 0..32 {
                let dx = (x as f32 - 15.5).abs();
                let dy = (y as f32 - 15.5).abs();
                let dist = (dx.powi(4) + dy.powi(4)).powf(0.25);

                if dist > 14.5 {
                    pixels[y * 32 + x] = 0; // Transparent outside the badge
                    continue;
                }

                // Outer ring + star + body colors
                let mut is_star = false;
                if y >= 16 && y <= 23 && x >= 12 && x <= 19 {
                    let sx = x as f32 - 15.5;
                    let sy = y as f32 - 19.5;
                    let sr = (sx * sx + sy * sy).sqrt();
                    let angle = sy.atan2(sx);
                    let star_radius =
                        1.3 + 2.0 * (5.0 * angle - std::f32::consts::FRAC_PI_2).cos().max(0.0);
                    if sr <= star_radius {
                        is_star = true;
                    }
                }

                if is_star {
                    // Gold star
                    pixels[y * 32 + x] = pack(251, 191, 36, 255);
                } else if dist >= 13.0 {
                    // Ring
                    pixels[y * 32 + x] = pack(245, 250, 255, 255);
                } else {
                    // Dark navy body
                    pixels[y * 32 + x] = pack(15, 23, 42, 245);
                }
            }
        }

        let hbm_color = CreateBitmap(32, 32, 1, 32, pixels.as_ptr() as *const _);
        let mask = [0u8; 128];
        let hbm_mask = CreateBitmap(32, 32, 1, 1, mask.as_ptr() as *const _);

        if hbm_color == 0 || hbm_mask == 0 {
            if hbm_color != 0 {
                DeleteObject(hbm_color);
            }
            if hbm_mask != 0 {
                DeleteObject(hbm_mask);
            }
            return 0;
        }

        let icon_info = IconInfo {
            f_icon: 1,
            x_hotspot: 0,
            y_hotspot: 0,
            hbm_mask,
            hbm_color,
        };

        let hicon = CreateIconIndirect(&icon_info);
        DeleteObject(hbm_color);
        DeleteObject(hbm_mask);

        hicon
    }
}

fn pack(r: u8, g: u8, b: u8, a: u32) -> u32 {
    (a << 24) | ((r as u32) << 16) | ((g as u32) << 8) | (b as u32)
}

#[derive(Clone, Copy)]
pub struct TrayHandle(pub isize);
unsafe impl Send for TrayHandle {}
unsafe impl Sync for TrayHandle {}

impl TrayHandle {
    pub fn show_balloon_notification(&self, title: &str, message: &str, sound: bool) {
        unsafe {
            let hwnd = self.0 as HWND;
            let mut nid = std::mem::zeroed::<NOTIFYICONDATAW>();
            nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
            nid.hWnd = hwnd;
            nid.uID = 1;
            nid.uFlags = NIF_INFO;
            nid.dwInfoFlags = NIIF_INFO;

            let title_wide = to_wide_chars(title);
            let t_len = title_wide.len().min(nid.szInfoTitle.len() - 1);
            nid.szInfoTitle[..t_len].copy_from_slice(&title_wide[..t_len]);

            let msg_wide = to_wide_chars(message);
            let m_len = msg_wide.len().min(nid.szInfo.len() - 1);
            nid.szInfo[..m_len].copy_from_slice(&msg_wide[..m_len]);

            Shell_NotifyIconW(NIM_MODIFY, &nid);

            if sound {
                crate::audio::play_steam_achievement_sound();
            }
        }
    }

    pub fn remove_icon(&self) {
        unsafe {
            let hwnd = self.0 as HWND;
            let mut nid = std::mem::zeroed::<NOTIFYICONDATAW>();
            nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
            nid.hWnd = hwnd;
            nid.uID = 1;
            Shell_NotifyIconW(NIM_DELETE, &nid);
        }
    }
}

pub struct SystemTray {
    pub handle: TrayHandle,
    pub icon_handle: isize,
}

impl SystemTray {
    pub fn new(port: u16) -> Result<Self, String> {
        unsafe {
            let class_name = to_wide_chars("SagerTrayClass");
            let hinstance = GetModuleHandleW(null());

            let s_icon = create_sager_icon();
            let icon = if s_icon != 0 {
                s_icon as windows_sys::Win32::UI::WindowsAndMessaging::HICON
            } else {
                LoadIconW(null_mut(), IDI_APPLICATION)
            };

            let wc = WNDCLASSW {
                style: 0,
                lpfnWndProc: Some(tray_window_proc),
                cbClsExtra: 0,
                cbWndExtra: 0,
                hInstance: hinstance,
                hIcon: icon,
                hCursor: null_mut(),
                hbrBackground: null_mut(),
                lpszMenuName: null(),
                lpszClassName: class_name.as_ptr(),
            };

            let _ = RegisterClassW(&wc);

            let window_title = to_wide_chars("Sager Game Companion");
            let hwnd = CreateWindowExW(
                0,
                class_name.as_ptr(),
                window_title.as_ptr(),
                WS_OVERLAPPEDWINDOW,
                0,
                0,
                0,
                0,
                null_mut(),
                null_mut(),
                hinstance,
                null(),
            );

            if hwnd.is_null() {
                return Err("Failed to create tray window".to_string());
            }

            let mut nid = std::mem::zeroed::<NOTIFYICONDATAW>();
            nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
            nid.hWnd = hwnd;
            nid.uID = 1;
            nid.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
            nid.uCallbackMessage = WM_TRAY_CALLBACK;
            nid.hIcon = icon;

            let tip = to_wide_chars("Sager");
            let len = tip.len().min(nid.szTip.len() - 1);
            nid.szTip[..len].copy_from_slice(&tip[..len]);

            if Shell_NotifyIconW(NIM_ADD, &nid) == 0 {
                return Err("Failed to add system tray icon".to_string());
            }

            // Remember the dashboard URL for the click handlers
            DASHBOARD_URL.with(|cell| *cell.borrow_mut() = format!("http://127.0.0.1:{}", port));

            Ok(Self {
                handle: TrayHandle(hwnd as isize),
                icon_handle: s_icon,
            })
        }
    }

    pub fn handle(&self) -> TrayHandle {
        self.handle
    }

    pub fn run_message_loop(&self) {
        unsafe {
            let mut msg = std::mem::zeroed();
            while GetMessageW(&mut msg, null_mut(), 0, 0) != 0 {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
            if self.icon_handle != 0 {
                DestroyIcon(self.icon_handle);
            }
        }
    }
}

// The tray message loop runs on the thread that created SystemTray, so a
// thread-local is the simplest way to share the dashboard URL with the handler.
thread_local! {
    static DASHBOARD_URL: std::cell::RefCell<String> =
        std::cell::RefCell::new("http://127.0.0.1:3939".to_string());
}

pub fn open_dashboard() {
    DASHBOARD_URL.with(|c| {
        let url = c.borrow().clone();
        let _ = open::that(&url);
    });
}

pub fn start_notification_listener(
    tray_handle: TrayHandle,
    rx: Receiver<AchievementNotification>,
    config_provider: Arc<Mutex<config::SagerConfig>>,
) {
    thread::spawn(move || {
        while let Ok(notif) = rx.recv() {
            let (notifications_enabled, sound_enabled) = {
                let cfg = config_provider.lock().unwrap();
                (cfg.notifications_enabled, cfg.sound_enabled)
            };

            if sound_enabled {
                crate::audio::play_steam_achievement_sound();
            }

            if notifications_enabled {
                crate::overlay::show_steam_overlay(
                    &notif.game_name,
                    &notif.title,
                    &notif.description,
                );

                // Also post a tray balloon so players still see it after
                // leaving the game window.
                let title = format!("Achievement unlocked ({})", notif.game_name);
                let message = if notif.description.is_empty() {
                    notif.title
                } else {
                    format!("{}: {}", notif.title, notif.description)
                };
                tray_handle.show_balloon_notification(&title, &message, false);
            }
        }
    });
}

/// Minimal context menu: status line, dashboard, backup, exit.
fn show_context_menu(hwnd: HWND) {
    unsafe {
        let mut pt = POINT { x: 0, y: 0 };
        GetCursorPos(&mut pt);
        let hmenu = CreatePopupMenu();

        let cfg = load_config();
        let games = crate::backup::SaveScanner::scan_all(&cfg).len();
        let trophies = crate::achievements::load_achievements_history().len();

        AppendMenuW(
            hmenu,
            MF_STRING | MF_GRAYED,
            ID_STATUS,
            to_wide_chars(&format!("{games} saved  |  {trophies} trophies")).as_ptr(),
        );
        AppendMenuW(hmenu, MF_SEPARATOR, 0, null());
        AppendMenuW(
            hmenu,
            MF_STRING,
            ID_OPEN_DASHBOARD,
            to_wide_chars("Open Sager").as_ptr(),
        );
        AppendMenuW(
            hmenu,
            MF_STRING,
            ID_BACKUP_ALL,
            to_wide_chars("Back up all games now").as_ptr(),
        );
        AppendMenuW(hmenu, MF_SEPARATOR, 0, null());
        AppendMenuW(hmenu, MF_STRING, ID_EXIT, to_wide_chars("Exit").as_ptr());

        SetForegroundWindow(hwnd);
        TrackPopupMenu(
            hmenu,
            TPM_RIGHTBUTTON | TPM_BOTTOMALIGN,
            pt.x,
            pt.y,
            0,
            hwnd,
            null(),
        );
        DestroyMenu(hmenu);
    }
}

pub unsafe extern "system" fn tray_window_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_TRAY_CALLBACK => {
            let event = lparam as u32;
            match event {
                WM_RBUTTONUP => show_context_menu(hwnd),
                WM_LBUTTONUP => open_dashboard(),
                _ => {}
            }
            0
        }
        WM_COMMAND => {
            match wparam as usize {
                ID_OPEN_DASHBOARD => open_dashboard(),
                ID_BACKUP_ALL => {
                    let cfg = load_config();
                    let results = crate::backup::SaveArchiver::backup_all(&cfg);
                    let success = results.iter().filter(|r| r.is_ok()).count();
                    let handle = TrayHandle(hwnd as isize);
                    handle.show_balloon_notification(
                        "Backup complete",
                        &format!("{} of {} games saved", success, results.len()),
                        false,
                    );
                }
                ID_EXIT => {
                    TrayHandle(hwnd as isize).remove_icon();
                    PostQuitMessage(0);
                }
                _ => {}
            }
            0
        }
        WM_DESTROY => {
            PostQuitMessage(0);
            0
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}
