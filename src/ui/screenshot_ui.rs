use std::cell::RefCell;
use std::cmp::{max, min};
use std::collections::HashMap;
use std::f64::consts::TAU;
use std::iter::zip;
use std::rc::Rc;

use anyhow::bail;
use niri_config::{Action, Config};
use niri_ipc::SizeChange;
use pango::{Alignment, FontDescription};
use pangocairo::cairo::{self, ImageSurface};
use smithay::backend::allocator::Fourcc;
use smithay::backend::input::TouchSlot;
use smithay::backend::renderer::element::utils::{Relocate, RelocateRenderElement};
use smithay::backend::renderer::element::Kind;
use smithay::backend::renderer::gles::{GlesRenderer, GlesTexture};
use smithay::backend::renderer::{Texture as _, TextureFilter};
use smithay::input::keyboard::{Keysym, ModifiersState};
use smithay::output::{Output, WeakOutput};
use smithay::utils::{Buffer, Physical, Point, Rectangle, Scale, Size, Transform};

use crate::animation::{Animation, Clock};
use crate::layout::floating::DIRECTIONAL_MOVE_PX;
use crate::niri_render_elements;
use crate::render_helpers::primary_gpu_texture::PrimaryGpuTextureRenderElement;
use crate::render_helpers::solid_color::{SolidColorBuffer, SolidColorRenderElement};
use crate::render_helpers::texture::{TextureBuffer, TextureRenderElement};
use crate::render_helpers::zoom::ZoomElement;
use crate::render_helpers::{render_to_vec, RenderTarget};
use crate::utils::geometry::PointExt;
use crate::utils::to_physical_precise_round;
use crate::utils::view::{OutputViewCtx, ViewportTransform};

const SELECTION_BORDER: i32 = 2;

const PADDING: i32 = 8;
const RADIUS: i32 = 16;
const FONT: &str = "sans 14px";
const BORDER: i32 = 4;
const TEXT_HIDE_P: &str =
    "Press <span face='mono' bgcolor='#2C2C2C'> Space </span> to save the screenshot.\n\
     Press <span face='mono' bgcolor='#2C2C2C'> P </span> to hide the pointer.";
const TEXT_SHOW_P: &str =
    "Press <span face='mono' bgcolor='#2C2C2C'> Space </span> to save the screenshot.\n\
     Press <span face='mono' bgcolor='#2C2C2C'> P </span> to show the pointer.";

// Ideally the screenshot UI should support cross-output selections. However, that poses some
// technical challenges when the outputs have different scales and such. So, this implementation
// allows only single-output selections for now.
//
// As a consequence of this, selection coordinates are in output-local coordinate space.
#[allow(clippy::large_enum_variant)]
pub enum ScreenshotUi {
    Closed {
        last_selection: Option<(WeakOutput, Rectangle<i32, Physical>)>,
        clock: Clock,
        config: Rc<RefCell<Config>>,
    },
    Open {
        selection: (Output, Point<i32, Physical>, Point<i32, Physical>),
        output_data: HashMap<Output, OutputData>,
        button: Button,
        show_pointer: bool,
        open_anim: Animation,
        clock: Clock,
        config: Rc<RefCell<Config>>,
        path: Option<String>,
    },
}

/// State for moving the selection (as opposed to just drawing).
pub struct MoveState {
    // Cursor offset from selection.1 when starting the move.
    pointer_offset: Point<i32, Physical>,
    // If the move is initiated by a touch, this is the slot. If `None`, the move was initiated by
    // holding Space.
    touch_slot: Option<TouchSlot>,
}

pub enum Button {
    Up,
    Down {
        touch_slot: Option<TouchSlot>,
        on_capture_button: bool,
        last_pos: (Output, Point<i32, Physical>),
        move_state: Option<MoveState>,
    },
}

pub struct OutputData {
    size: Size<i32, Physical>,
    scale: f64,
    transform: Transform,
    // Output, screencast, screen capture.
    screenshot: [OutputScreenshot; 3],
    // Chrome colors/ids/commits; layout derives at render time (see
    // render_output). Only index 4 (fullscreen dim for other outputs) is
    // sized in update_buffers.
    buffers: [SolidColorBuffer; 8],
    panel: Option<(TextureBuffer<GlesTexture>, TextureBuffer<GlesTexture>)>,
}

pub struct OutputScreenshot {
    buffer: PrimaryGpuTextureRenderElement,
    pointer: Option<FrozenPointer>,
    /// Live zoom factor at capture time: baseline for relative scaling.
    ///
    /// The texture itself is always unzoomed — this is NOT baked zoom.
    capture_level: f64,
}

/// A frozen pointer graphic: element plus capture-time hotspot and tip.
///
/// Tip is stored, not re-derived, so fractional scales can't shift it.
#[derive(Debug, Clone)]
struct FrozenPointer {
    element: PrimaryGpuTextureRenderElement,
    hotspot: Point<i32, Physical>,
    tip: Point<i32, Physical>,
}

/// Captured pointer texture, placement, hotspot and tip.
///
/// Hotspot and tip are resolved once at capture time.
pub(crate) struct CapturedPointer {
    pub(crate) texture: GlesTexture,
    pub(crate) geo: Rectangle<i32, Physical>,
    pub(crate) hotspot: Point<i32, Physical>,
    pub(crate) tip: Point<i32, Physical>,
}

/// Live-zoom parameters for the screenshot UI preview.
///
/// The captured scene is static and unzoomed; the preview re-applies these
/// each frame so the UI acts as a magnifier while selecting.
#[derive(Clone, Copy)]
pub struct ScreenshotPreviewZoom {
    pub viewport: ViewportTransform,
    pub view_ctx: OutputViewCtx,
    pub filter: Option<TextureFilter>,
    /// Hotspot-centered construction like the live pointer; otherwise the
    /// graphic scales relative to the capture baseline.
    pub scale_with_zoom: bool,
}

