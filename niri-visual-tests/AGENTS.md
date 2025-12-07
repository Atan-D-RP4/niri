## Build & Test Commands
- **Build**: `cargo build` (or `cargo build --release`)
- **Lint**: `cargo clippy --all-targets --all-features`
- **Format**: `cargo +nightly fmt --all` (check: `cargo +nightly fmt --all -- --check`)
- **Test all**: `cargo test`
- **Single test**: `cargo test test_name` or `cargo test module::test_name`
- **Update snapshots**: `cargo insta review` (uses insta for snapshot testing)

## Code Style
- **Imports**: Module-level granularity, grouped as std/external/crate (see rustfmt.toml)
- **Comments**: Wrap at 100 chars
- **Naming**: snake_case for functions/variables, CamelCase for types
- **Errors**: Use `anyhow` for error handling with `.context()` for context
- **Commits**: Small, focused, self-contained; each must build and pass tests
- **Clippy**: `new_without_default` is allowed; interior mutability ignored for Smithay types

---

## Niri Visual Tests Crate Architecture

Comprehensive architecture of the GTK-based visual testing framework. Covers test infrastructure [1a-1f], test case system [2a-2e], rendering pipeline [3a-3e], and animation control [4a-4e].

The niri-visual-tests crate provides a graphical testing environment using GTK 4 and Adwaita for testing Niri's layout engine, window tiling, and rendering without a running compositor. It uses Smithay for rendering and tests gradient color spaces, window layouts, and animations.

### 1. Application Structure

### 1a. GTK Application Initialization (`main.rs:44`)

Creates the Adwaita application with startup and activation handlers

```text
let app = adw::Application::new(None::<&str>, gio::ApplicationFlags::NON_UNIQUE);
app.connect_startup(on_startup);
app.connect_activate(build_ui);
app.run()
```

**Purpose**: Sets up the GTK/Adwaita application lifecycle. NON_UNIQUE allows multiple instances to run simultaneously.

### 1b. CSS Styling (`main.rs:50`)

Loads custom CSS stylesheet for the visual test interface

```text
fn on_startup(_app: &adw::Application) {
    let provider = gtk::CssProvider::new();
    provider.load_from_string(include_str!("../resources/style.css"));
```

**Purpose**: Applies consistent styling to GTK widgets including the animation control bar and test case layout.

### 1c. Main Window Construction (`main.rs:149`)

Builds the main application window with split pane layout

```text
let content = adw::NavigationPage::new(&content_view, "Test Cases");
let sidebar = gtk::ListBox::new(); // Sidebar with test case list
```

**Purpose**: Creates a split-pane interface with test case selection sidebar and content area.

### 1d. Test Case Stack (`main.rs:64`)

GTK Stack widget for switching between test case views

```text
let stack = gtk::Stack::new();
// ... add test cases to stack
let anim_adjustment = gtk::Adjustment::new(1., 0., 10., 0.1, 0.5, 0.);
```

**Purpose**: Manages multiple test case views with seamless switching. Animation adjustment controls slowdown factor for all active test cases.

### 1e. Environment Logging (`main.rs:36`)

Configures Rust logging with environment variable support

```text
let directives = env::var("RUST_LOG")
    .unwrap_or_else(|_| "niri-visual-tests=debug,niri=debug".to_owned());
let env_filter = EnvFilter::builder().parse_lossy(directives);
```

**Purpose**: Sets up structured logging to help debug rendering and layout issues.

### 1f. Test Case Registration (`main.rs:73`)

Registers test cases with the UI for selection

```text
impl S {
    fn add<T: TestCase + 'static>(&self, make: impl Fn(Args) -> T + 'static, title: &str) {
        let view = SmithayView::new(make, &self.anim_adjustment);
        self.stack.add_titled(&view, None, title);
    }
}
```

**Purpose**: Generic registration method allowing test cases to be added to the stack with human-readable titles.

### 2. Test Case System

### 2a. TestCase Trait (`cases/mod.rs`)

Core trait that all test cases must implement

