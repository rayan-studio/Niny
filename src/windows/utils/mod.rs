/*
 * Copyright (c) Axe. All rights reserved.
 * Licensed under the MIT License. See License.txt in the project root for license information.
 */

// Convertit une couleur sRGB (0-255) en espace linéaire pour le GPU
pub fn srgb_to_linear(c: f64) -> f64 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

pub fn srgb(r: u8, g: u8, b: u8) -> wgpu::Color {
    wgpu::Color {
        r: srgb_to_linear(r as f64 / 255.0),
        g: srgb_to_linear(g as f64 / 255.0),
        b: srgb_to_linear(b as f64 / 255.0),
        a: 1.0,
    }
}


#[cfg(target_os = "windows")]
pub fn apply_chromium_dwm_shadow(window: &winit::window::Window) {
    use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};

    #[repr(C)]
    struct Margins {
        cx_left_width: i32,
        cx_right_width: i32,
        cy_top_height: i32,
        cy_bottom_height: i32,
    }

    #[link(name = "dwmapi")]
    unsafe extern "system" {
        fn DwmExtendFrameIntoClientArea(hwnd: isize, p_mar_inset: *const Margins) -> i32;
        fn DwmSetWindowAttribute(
            hwnd: isize,
            dw_attribute: u32,
            pv_attribute: *const std::ffi::c_void,
            cb_attribute: u32,
        ) -> i32;
    }

    #[link(name = "user32")]
    unsafe extern "system" {
        fn LoadImageW(hinst: isize, name: *const u16, type_: u32, cx: i32, cy: i32, fu_load: u32) -> isize;
        fn SendMessageW(hwnd: isize, msg: u32, wparam: usize, lparam: isize) -> isize;
        fn SetClassLongPtrW(hwnd: isize, n_index: i32, new_long: isize) -> isize;
        fn GetWindowLongW(hwnd: isize, n_index: i32) -> i32;
        fn SetWindowLongW(hwnd: isize, n_index: i32, new_long: i32) -> i32;
        fn SetWindowPos(hwnd: isize, insert_after: isize, x: i32, y: i32, cx: i32, cy: i32, flags: u32) -> i32;
    }

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetModuleHandleW(lp_module_name: *const u16) -> isize;
    }

    if let Ok(handle) = window.window_handle() {
        if let RawWindowHandle::Win32(handle) = handle.as_raw() {
            let hwnd = handle.hwnd.get();
            unsafe {
                // Style nécessaire pour que Windows Taskbar traite la fenêtre comme une vraie application avec icône
                let current_style = GetWindowLongW(hwnd, -16);
                const WS_SYSMENU: i32 = 0x00080000;
                const WS_MINIMIZEBOX: i32 = 0x00020000;
                SetWindowLongW(hwnd, -16, current_style | WS_SYSMENU | WS_MINIMIZEBOX);
                SetWindowPos(hwnd, 0, 0, 0, 0, 0, 0x0027); // SWP_FRAMECHANGED

                // 1. Charger l'icône compilée avec winres (Ressource ID 1)
                let hinstance = GetModuleHandleW(std::ptr::null());
                let hicon_big = LoadImageW(hinstance, 1 as *const u16, 1, 32, 32, 0);
                let hicon_small = LoadImageW(hinstance, 1 as *const u16, 1, 16, 16, 0);

                if hicon_big != 0 {
                    SendMessageW(hwnd, 0x0080, 1, hicon_big); // WM_SETICON ICON_BIG
                    SetClassLongPtrW(hwnd, -14, hicon_big);    // GCLP_HICON (force la barre des tâches)
                }
                if hicon_small != 0 {
                    SendMessageW(hwnd, 0x0080, 0, hicon_small); // WM_SETICON ICON_SMALL
                    SetClassLongPtrW(hwnd, -34, hicon_small);   // GCLP_HICONSM
                }

                // 2. Étend la frame pour l'ombre
                let margins = Margins {
                    cx_left_width: 1,
                    cx_right_width: 1,
                    cy_top_height: 1,
                    cy_bottom_height: 1,
                };
                DwmExtendFrameIntoClientArea(hwnd, &margins);

                // 3. Coins arrondis Windows 11
                let corner_pref: u32 = 2; // DWMWCP_ROUND
                DwmSetWindowAttribute(
                    hwnd,
                    33,
                    &corner_pref as *const u32 as *const _,
                    std::mem::size_of::<u32>() as u32,
                );
            }
        }
    }
}