niri_render_elements! {
    ScreenshotUiRenderElement => {
        Screenshot = PrimaryGpuTextureRenderElement,
        SolidColor = SolidColorRenderElement,
        Zoomed = ZoomElement<PrimaryGpuTextureRenderElement>,
    }
}

impl Button {
    fn is_down(&self) -> bool {
        matches!(self, Self::Down { .. })
    }

    fn is_dragging_selection(&self) -> bool {
        matches!(
            self,
            Self::Down {
                on_capture_button: false,
                ..
            }
        )
    }
}

impl ScreenshotUi {
    pub fn new(clock: Clock, config: Rc<RefCell<Config>>) -> Self {
        Self::Closed {
            last_selection: None,
            clock,
            config,
        }
    }

    pub fn open(
        &mut self,
        renderer: &mut GlesRenderer,
        // Output, screencast, screen capture.
        screenshots: HashMap<Output, [OutputScreenshot; 3]>,
        default_output: Output,
        show_pointer: bool,
        path: Option<String>,
    ) -> bool {
        if screenshots.is_empty() {
            return false;
        }

        let Self::Closed {
            last_selection,
            clock,
            config,
        } = self
        else {
            return false;
        };

        let last_selection = last_selection
            .take()
            .and_then(|(weak, sel)| weak.upgrade().map(|output| (output, sel)));
        let selection = match last_selection {
            Some(selection) if screenshots.contains_key(&selection.0) => selection,
            _ => {
                let output = default_output;
                let output_transform = output.current_transform();
                let output_mode = output.current_mode().unwrap();
                let size = output_transform.transform_size(output_mode.size);
                (
                    output,
                    Rectangle::new(
                        Point::from((size.w / 4, size.h / 4)),
                        Size::from((size.w / 2, size.h / 2)),
                    ),
                )
            }
        };

        let selection = (
            selection.0,
            selection.1.loc,
            selection.1.loc + selection.1.size - Size::from((1, 1)),
        );

        let output_data = screenshots
            .into_iter()
            .map(|(output, screenshot)| {
                let transform = output.current_transform();
                let output_mode = output.current_mode().unwrap();
                let size = transform.transform_size(output_mode.size);
                let scale = output.current_scale().fractional_scale();
                let buffers = [
                    SolidColorBuffer::new((0., 0.), [1., 1., 1., 1.]),
                    SolidColorBuffer::new((0., 0.), [1., 1., 1., 1.]),
                    SolidColorBuffer::new((0., 0.), [1., 1., 1., 1.]),
                    SolidColorBuffer::new((0., 0.), [1., 1., 1., 1.]),
                    SolidColorBuffer::new((0., 0.), [0., 0., 0., 0.5]),
                    SolidColorBuffer::new((0., 0.), [0., 0., 0., 0.5]),
                    SolidColorBuffer::new((0., 0.), [0., 0., 0., 0.5]),
                    SolidColorBuffer::new((0., 0.), [0., 0., 0., 0.5]),
                ];
                let mut render_panel_ = |text| {
                    render_panel(renderer, scale, text)
                        .map_err(|err| warn!("error rendering help panel: {err:?}"))
                        .ok()
                };
                let panel_show = render_panel_(TEXT_SHOW_P);
                let panel_hide = render_panel_(TEXT_HIDE_P);
                let panel = Option::zip(panel_show, panel_hide);

                let data = OutputData {
                    size,
                    scale,
                    transform,
                    screenshot,
                    buffers,
                    panel,
                };
                (output, data)
            })
            .collect();

        let open_anim = {
            let c = config.borrow();
            Animation::new(clock.clone(), 0., 1., 0., c.animations.screenshot_ui_open.0)
        };

        *self = Self::Open {
            selection,
            output_data,
            button: Button::Up,
            show_pointer,
            open_anim,
            clock: clock.clone(),
            config: config.clone(),
            path,
        };

        self.update_buffers();

        true
    }

    pub fn close(&mut self) -> bool {
        let Self::Open {
            selection,
            clock,
            config,
            ..
        } = self
        else {
            return false;
        };

        let last_selection = Some((
            selection.0.downgrade(),
            rect_from_corner_points(selection.1, selection.2),
        ));

        *self = Self::Closed {
            last_selection,
            clock: clock.clone(),
            config: config.clone(),
        };

        true
    }

    pub fn toggle_pointer(&mut self) {
        if let Self::Open { show_pointer, .. } = self {
            *show_pointer = !*show_pointer;
        }
    }

    pub fn is_open(&self) -> bool {
        matches!(self, ScreenshotUi::Open { .. })
    }

    pub fn set_space_down(&mut self, down: bool) {
        if let Self::Open {
            selection,
            button:
                Button::Down {
                    move_state,
                    last_pos,
                    ..
                },
            ..
        } = self
        {
            if down {
                if move_state.is_none() {
                    *move_state = Some(MoveState {
                        pointer_offset: last_pos.1 - selection.1,
                        touch_slot: None,
                    });
                }
            } else {
                // Only clear if moving with Space.
                if let Some(MoveState {
                    touch_slot: None, ..
                }) = move_state
                {
                    *move_state = None;
                }
            }
        }
    }

    pub fn move_left(&mut self) {
        let Self::Open {
            selection: (output, a, b),
            output_data,
            ..
        } = self
        else {
            return;
        };

        let data = &output_data[output];

        let delta: i32 = to_physical_precise_round(data.scale, DIRECTIONAL_MOVE_PX);
        let delta = min(delta, min(a.x, b.x));
        a.x -= delta;
        b.x -= delta;

        self.update_buffers();
    }

