use std::{mem::size_of, path::Path, ptr::null_mut, slice};

use base64::{engine::general_purpose::STANDARD, Engine as _};
use windows::{
    core::PCWSTR,
    Win32::{
        Graphics::Gdi::{
            CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, GdiFlush, SelectObject,
            BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, HGDIOBJ,
        },
        Storage::FileSystem::FILE_FLAGS_AND_ATTRIBUTES,
        UI::{
            Shell::{SHGetFileInfoW, SHFILEINFOW, SHGFI_ICON},
            WindowsAndMessaging::{DestroyIcon, DrawIconEx, DI_NORMAL, HICON},
        },
    },
};

const SIZE: i32 = 48;
const BYTES: usize = (SIZE * SIZE * 4) as usize;

pub fn read(path: &Path) -> Option<String> {
    let path = crate::windows_shell::wide(path.as_os_str());
    let mut info = SHFILEINFOW::default();
    let result = unsafe {
        SHGetFileInfoW(
            PCWSTR(path.as_ptr()),
            FILE_FLAGS_AND_ATTRIBUTES(0),
            Some(&mut info),
            size_of::<SHFILEINFOW>() as u32,
            SHGFI_ICON,
        )
    };
    if result == 0 || info.hIcon.is_invalid() {
        return None;
    }

    let pixels = pixels(info.hIcon);
    let _ = unsafe { DestroyIcon(info.hIcon) };
    let mut pixels = pixels?;
    // A modern Windows icon uses premultiplied BGRA. Convert to PNG's straight RGBA.
    for pixel in pixels.chunks_exact_mut(4) {
        let alpha = u32::from(pixel[3]);
        pixel.swap(0, 2);
        if alpha != 0 {
            for channel in &mut pixel[..3] {
                *channel = (u32::from(*channel) * 255 / alpha).min(255) as u8;
            }
        }
    }
    if pixels.chunks_exact(4).all(|pixel| pixel[3] == 0) {
        return None;
    }

    let mut png = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut png, SIZE as u32, SIZE as u32);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder
            .write_header()
            .ok()?
            .write_image_data(&pixels)
            .ok()?;
    }
    Some(format!("data:image/png;base64,{}", STANDARD.encode(png)))
}

fn pixels(icon: HICON) -> Option<Vec<u8>> {
    let bitmap = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: SIZE,
            biHeight: -SIZE,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        },
        ..Default::default()
    };
    unsafe {
        let dc = CreateCompatibleDC(None);
        if dc.is_invalid() {
            return None;
        }
        let mut bits = null_mut();
        let dib = CreateDIBSection(Some(dc), &bitmap, DIB_RGB_COLORS, &mut bits, None, 0).ok();
        let pixels = dib.and_then(|dib| {
            let object = HGDIOBJ(dib.0);
            let previous = SelectObject(dc, object);
            let data = if previous.is_invalid() || bits.is_null() {
                None
            } else {
                bits.cast::<u8>().write_bytes(0, BYTES);
                let painted = DrawIconEx(dc, 0, 0, icon, SIZE, SIZE, 0, None, DI_NORMAL).is_ok();
                let _ = GdiFlush();
                painted.then(|| slice::from_raw_parts(bits.cast::<u8>(), BYTES).to_vec())
            };
            if !previous.is_invalid() {
                SelectObject(dc, previous);
            }
            let _ = DeleteObject(object);
            data
        });
        let _ = DeleteDC(dc);
        pixels
    }
}
