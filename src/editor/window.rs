//! The plugin's `Editor`: opens an egui-baseview window inside the host's window.
//!
//! Adapted from nih_plug_egui's `editor.rs` (nih-plug, ISC License,
//! Copyright (c) 2022-2024 Robbert van der Helm). Differences:
//! - No forced repaint every frame. egui-baseview runs the egui pass every frame but only
//!   renders when a repaint is requested; `EditorUi::update` requests one when a parameter
//!   changed, on input, and for the first frames after opening.
//! - No resizing (the panel is a fixed-size image).

use super::EditorUi;
use crate::IabSaturatorParams;
use baseview::gl::GlConfig;
use baseview::{Size, WindowHandle, WindowOpenOptions, WindowScalePolicy};
use egui_baseview::{EguiWindow, GraphicsConfig};
use nih_plug::prelude::{Editor, GuiContext, ParamSetter, ParentWindowHandle};
use raw_window_handle::{HasRawWindowHandle, RawWindowHandle};
use std::any::Any;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;

pub struct IabEditor {
    params: Arc<IabSaturatorParams>,
    /// Host DPI scale factor, as `f32` bits. 1.0 until the host sets one (Windows/Linux).
    scale_factor: AtomicU32,
    /// Whether a window is open. The host must not change the scale while it is.
    open: Arc<AtomicBool>,
}

impl IabEditor {
    pub fn new(params: Arc<IabSaturatorParams>) -> Self {
        Self {
            params,
            scale_factor: AtomicU32::new(1.0f32.to_bits()),
            open: Arc::new(AtomicBool::new(false)),
        }
    }
}

/// baseview uses a different `raw_window_handle` version than nih-plug: adapt the handle.
struct ParentWindowHandleAdapter(ParentWindowHandle);

unsafe impl HasRawWindowHandle for ParentWindowHandleAdapter {
    fn raw_window_handle(&self) -> RawWindowHandle {
        match self.0 {
            ParentWindowHandle::X11Window(window) => {
                let mut handle = raw_window_handle::XcbWindowHandle::empty();
                handle.window = window;
                RawWindowHandle::Xcb(handle)
            }
            ParentWindowHandle::AppKitNsView(ns_view) => {
                let mut handle = raw_window_handle::AppKitWindowHandle::empty();
                handle.ns_view = ns_view;
                RawWindowHandle::AppKit(handle)
            }
            ParentWindowHandle::Win32Hwnd(hwnd) => {
                let mut handle = raw_window_handle::Win32WindowHandle::empty();
                handle.hwnd = hwnd;
                RawWindowHandle::Win32(handle)
            }
        }
    }
}

impl Editor for IabEditor {
    fn spawn(
        &self,
        parent: ParentWindowHandle,
        context: Arc<dyn GuiContext>,
    ) -> Box<dyn Any + Send> {
        let (width, height) = self.params.editor_state.size();

        // On macOS the system scale must be used (see nih_plug_egui); elsewhere the host's.
        #[cfg(target_os = "macos")]
        let scale = WindowScalePolicy::SystemScaleFactor;
        #[cfg(not(target_os = "macos"))]
        let scale = WindowScalePolicy::ScaleFactor(f32::from_bits(
            self.scale_factor.load(Ordering::Acquire),
        ) as f64);

        let window = EguiWindow::open_parented(
            &ParentWindowHandleAdapter(parent),
            WindowOpenOptions {
                title: String::from("IAB-Saturator"),
                // Logical size; baseview applies the scale factor.
                size: Size::new(width as f64, height as f64),
                scale,
                gl_config: Some(GlConfig {
                    version: (3, 2),
                    red_bits: 8,
                    blue_bits: 8,
                    green_bits: 8,
                    alpha_bits: 8,
                    depth_bits: 24,
                    stencil_bits: 8,
                    samples: None,
                    srgb: true,
                    double_buffer: true,
                    vsync: true,
                    ..Default::default()
                }),
            },
            GraphicsConfig::default(),
            // Decodes the images: once per opening, on the GUI thread.
            EditorUi::new(self.params.clone()),
            |ctx, _queue, ui| ui.build(ctx),
            move |ctx, _queue, ui| {
                let setter = ParamSetter::new(context.as_ref());
                ui.update(ctx, &setter);
            },
        );

        self.open.store(true, Ordering::Release);
        Box::new(EditorHandle {
            open: self.open.clone(),
            window,
        })
    }

    fn size(&self) -> (u32, u32) {
        self.params.editor_state.size()
    }

    fn set_scale_factor(&self, factor: f32) -> bool {
        // The scale cannot change while the window is open (Ableton Live tries to).
        if self.open.load(Ordering::Acquire) {
            return false;
        }
        self.scale_factor.store(factor.to_bits(), Ordering::Release);
        true
    }

    // Parameter changes are picked up by polling the parameters each egui pass (see
    // `EditorUi::update`), which needs no state shared with the host's calling thread.
    fn param_value_changed(&self, _id: &str, _normalized_value: f32) {}
    fn param_modulation_changed(&self, _id: &str, _modulation_offset: f32) {}
    fn param_values_changed(&self) {}
}

struct EditorHandle {
    open: Arc<AtomicBool>,
    window: WindowHandle,
}

/// `WindowHandle` holds raw pointers, so it is not `Send` by itself. Same as nih_plug_egui.
unsafe impl Send for EditorHandle {}

impl Drop for EditorHandle {
    fn drop(&mut self) {
        self.open.store(false, Ordering::Release);
        // Should happen when the handle is dropped, but does not always (see nih_plug_egui).
        self.window.close();
    }
}