    pub fn move_right(&mut self) {
        let Self::Open {
            selection: (output, a, b),
            output_data,
            ..
        } = self
        else {
            return;
        };

        let data = &output_data[output];

        let delta: i32 = to_physical_precise_round(data.scale, DIRECTIONAL_MOVE_PX);
        let delta = min(delta, data.size.w - max(a.x, b.x) - 1);
        a.x += delta;
        b.x += delta;

        self.update_buffers();
    }

    pub fn move_up(&mut self) {
        let Self::Open {
            selection: (output, a, b),
            output_data,
            ..
        } = self
        else {
            return;
        };

        let data = &output_data[output];

        let delta: i32 = to_physical_precise_round(data.scale, DIRECTIONAL_MOVE_PX);
        let delta = min(delta, min(a.y, b.y));
        a.y -= delta;
        b.y -= delta;

        self.update_buffers();
    }

    pub fn move_down(&mut self) {
        let Self::Open {
            selection: (output, a, b),
            output_data,
            ..
        } = self
        else {
            return;
        };

        let data = &output_data[output];

        let delta: i32 = to_physical_precise_round(data.scale, DIRECTIONAL_MOVE_PX);
        let delta = min(delta, data.size.h - max(a.y, b.y) - 1);
        a.y += delta;
        b.y += delta;

        self.update_buffers();
    }

    /// Moves the screenshot selection to a different output.
    ///
    /// This preserves the relative position while keeping logical size. It is (intentionally) very
    /// similar to how floating windows move across monitors, but with one difference: floating
    /// windows can go partially outside the view, while the screenshot selection cannot. So, we
    /// clamp it to new output bounds, trying to preserve the size if possible.
    pub fn move_to_output(&mut self, new_output: Output) {
        let Self::Open {
            selection,
            output_data,
            ..
        } = self
        else {
            return;
        };

        let (current_output, current_a, current_b) = selection;

        if current_output == &new_output {
            return;
        }

        let Some(target_data) = output_data.get(&new_output) else {
            return;
        };

        let current_data = &output_data[current_output];

        let current_rect: Rectangle<_, Physical> = Rectangle::new(
            Point::from((current_a.x.min(current_b.x), current_a.y.min(current_b.y))),
            Size::from((
                (current_a.x.max(current_b.x) - current_a.x.min(current_b.x) + 1),
                (current_a.y.max(current_b.y) - current_a.y.min(current_b.y) + 1),
            )),
        );
        let current_rect = current_rect.to_f64();

        let rel_x = current_rect.loc.x / current_data.size.w as f64;
        let rel_y = current_rect.loc.y / current_data.size.h as f64;

        let factor = target_data.scale / current_data.scale;
        let mut new_width = (current_rect.size.w * factor).round() as i32;
        let mut new_height = (current_rect.size.h * factor).round() as i32;

        new_width = new_width.clamp(1, target_data.size.w);
        new_height = new_height.clamp(1, target_data.size.h);

        let new_x = (rel_x * target_data.size.w as f64).round() as i32;
        let new_y = (rel_y * target_data.size.h as f64).round() as i32;

        let max_x = target_data.size.w - new_width;
        let max_y = target_data.size.h - new_height;
        let new_x = new_x.clamp(0, max_x);
        let new_y = new_y.clamp(0, max_y);

        let new_rect = Rectangle::new(
            Point::from((new_x, new_y)),
            Size::from((new_width, new_height)),
        );

        *selection = (
            new_output,
            new_rect.loc,
            new_rect.loc + new_rect.size - Size::from((1, 1)),
        );

        self.update_buffers();
    }

    pub fn set_width(&mut self, change: SizeChange) {
        let Self::Open {
            selection: (output, a, b),
            output_data,
            ..
        } = self
        else {
            return;
        };

        let data = &output_data[output];

        let available_size = f64::from(data.size.w);
        let current_size = max(a.x, b.x) + 1 - min(a.x, b.x);

        let new_size = match change {
            SizeChange::SetFixed(fixed) => to_physical_precise_round(data.scale, fixed),
            SizeChange::SetProportion(prop) => {
                let prop = (prop / 100.).clamp(0., 1.);
                (available_size * prop).round() as i32
            }
            SizeChange::AdjustFixed(delta) => {
                let delta = to_physical_precise_round(data.scale, delta);
                current_size.saturating_add(delta)
            }
            SizeChange::AdjustProportion(delta) => {
                let current_prop = f64::from(current_size) / available_size;
                let prop = (current_prop + delta / 100.).clamp(0., 1.);
                (available_size * prop).round() as i32
            }
        };
        let new_size = new_size.clamp(1, data.size.w - min(a.x, b.x)) - 1;
        a.x = min(a.x, b.x);
        b.x = a.x + new_size;

        self.update_buffers();
    }

    pub fn set_height(&mut self, change: SizeChange) {
        let Self::Open {
            selection: (output, a, b),
            output_data,
            ..
        } = self
        else {
            return;
        };

        let data = &output_data[output];

        let available_size = f64::from(data.size.h);
        let current_size = max(a.y, b.y) + 1 - min(a.y, b.y);

        let new_size = match change {
            SizeChange::SetFixed(fixed) => to_physical_precise_round(data.scale, fixed),
            SizeChange::SetProportion(prop) => {
                let prop = (prop / 100.).clamp(0., 1.);
                (available_size * prop).round() as i32
            }
            SizeChange::AdjustFixed(delta) => {
                let delta = to_physical_precise_round(data.scale, delta);
                current_size.saturating_add(delta)
            }
            SizeChange::AdjustProportion(delta) => {
                let current_prop = f64::from(current_size) / available_size;
                let prop = (current_prop + delta / 100.).clamp(0., 1.);
                (available_size * prop).round() as i32
            }
        };
        let new_size = new_size.clamp(1, data.size.h - min(a.y, b.y)) - 1;
        a.y = min(a.y, b.y);
        b.y = a.y + new_size;

        self.update_buffers();
    }

