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

fn coverage(d: f32) -> u32 {
    if d >= 1.0 {
        return 0;
    }
    if d < 0.55 {
        return 255;
    }
    (255.0 * (1.0 - (d - 0.55) / 0.45).clamp(0.0, 1.0)) as u32
}

fn create_sager_icon() -> isize {
    unsafe {
        let pixels = outline_icon_pixels();
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

// ── Outline geometry helpers (distances to strokes, so the icon can be
//    previewed in a unit test and ported to the tray bitmap 1:1) ──

/// Signed distance to a rounded rectangle's boundary (outline stroke).
fn sd_roundbox_outline(px: f32, py: f32, cx: f32, cy: f32, hx: f32, hy: f32, r: f32) -> f32 {
    let qx = (px - cx).abs() - hx + r;
    let qy = (py - cy).abs() - hy + r;
    let outside = ((qx.max(0.0)).powi(2) + (qy.max(0.0)).powi(2)).sqrt();
    outside + qx.max(qy).min(0.0) - r
}

/// Distance to a vertical segment (x constant).
fn seg_v_dist(px: f32, py: f32, x: f32, y0: f32, y1: f32) -> f32 {
    let t = py.clamp(y0.min(y1), y0.max(y1));
    ((px - x).powi(2) + (py - t).powi(2)).sqrt()
}

/// Distance to a circular arc (cx, cy, radius) restricted to the angular
/// span [a0, a1) in degrees. Angles use screen coordinates (y down).
fn arc_dist(px: f32, py: f32, cx: f32, cy: f32, r: f32, a0: f32, a1: f32) -> f32 {
    let dx = px - cx;
    let dy = py - cy;
    let len = (dx * dx + dy * dy).sqrt();
    let mut theta = dy.atan2(dx).to_degrees();
    // normalize into [a0, a0 + 360)
    let span = a1 - a0;
    while theta < a0 {
        theta += 360.0;
    }
    while theta >= a0 + 360.0 {
        theta -= 360.0;
    }
    if theta < a0 + span {
        (len - r).abs()
    } else {
        // nearest arc endpoint
        let (ex, ey) = {
            let a = if (theta - a0).abs() < (theta - a1).abs() {
                a0
            } else {
                a1
            };
            let rad = a.to_radians();
            (cx + r * rad.cos(), cy + r * rad.sin())
        };
        ((px - ex).powi(2) + (py - ey).powi(2)).sqrt()
    }
}

/// The shackle: a U-shaped stroke (left leg, top arc, right leg).
fn shackle_dist(px: f32, py: f32) -> f32 {
    seg_v_dist(px, py, 10.4, 8.0, 14.0)
        .min(arc_dist(px, py, 16.0, 8.0, 5.6, 180.0, 360.0))
        .min(seg_v_dist(px, py, 21.6, 8.0, 14.0))
}

/// The keyhole: small ring + stem, drawn in the accent cyan.
fn key_dist(px: f32, py: f32) -> f32 {
    let dx = px - 16.0;
    let dy = py - 19.0;
    let ring = ((dx * dx + dy * dy).sqrt() - 2.3).abs();
    ring.min(seg_v_dist(px, py, 16.0, 21.0, 24.2))
}

fn outline_icon_pixels() -> [u32; 32 * 32] {
    let mut pixels = [0u32; 32 * 32];
    for y in 0..32 {
        for x in 0..32 {
            let px = x as f32 + 0.5;
            let py = y as f32 + 0.5;

            let d_body = sd_roundbox_outline(px, py, 16.0, 20.0, 7.6, 5.8, 2.5).abs();
            let d_shackle = shackle_dist(px, py);
            let d_key = key_dist(px, py);

            let mut d = d_body.min(d_shackle);
            let mut cyan = false;
            if d_key < d {
                d = d_key;
                cyan = true;
            }

            let alpha = coverage(d);
            if alpha > 0 {
                let (r, g, b) = if cyan {
                    (126u32, 200u32, 244u32)
                } else {
                    (255u32, 255u32, 255u32)
                };
                pixels[y * 32 + x] = (alpha << 24) | (r << 16) | (g << 8) | b;
            }
        }
    }
    pixels
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Renders the tray icon over a dark and a light background so the
    /// outline can be reviewed: target\tray_icon_preview.bmp
    #[test]
    fn render_preview_bitmap() {
        let icon = outline_icon_pixels();
        let w = 72usize;
        let h = 32usize;
        let mut out = vec![0u8; 54 + w * h * 4];

        // BMP headers
        out[0] = b'B';
        out[1] = b'M';
        let file_size = (out.len()) as u32;
        out[2..6].copy_from_slice(&file_size.to_le_bytes());
        out[10..14].copy_from_slice(&54u32.to_le_bytes());
        out[14..18].copy_from_slice(&(40u32).to_le_bytes());
        out[18..22].copy_from_slice(&(w as i32).to_le_bytes());
        out[22..26].copy_from_slice(&(-(h as i32)).to_le_bytes());
        out[26..28].copy_from_slice(&1u16.to_le_bytes());
        out[28..30].copy_from_slice(&32u16.to_le_bytes());

        for y in 0..h {
            for x in 0..w {
                // background: dark navy strip on the left, white strip on the right
                let (br, bg, bb) = if x < 36 {
                    (15u8, 23u8, 42u8)
                } else {
                    (250u8, 250u8, 250u8)
                };
                // icon is sampled twice: dark half uses x, light half uses x - 36
                let ix = (if x < 36 { x } else { x - 36 }).min(31);
                let a = icon[y * 32 + ix];
                let ia = ((a >> 24) & 0xFF) as u32;
                let ir = ((a >> 16) & 0xFF) as u32;
                let ig = ((a >> 8) & 0xFF) as u32;
                let ib = (a & 0xFF) as u32;
                let mix = |fg: u32, bk: u8| (fg * ia + (bk as u32) * (255 - ia)) / 255;
                let pos = 54 + (y * w + x) * 4;
                out[pos] = mix(ib, bb) as u8;
                out[pos + 1] = mix(ig, bg) as u8;
                out[pos + 2] = mix(ir, br) as u8;
                out[pos + 3] = 255;
            }
        }

        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("tray_icon_preview.bmp");
        std::fs::write(&path, &out).unwrap();
        println!("preview written to {}", path.display());
    }
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