```text
pub trait TestCase {
    fn render(&mut self, renderer: &mut GlowRenderer, size: Size<i32, Physical>) -> Vec<Element>;
    fn are_animations_ongoing(&self) -> bool;
}
```

**Purpose**: Defines the interface for test cases. Each test case can render elements and report animation status.

### 2b. Window Test Case (`cases/window.rs`)

Tests rendering of individual windows with various size configurations

```text
pub struct Window {
    // Test window with fixed or freeform size
    pub inner: TestWindow,
}

impl Window {
    pub fn freeform(args: Args) -> Self { /* ... */ }
    pub fn fixed_size(args: Args) -> Self { /* ... */ }
    pub fn fixed_size_with_csd_shadow(args: Args) -> Self { /* ... */ }
}
```

**Purpose**: Tests different window sizing modes including freeform, fixed size, and CSD (Client-Side Decoration) shadow rendering.

### 2c. Tile Test Case (`cases/tile.rs`)

Tests the basic tiling column with opening/closing animations

```text
pub struct Tile {
    pub column: Column,
}

impl Tile {
    pub fn freeform(args: Args) -> Self { /* ... */ }
    pub fn freeform_open(args: Args) -> Self { /* ... */ }
    pub fn fixed_size_open(args: Args) -> Self { /* ... */ }
}
```

**Purpose**: Tests tiling column rendering with different window sizes and open/close animations.

### 2d. Layout Test Case (`cases/layout.rs`)

Tests complex multi-column layouts with opening and closing transitions

```text
pub struct Layout {
    pub layout: ColumnLayout,
}

impl Layout {
    pub fn open_in_between(args: Args) -> Self { /* ... */ }
    pub fn open_multiple_quickly(args: Args) -> Self { /* ... */ }
    pub fn open_to_the_left(args: Args) -> Self { /* ... */ }
}
```

**Purpose**: Tests complex layout scenarios including opening windows in-between existing columns and rapid window spawning.

### 2e. Gradient Test Cases (`cases/gradient_*.rs`)

Tests gradient rendering with different color spaces and interpolation modes

```text
pub struct GradientSrgb { /* sRGB color space */ }
pub struct GradientOklab { /* Oklab color space */ }
pub struct GradientOklch { /* Oklch color space */ }
// ... additional color space variants
```

**Purpose**: Validates gradient rendering across multiple color spaces (sRGB, sRGB-linear, Oklab, Oklch) and interpolation modes (increasing/decreasing hue, shorter/longer hue arc).

### 3. Rendering Pipeline

### 3a. SmithayView Creation (`main.rs:74`)

Instantiates a Smithay-based renderer for each test case

```text
let view = SmithayView::new(make, &self.anim_adjustment);
self.stack.add_titled(&view, None, title);
```

**Purpose**: Creates a new rendering view that initializes Smithay's GL renderer and test case instance.

### 3b. SmithayView Structure (`smithay_view.rs`)

GTK custom widget wrapping Smithay renderer

```text
pub struct SmithayView {
    imp: SmithayViewPrivate,
}

struct SmithayViewPrivate {
    gl_area: gtk::GLArea,
    renderer: RefCell<Option<GlowRenderer>>,
    case: RefCell<Box<dyn TestCase>>,
    anim_clock: Cell<Clock>,
}
```

**Purpose**: Encapsulates the GL rendering context, Smithay renderer instance, test case, and animation clock.

### 3c. GL Area Setup (`smithay_view.rs`)

Initializes GTK's OpenGL rendering context

```text
let gl_area = gtk::GLArea::new();
gl_area.set_error_handler(|e| { /* handle errors */ });
```

**Purpose**: Sets up OpenGL context for hardware-accelerated rendering via Smithay.

### 3d. Render Cycle Entry (`smithay_view.rs:124`)

Main rendering function called each frame

```text
fn render(&self, _gl_context: &gdk::GLContext) -> anyhow::Result<()> {
    let size = self.imp.gl_area.allocated_size();
    let mut renderer = self.imp.renderer.borrow_mut();
    let mut case = self.imp.case.borrow_mut();
    // Render test case
}
```