    pub fn advance_animations(&mut self) {}

    pub fn are_animations_ongoing(&self) -> bool {
        let Self::Open { open_anim, .. } = self else {
            return false;
        };

        !open_anim.is_done()
    }

    fn update_buffers(&mut self) {
        let Self::Open {
            selection,
            output_data,
            ..
        } = self
        else {
            panic!("screenshot UI must be open to update buffers");
        };

        let (selection_output, a, b) = selection;
        let mut rect = rect_from_corner_points(*a, *b);

        for (output, data) in output_data {
            let buffers = &mut data.buffers;
            let size = data.size;

            if output == selection_output {
                // Check if the selection is still valid. If not, reset it back to default.
                if !Rectangle::from_size(size).contains_rect(rect) {
                    rect = Rectangle::new(
                        Point::from((size.w / 4, size.h / 4)),
                        Size::from((size.w / 2, size.h / 2)),
                    );
                    *a = rect.loc;
                    *b = rect.loc + rect.size - Size::from((1, 1));
                }
                // Chrome layout happens at render time from the live viewport
                // (see render_output); buffers only carry color/id/commit.
            } else {
                buffers[4].resize(size.to_f64().to_logical(data.scale));
            }
        }
    }

    /// Renders the screenshot UI, magnifying captured-content layers through
    /// the live viewport (loupe). Panel stays screen-fixed; at 1x everything
    /// passes through. Selection stays in content pixels; export renders
    /// these same layers into the selection's screen image.
    pub fn render_output(
        &self,
        output: &Output,
        target: RenderTarget,
        zoom: ScreenshotPreviewZoom,
        push: &mut dyn FnMut(ScreenshotUiRenderElement),
    ) {
        let _span = tracy_client::span!("ScreenshotUi::render_output");

        let Self::Open {
            selection,
            output_data,
            show_pointer,
            button,
            open_anim,
            ..
        } = self
        else {
            panic!("screenshot UI must be open to render it");
        };

        let Some(output_data) = output_data.get(output) else {
            return;
        };

        let scale = output_data.scale;
        let progress = open_anim.clamped_value().clamp(0., 1.) as f32;

        // The help panel goes on top.
        if let Some((show, hide)) = &output_data.panel {
            let buffer = if *show_pointer { hide } else { show };
            let alpha = if button.is_dragging_selection() {
                0.3
            } else {
                0.9
            };
            let location = panel_location(output_data, buffer.texture().size())
                .to_f64()
                .to_logical(scale);

            let elem = PrimaryGpuTextureRenderElement(TextureRenderElement::from_texture_buffer(
                buffer.clone(),
                location,
                alpha * progress,
                None,
                None,
                Kind::Unspecified,
            ));
            push(ScreenshotUiRenderElement::Screenshot(elem))
        }

        // Chrome derives from one rounded screen rect with integer math, so
        // adjacent strips abut exactly: independent viewport rounding per
        // strip would open 1px hairlines as the focal animates.
        let viewport = zoom.viewport;
        if output == &selection.0 {
            let content = rect_from_corner_points(selection.1, selection.2);
            let display = export_rect(content, viewport, &zoom.view_ctx)
                .intersection(Rectangle::from_size(output_data.size))
                .unwrap_or_default();
            let border = to_physical_precise_round(scale, SELECTION_BORDER);
            for (buffer, (loc, size)) in zip(
                &output_data.buffers,
                selection_chrome(display, border, output_data.size),
            ) {
                let elem = SolidColorRenderElement::new(
                    buffer.id(),
                    Rectangle::new(
                        loc.to_f64().to_logical(scale),
                        size.to_f64().to_logical(scale),
                    ),
                    buffer.commit(),
                    buffer.color() * progress,
                    Kind::Unspecified,
                );
                push(ScreenshotUiRenderElement::SolidColor(elem));
            }
        } else if let Some(buffer) = output_data.buffers.get(4) {
            // Other outputs show a uniform fullscreen dim: viewport-invariant.
            let elem = SolidColorRenderElement::from_buffer(
                buffer,
                Point::from((0., 0.)),
                progress,
                Kind::Unspecified,
            );
            push(ScreenshotUiRenderElement::SolidColor(elem));
        }

        // The screenshot itself goes last.
        let index = match target {
            RenderTarget::Output => 0,
            RenderTarget::Screencast => 1,
            RenderTarget::ScreenCapture => 2,
        };
        let screenshot = &output_data.screenshot[index];

        Self::push_content(screenshot, zoom, *show_pointer, push);
    }

    pub fn capture(
        &self,
        renderer: &mut GlesRenderer,
        zoom: ScreenshotPreviewZoom,
    ) -> anyhow::Result<(Size<i32, Physical>, Vec<u8>)> {
        let _span = tracy_client::span!("ScreenshotUi::capture");

        let Self::Open {
            selection,
            output_data,
            show_pointer,
            ..
        } = self
        else {
            panic!("screenshot UI must be open to capture");
        };

        let data = &output_data[&selection.0];
        let content_rect = rect_from_corner_points(selection.1, selection.2);

        let screenshot = &data.screenshot[0];

        // WYSIWYG export: re-render the displayed layers into the
        // selection's screen image (never the chrome), then read back.
        let export = export_rect(content_rect, zoom.viewport, &zoom.view_ctx);
        if export.size.is_empty() {
            bail!("screenshot selection is empty after zoom mapping");
        }

        let mut elements = Vec::new();
        Self::push_content(screenshot, zoom, *show_pointer, &mut |elem| {
            elements.push(elem)
        });

        // Crop to the selection's screen image. Relocation happens in output
        // space, so regions outside the output (selected content currently
        // panned out of view) still render.
        let offset = export.loc.upscale(-1);
        let pixels = render_to_vec(
            renderer,
            export.size,
            zoom.view_ctx.scale,
            Transform::Normal,
            Fourcc::Abgr8888,
            elements
                .iter()
                .rev()
                .map(|elem| RelocateRenderElement::from_element(elem, offset, Relocate::Relative)),
        )?;

        Ok((export.size, pixels))
    }

    /// Previewed content layers (scene + frozen pointer) without chrome.
    ///
    /// Shared by preview and WYSIWYG export so the file matches the display.
    fn push_content(
        screenshot: &OutputScreenshot,
        zoom: ScreenshotPreviewZoom,
        show_pointer: bool,
        push: &mut dyn FnMut(ScreenshotUiRenderElement),
    ) {
        let ScreenshotPreviewZoom {
            viewport,
            view_ctx,
            filter,
            scale_with_zoom,
        } = zoom;
        // Render-path optimization, not an "is zoom active" predicate: the
        // factor <= 1 skip only avoids needless wrapping.
        let zoomed = viewport.factor() > 1.0;

        if show_pointer {
            if let Some(frozen) = screenshot.pointer.clone() {
                if zoomed {
                    let output_scale = view_ctx.scale;
                    // A scaling cursor magnifies with the live level. Otherwise
                    // the frozen graphic scales relative to its capture baseline,
                    // preserving the open-frame ratio.
                    let graphic_scale = if scale_with_zoom {
                        viewport.factor()
                    } else {
                        pointer_scale(viewport.factor(), screenshot.capture_level)
                    };
                    let tip = frozen.tip.to_f64();
                    let elem = ZoomElement::cursor(
                        frozen.element,
                        tip,
                        tip.to_logical(output_scale).assume_local(),
                        frozen.hotspot,
                        viewport,
                        graphic_scale,
                        view_ctx,
                        output_scale,
                    );
                    push(ScreenshotUiRenderElement::Zoomed(elem));
                } else {
                    push(ScreenshotUiRenderElement::Screenshot(frozen.element));
                }
            }
        }

        if zoomed {
            let elem = ZoomElement::from_element(
                screenshot.buffer.clone(),
                viewport,
                view_ctx,
                Point::from((0., 0.)),
                Relocate::Relative,
            )
            .with_filter(filter);
            push(ScreenshotUiRenderElement::Zoomed(elem));
        } else {
            push(ScreenshotUiRenderElement::Screenshot(
                screenshot.buffer.clone(),
            ));
        }
    }

    pub fn action(&self, raw: Keysym, mods: ModifiersState) -> Option<Action> {
        let Self::Open { button, .. } = self else {
            return None;
        };

        // Pressing Space while the button is down goes into origin moving rather than capture.
        if matches!(button, Button::Down { .. }) && raw == Keysym::space {
            return None;
        }

        action(raw, mods)
    }

    pub fn selection_output(&self) -> Option<&Output> {
        if let Self::Open {
            selection: (output, _, _),
            ..
        } = self
        {
            Some(output)
        } else {
            None
        }
    }

    pub fn output_size(&self, output: &Output) -> Option<(Size<i32, Physical>, f64, Transform)> {
        if let Self::Open { output_data, .. } = self {
            let data = output_data.get(output)?;
            Some((data.size, data.scale, data.transform))
        } else {
            None
        }
    }

    /// The pointer has moved to `point` relative to the current selection output.
    ///
    /// The point may be outside output bounds.
    pub fn pointer_motion(&mut self, point: Point<i32, Physical>, slot: Option<TouchSlot>) {
        let Self::Open {
            selection,
            output_data,
            button:
                Button::Down {
                    touch_slot,
                    on_capture_button,
                    last_pos,
                    move_state,
                },
            ..
        } = self
        else {
            return;
        };

        if *touch_slot != slot {
            return;
        }

        last_pos.1 = point;

        if *on_capture_button {
            return;
        }

        if let Some(move_state) = move_state {
            // The cursor offset is relative to selection.1.
            let delta = point - (selection.1 + move_state.pointer_offset);

            let desired = rect_from_corner_points(selection.1 + delta, selection.2 + delta);
            let bounds = Rectangle::from_size(output_data[&selection.0].size - desired.size);
            let clamped_loc = desired.loc.constrain(bounds);

            let delta = clamped_loc - rect_from_corner_points(selection.1, selection.2).loc;
            selection.1 += delta;
            selection.2 += delta;
        } else {
            let size = output_data[&selection.0].size;
            selection.2 = Point::new(point.x.clamp(0, size.w - 1), point.y.clamp(0, size.h - 1));
        }

        self.update_buffers();
    }