**Purpose**: Called by GTK when the GL area needs redrawing. Borrows renderer and test case to perform rendering.

### 3e. Element Rendering (`smithay_view.rs:174`)

Renders test case and collects render elements

```text
let elements = case.render(&mut renderer, Size::from(size));
```

**Purpose**: Asks the test case to generate renderable elements (windows, tiles, gradients, etc.).

### 3f. Frame Rendering and Submission (`smithay_view.rs:210`)

Draws elements to framebuffer with damage tracking

```text
element.draw(&mut frame, src, dst, &[damage], &[])
```

**Purpose**: Iterates through elements and renders each to the GL framebuffer. Tracks damage regions for efficient updates.

### 4. Animation Control and Clock Management

### 4a. Animation Adjustment Creation (`main.rs:65`)

GTK Adjustment widget for animation slowdown control

```text
let anim_adjustment = gtk::Adjustment::new(
    1.,   // initial value (1x speed)
    0.,   // lower bound
    10.,  // upper bound (10x slowdown)
    0.1,  // step increment
    0.5,  // page increment
    0.,   // page size
);
```

**Purpose**: Creates a slider control ranging from no slowdown (1.0) to 10x slowdown (10.0).

### 4b. Animation Control Bar (`main.rs:138`)

UI widget for animation speed adjustment

```text
let anim_scale = gtk::Scale::new(gtk::Orientation::Horizontal, Some(&anim_adjustment));
anim_scale.set_hexpand(true);

let anim_control_bar = gtk::Box::new(gtk::Orientation::Horizontal, 6);
anim_control_bar.append(&gtk::Label::new(Some("Slowdown")));
anim_control_bar.append(&anim_scale);
```

**Purpose**: Creates the UI for animation speed control at the bottom of the window.

### 4c. Value Change Handler (`smithay_view.rs:272`)

Connects slider changes to animation clock updates

```text
anim_adjustment.connect_value_changed({
    let clock = imp.anim_clock.clone();
    move |adj| {
        let rate = if instantly { 1.0 } else { 1.0 / adj.value().max(0.001) };
        clock.borrow_mut().set_rate(rate);
        imp.gl_area.queue_draw();
    }
});
```

**Purpose**: When user moves the slider, updates the animation clock's rate and queues a redraw.

### 4d. Rate Calculation (`smithay_view.rs:281`)

Converts adjustment value to animation clock rate

```text
let rate = if instantly { 1.0 } else { 1.0 / adj.value().max(0.001) };
```

**Purpose**: Maps adjustment value (slowdown factor) to clock rate (speed multiplier). Clamps minimum value to prevent division by zero.

### 4e. Clock Rate Application (`smithay_view.rs:283`)

Updates the animation clock with computed rate

```text
clock.set_rate(rate);
```

**Purpose**: Sets the new rate on the clock. All animations using this clock will advance at the new rate.

### 4f. Animation Frame Loop (`smithay_view.rs:90`)

Continuously queues redraws while animations are active

```text
if case.are_animations_ongoing() {
    imp.gl_area.queue_draw();
}
```

**Purpose**: If the test case has ongoing animations, requests a redraw. GTK will call render again, advancing animations by the clock delta.

### 5. Test Coverage

The crate tests critical rendering and layout systems:

- **Window rendering**: Different size configurations and CSD shadows
- **Column tiling**: Single and multi-column layouts with animations
- **Complex layouts**: In-between window openings and rapid spawning
- **Gradient rendering**: Multiple color spaces and interpolation modes
- **Animation timing**: Animation speeds and frame-accurate rendering

### 6. Architecture Benefits

- **Offline testing**: Tests run without a full Niri instance
- **Reproducible**: Test cases are deterministic and scriptable
- **Isolated**: Each test case is independent
- **Interactive**: Animation slowdown allows frame-by-frame inspection
- **Visual feedback**: Immediate visual confirmation of rendering correctness