    pub fn pointer_down(
        &mut self,
        output: Output,
        point: Point<i32, Physical>,
        slot: Option<TouchSlot>,
        move_existing: bool,
    ) -> bool {
        let Self::Open {
            selection,
            output_data,
            show_pointer,
            button,
            ..
        } = self
        else {
            return false;
        };

        // Check if this is a second touch (different slot) while already dragging.
        if let Some(new_slot) = slot {
            if let Button::Down {
                on_capture_button: false,
                move_state,
                last_pos,
                ..
            } = button
            {
                if move_state.is_none() {
                    *move_state = Some(MoveState {
                        pointer_offset: last_pos.1 - selection.1,
                        touch_slot: Some(new_slot),
                    });
                }
            }
        }

        if button.is_down() {
            return false;
        }

        if move_existing {
            if output != selection.0 {
                return false;
            }

            *button = Button::Down {
                touch_slot: slot,
                on_capture_button: false,
                last_pos: (output, point),
                move_state: Some(MoveState {
                    pointer_offset: point - selection.1,
                    touch_slot: slot,
                }),
            };
            return true;
        }

        let Some(output_data) = output_data.get(&output) else {
            return false;
        };

        if let Some((show, hide)) = &output_data.panel {
            let buffer = if *show_pointer { hide } else { show };
            let panel_size = buffer.texture().size();
            let location = panel_location(output_data, panel_size);

            if is_within_capture_button(output_data.scale, panel_size, point - location) {
                *button = Button::Down {
                    touch_slot: slot,
                    on_capture_button: true,
                    last_pos: (output, point),
                    move_state: None,
                };
                return false;
            }
        }

        *button = Button::Down {
            touch_slot: slot,
            on_capture_button: false,
            last_pos: (output.clone(), point),
            move_state: None,
        };

        let point = Point::new(
            point.x.clamp(0, output_data.size.w - 1),
            point.y.clamp(0, output_data.size.h - 1),
        );
        *selection = (output, point, point);

        self.update_buffers();

        true
    }

    pub fn pointer_up(&mut self, slot: Option<TouchSlot>) -> Option<bool> {
        let Self::Open {
            selection,
            output_data,
            button,
            show_pointer,
            ..
        } = self
        else {
            return None;
        };

        let Button::Down {
            touch_slot,
            on_capture_button,
            ref last_pos,
            ref mut move_state,
            ..
        } = *button
        else {
            return None;
        };

        if touch_slot != slot {
            // This is not our main touch, but it might be the move touch. If so, stop the move.
            if let Some(state) = move_state {
                if state.touch_slot.is_some_and(|m_slot| Some(m_slot) == slot) {
                    *move_state = None;
                }
            };

            return None;
        }

        let last_pos = last_pos.clone();
        *button = Button::Up;

        // Check if we released still on the capture button.
        if on_capture_button {
            let (output, point) = last_pos;

            #[allow(clippy::question_mark)]
            let Some(output_data) = output_data.get(&output) else {
                return None;
            };

            if let Some((show, hide)) = &output_data.panel {
                let buffer = if *show_pointer { hide } else { show };
                let panel_size = buffer.texture().size();
                let location = panel_location(output_data, panel_size);

                if is_within_capture_button(output_data.scale, panel_size, point - location) {
                    return Some(true);
                }
            }
        }

        // Check if the resulting selection is zero-sized, and try to come up with a small
        // default rectangle.
        let (output, a, b) = selection;
        let mut rect = rect_from_corner_points(*a, *b);
        if rect.size.is_empty() || rect.size == Size::from((1, 1)) {
            let data = &output_data[output];
            rect = Rectangle::new(
                Point::from((rect.loc.x - 16, rect.loc.y - 16)),
                Size::from((32, 32)),
            )
            .intersection(Rectangle::from_size(data.size))
            .unwrap_or_default();
            *a = rect.loc;
            *b = rect.loc + rect.size - Size::from((1, 1));
        }

        self.update_buffers();

        Some(false)
    }
}

impl OutputScreenshot {
    pub(crate) fn from_textures(
        renderer: &mut GlesRenderer,
        scale: Scale<f64>,
        texture: GlesTexture,
        pointer: Option<CapturedPointer>,
        capture_level: f64,
    ) -> Self {
        let buffer = PrimaryGpuTextureRenderElement(TextureRenderElement::from_texture_buffer(
            TextureBuffer::from_texture(renderer, texture, scale, Transform::Normal, Vec::new()),
            (0., 0.),
            1.,
            None,
            None,
            Kind::Unspecified,
        ));

        let pointer = pointer.map(
            |CapturedPointer {
                 texture,
                 geo,
                 hotspot,
                 tip,
             }| {
                let element =
                    PrimaryGpuTextureRenderElement(TextureRenderElement::from_texture_buffer(
                        TextureBuffer::from_texture(
                            renderer,
                            texture,
                            scale,
                            Transform::Normal,
                            Vec::new(),
                        ),
                        geo.to_f64().to_logical(scale).loc,
                        1.,
                        None,
                        None,
                        Kind::Unspecified,
                    ));
                FrozenPointer {
                    element,
                    hotspot,
                    tip,
                }
            },
        );

        Self {
            buffer,
            pointer,
            capture_level,
        }
    }
}

/// Frozen-pointer scale: `live / capture`, unfloored, so the open-frame
/// appearance ratio holds at every level.
fn pointer_scale(live_level: f64, capture_level: f64) -> f64 {
    live_level / capture_level
}

/// Selection chrome rectangles from one rounded screen rect.
///
/// Integer math from a single rect keeps adjacent strips exactly abutting under any viewport — no
/// hairlines. Requires `display` clamped to the output (no negative sizes).
fn selection_chrome(
    display: Rectangle<i32, Physical>,
    border: i32,
    output: Size<i32, Physical>,
) -> [(Point<i32, Physical>, Size<i32, Physical>); 8] {
    let (x, y) = (display.loc.x, display.loc.y);
    let (w, h) = (display.size.w, display.size.h);
    let strip = |x, y, w, h| (Point::from((x, y)), Size::from((w, h)));
    [
        // Borders around the selection.
        strip(x - border, y - border, w + border * 2, border),
        strip(x - border, y + h, w + border * 2, border),
        strip(x - border, y, border, h),
        strip(x + w, y, border, h),
        // Dimming for the rest of the output.
        strip(0, 0, output.w, y),
        strip(0, y + h, output.w, output.h - y - h),
        strip(0, y, x, h),
        strip(x + w, y, output.w - x - w, h),
    ]
}

/// Content-space selection to its screen image under the viewport.
/// Rounds like `ZoomElement::geometry` so the crop aligns with display.
fn export_rect(
    content: Rectangle<i32, Physical>,
    viewport: ViewportTransform,
    view_ctx: &OutputViewCtx,
) -> Rectangle<i32, Physical> {
    let local = view_ctx.physical_rect_to_local(content.to_f64());
    let mapped = viewport.apply_rect(local);
    let physical = view_ctx.local_rect_to_physical(mapped);
    let loc = physical.loc.to_i32_round();
    let bottom_right = (physical.loc + physical.size).to_i32_round();
    Rectangle::new(loc, (bottom_right - loc).to_size())
}

fn action(raw: Keysym, mods: ModifiersState) -> Option<Action> {
    if raw == Keysym::Escape {
        return Some(Action::CancelScreenshot);
    }

    if mods.alt || mods.shift {
        return None;
    }

    if !mods.ctrl && (raw == Keysym::space || raw == Keysym::Return) {
        return Some(Action::ConfirmScreenshot {
            write_to_disk: true,
        });
    }
    if mods.ctrl && raw == Keysym::c {
        return Some(Action::ConfirmScreenshot {
            write_to_disk: false,
        });
    }

    if !mods.ctrl && raw == Keysym::p {
        return Some(Action::ScreenshotTogglePointer);
    }

    None
}

pub fn rect_from_corner_points(
    a: Point<i32, Physical>,
    b: Point<i32, Physical>,
) -> Rectangle<i32, Physical> {
    let x1 = min(a.x, b.x);
    let y1 = min(a.y, b.y);
    let x2 = max(a.x, b.x);
    let y2 = max(a.y, b.y);
    // We're adding + 1 because the pointer is clamped to output size - 1, so to get the full
    // screen worth of selection we must add back that + 1.
    Rectangle::from_extremities((x1, y1), (x2 + 1, y2 + 1))
}

fn panel_location(output_data: &OutputData, panel_size: Size<i32, Buffer>) -> Point<i32, Physical> {
    let scale = output_data.scale;
    let padding: i32 = to_physical_precise_round(scale, PADDING);
    let x = max(0, (output_data.size.w - panel_size.w) / 2);
    let y = max(0, output_data.size.h - panel_size.h - padding * 2);
    Point::from((x, y))
}

fn is_within_capture_button(
    scale: f64,
    panel_size: Size<i32, Buffer>,
    pos_within_panel: Point<i32, Physical>,
) -> bool {
    let padding: i32 = to_physical_precise_round(scale, PADDING);
    let radius = to_physical_precise_round::<i32>(scale, RADIUS) - 2;

    let xc = padding + radius;
    let yc = panel_size.h / 2;
    let pos = pos_within_panel;

    (pos.x - xc) * (pos.x - xc) + (pos.y - yc) * (pos.y - yc) <= radius * radius
}

fn render_panel(
    renderer: &mut GlesRenderer,
    scale: f64,
    text: &str,
) -> anyhow::Result<TextureBuffer<GlesTexture>> {
    let _span = tracy_client::span!("screenshot_ui::render_panel");

    let padding: i32 = to_physical_precise_round(scale, PADDING);
    // Keep the border width even to avoid blurry edges.
    let border_width = (f64::from(BORDER) / 2. * scale).round() * 2.;
    let half_border_width = (border_width / 2.) as i32;
    let radius: i32 = to_physical_precise_round(scale, RADIUS);
    let circle_stroke: f64 = to_physical_precise_round(scale, 2.);

    // Add 2 px of spacing to separate the backgrounds of the "Space" and "P" keys.
    let spacing = to_physical_precise_round::<i32>(scale, 2) * 1024;

    let mut font = FontDescription::from_string(FONT);
    font.set_absolute_size(to_physical_precise_round(scale, font.size()));

    let surface = ImageSurface::create(cairo::Format::ARgb32, 0, 0)?;
    let cr = cairo::Context::new(&surface)?;
    let layout = pangocairo::functions::create_layout(&cr);
    layout.context().set_round_glyph_positions(false);
    layout.set_font_description(Some(&font));
    layout.set_alignment(Alignment::Left);
    layout.set_markup(text);
    layout.set_spacing(spacing);

    let (mut width, mut height) = layout.pixel_size();

    width += padding + radius * 2 + padding - half_border_width + padding;
    height = max(height, radius * 2);
    height += padding * 2;

    let surface = ImageSurface::create(cairo::Format::ARgb32, width, height)?;
    let cr = cairo::Context::new(&surface)?;
    cr.set_source_rgb(0.1, 0.1, 0.1);
    cr.paint()?;

    let padding = f64::from(padding);
    let half_border_width = f64::from(half_border_width);
    let r = f64::from(radius);

    let yc = f64::from(height / 2);

    cr.new_sub_path();
    cr.arc(padding + r, yc, r, 0., TAU);
    cr.set_source_rgb(1., 1., 1.);
    cr.fill()?;

    cr.new_sub_path();
    cr.arc(padding + r, yc, r - circle_stroke, 0., TAU);
    cr.set_source_rgb(0.1, 0.1, 0.1);
    cr.fill()?;

    cr.new_sub_path();
    cr.arc(padding + r, yc, r - circle_stroke * 2., 0., TAU);
    cr.set_source_rgb(1., 1., 1.);
    cr.fill()?;

    cr.move_to(padding + r * 2. + padding - half_border_width, padding);

    let layout = pangocairo::functions::create_layout(&cr);
    layout.context().set_round_glyph_positions(false);
    layout.set_font_description(Some(&font));
    layout.set_alignment(Alignment::Left);
    layout.set_markup(text);
    layout.set_spacing(spacing);

    cr.set_source_rgb(1., 1., 1.);
    pangocairo::functions::show_layout(&cr, &layout);

    cr.move_to(0., 0.);
    cr.line_to(width.into(), 0.);
    cr.line_to(width.into(), height.into());
    cr.line_to(0., height.into());
    cr.line_to(0., 0.);
    cr.set_source_rgb(0.3, 0.3, 0.3);
    cr.set_line_width(border_width);
    cr.stroke()?;
    drop(cr);

    let data = surface.take_data().unwrap();
    let buffer = TextureBuffer::from_memory(
        renderer,
        &data,
        Fourcc::Argb8888,
        (width, height),
        false,
        scale,
        Transform::Normal,
        Vec::new(),
    )?;

    Ok(buffer)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::geometry::{Local, RectExt};

    #[test]
    fn scale_tracks_capture_baseline() {
        // Opened unzoomed, zoomed to 2x: full relative scale.
        assert_eq!(pointer_scale(2., 1.), 2.);
        // Opened at 2x, zoomed to 3x: scales from the capture baseline.
        assert_eq!(pointer_scale(3., 2.), 1.5);
        // Unchanged level: captured size.
        assert_eq!(pointer_scale(2., 2.), 1.);
        // Zoomed back out below capture: shrinks proportionally, keeping the
        // open-frame appearance ratio instead of looming over the scene.
        assert_eq!(pointer_scale(1., 2.), 0.5);
        assert_eq!(pointer_scale(1.5, 2.), 0.75);
    }

    #[test]
    fn pointer_scales_relative_to_capture() {
        // Opened at 2x, now at 3x: graphic scales 1.5x around its tip.
        let viewport = ViewportTransform::new((0., 0.).into(), 3.);
        let scale = pointer_scale(viewport.factor(), 2.);
        let tip = Point::<f64, Physical>::from((10., 10.));
        let display = Point::<f64, Local>::from((10., 10.));
        let (final_pos, wrapper) =
            viewport.place_cursor(tip, display, (4, 4).into(), scale, Scale::from(1.));

        // Tip (10, 10) -> (30, 30); hotspot (4, 4) * 1.5 -> (6, 6).
        assert_eq!(wrapper.factor(), 1.5);
        assert_eq!(final_pos, Point::<f64, Physical>::from((24., 24.)));
    }

    fn export_view_ctx(scale: f64) -> OutputViewCtx {
        OutputViewCtx::new(
            Rectangle::new((0., 0.).into(), (1920., 1080.).into()).assume_global(),
            Rectangle::new((0., 0.).into(), (1920., 1080.).into()).assume_local(),
            Transform::Normal,
            Scale::from(scale),
        )
    }

    #[test]
    fn export_rect_is_identity_without_zoom() {
        let content = Rectangle::new((10, 20).into(), (100, 80).into());
        let ctx = export_view_ctx(1.);
        assert_eq!(
            export_rect(content, ViewportTransform::identity(), &ctx),
            content
        );
    }

    #[test]
    fn export_rect_matches_displayed_image() {
        // Pure 2x around the origin: the screen image is the doubled rect.
        let viewport = ViewportTransform::new((0., 0.).into(), 2.);
        let ctx = export_view_ctx(1.);
        let content = Rectangle::new((10, 20).into(), (100, 80).into());
        assert_eq!(
            export_rect(content, viewport, &ctx),
            Rectangle::new((20, 40).into(), (200, 160).into())
        );
    }

    #[test]
    fn export_rect_follows_focal_and_scale() {
        let viewport = ViewportTransform::new((100., 80.).into(), 2.);
        let ctx = export_view_ctx(1.);
        let content = Rectangle::new((110, 90).into(), (20, 10).into());
        // Loc (110, 90) -> (120, 100); BR (130, 100) -> (160, 120).
        assert_eq!(
            export_rect(content, viewport, &ctx),
            Rectangle::new((120, 100).into(), (40, 20).into())
        );
    }

    #[test]
    fn chrome_tiles_output_without_seams() {
        let display = Rectangle::new((100, 80).into(), (400, 300).into());
        let output = Size::from((1920, 1080));
        let strips = selection_chrome(display, 2, output);

        // Borders frame the hole; dimming abuts it exactly.
        assert_eq!(strips[0], ((98, 78).into(), (404, 2).into()));
        assert_eq!(strips[2], ((98, 80).into(), (2, 300).into()));
        assert_eq!(strips[4], ((0, 0).into(), (1920, 80).into()));
        assert_eq!(strips[6], ((0, 80).into(), (100, 300).into()));
        assert_eq!(strips[7], ((500, 80).into(), (1420, 300).into()));

        // Joints align: border bottom meets hole top, dim meets hole edge.
        assert_eq!(strips[0].0.y + strips[0].1.h, display.loc.y);
        assert_eq!(strips[2].0.x + strips[2].1.w, display.loc.x);
        assert_eq!(strips[4].1.h, display.loc.y);
        assert_eq!(strips[6].0.x + strips[6].1.w, display.loc.x);

        // Dimming plus hole covers the output exactly once.
        let dim_area: i32 = strips[4..].iter().map(|(_, s)| s.w * s.h).sum();
        let hole = display.size.w * display.size.h;
        assert_eq!(dim_area + hole, 1920 * 1080);
    }
}
