use silk_engine::prelude::*;

fn init(event_loop: Res<EventLoop>, mut cmd: Commands) {
    let vulkan = Vulkan::new(VulkanConfig::default()).unwrap();
    let mut gfx = Gfx::new(&vulkan).unwrap();

    let window = gfx.create_window(
        &event_loop,
        WindowAttributes::default().with_inner_size(PhysicalSize::new(1280, 720)),
    );

    gfx.load_img("cursor.qoi");

    let sfx = Sfx::new();
    // let mut src = sfx.load("steingen");
    // sfx.play(&mut src);

    cmd.insert_resource(window);
    cmd.insert_resource(Input::new());
    cmd.insert_resource(gfx);
    cmd.insert_resource(sfx);
}

#[inline_tweak::tweak_fn]
fn update(mut gfx: ResMut<Gfx>, mut input: ResMut<Input>, mut window: ResMut<Window>) {
    if input.key_pressed(Key::Escape) {
        std::process::exit(0);
    }

    gfx.gradient_dir = 0.0;
    gfx.rgb(255, 0, 0);
    gfx.gradient_rgb(255, 255, 0);
    gfx.rect(Pc(0.2), Pc(0.8), Pc(0.1), Pc(0.1));

    gfx.rgb(255, 255, 0);
    gfx.gradient_rgb(0, 255, 0);
    gfx.rect(Pc(0.3), Pc(0.8), Pc(0.05), Pc(0.1));

    gfx.rgb(0, 255, 0);
    gfx.gradient_rgb(0, 255, 255);
    gfx.rect(Pc(0.35), Pc(0.8), Pc(0.05), Pc(0.1));

    gfx.rgb(0, 255, 255);
    gfx.gradient_rgb(0, 0, 255);
    gfx.rect(Pc(0.4), Pc(0.8), Pc(0.1), Pc(0.1));

    gfx.stroke_color = [255, 0, 0, 255];
    gfx.stroke_width = 0.25;
    gfx.rgb(255, 255, 255);
    gfx.blur = 1.0;
    gfx.circle(Pc(0.7), Pc(0.5), Pc(0.04));

    gfx.blur = -1.0;
    gfx.circle(Pc(0.8), Pc(0.5), Pc(0.04));

    gfx.blur = 0.0;
    gfx.circle(Pc(0.7), Pc(0.4), Pc(0.04));

    gfx.blur = 1.0;
    gfx.stroke_blur = 1.0;
    gfx.circle(Pc(0.8), Pc(0.4), Pc(0.04));

    gfx.blur = -1.0;
    gfx.stroke_blur = 1.0;
    gfx.circle(Pc(0.7), Pc(0.3), Pc(0.04));

    gfx.blur = 0.0;
    gfx.stroke_blur = 1.0;
    gfx.circle(Pc(0.8), Pc(0.3), Pc(0.04));

    gfx.bold = 1.0;
    gfx.stroke_width = 0.5;
    gfx.stroke_blur = 0.0;
    gfx.blur = 0.0;
    gfx.stroke_rgb(255, 111, 32);
    gfx.rgb(32, 123, 222);
    // gfx.blur = -1.0;
    // gfx.stroke_blur = 0.5;
    gfx.font("zenmaru");
    gfx.text("鬱龍龍龜鷲鷹魁鬼鉄鬼こんにちは", Px(50), Px(110), Px(24));
    gfx.font("segoe-ui");
    gfx.text("stuff be workin", Px(150), Px(250), Px(24));
    gfx.bold = 0.0;
    gfx.stroke_width = 0.0;
    gfx.stroke_blur = 0.0;
    gfx.blur = 0.0;
    gfx.text(
        "quick brown fox jumped over a lazy dog",
        Pc(0.1),
        Px(190),
        Px(8),
    );
    gfx.rgb(255, 255, 255);
    gfx.no_gradient();
    gfx.font("zenmaru");
    gfx.text("鬱龍龍龜鷲鷹魁鬼鉄鬼こんにちは", Pc(0.7), Pc(0.7), Px(8));

    gfx.atlas();
    gfx.rect(Pc(0.4), Pc(0.4), Px(1024), Px(1024));
    gfx.no_img();
    gfx.superellipse = (input.mouse_x() / window.width() as f32 * 2.0 - 1.0
        + input.mouse_y() / window.height() as f32 * 2.0
        - 1.0)
        * 2.0;
    gfx.stroke_width = 0.5;
    gfx.rrect(Px(730), Px(30), Px(150), Px(150), 1.0);
    gfx.rrect(Px(130), Px(30), Px(430), Px(150), 0.5);
    gfx.rrect(Px(30), Px(130), Px(60), Px(150), 0.2);

    gfx.circle(Px(30), Px(530), Px(30));

    gfx.render(&mut window);

    window.request_redraw();
    input.reset();
}

fn on_event(
    event: On<WindowEvent>,
    event_loop: Res<EventLoop>,
    window: Res<Window>,
    mut input: ResMut<Input>,
) {
    if event.window_id == window.id() {
        let event = &event.window_event;
        let outer_pos = window.outer_position().unwrap_or_default();
        let window_height = window.inner_size().height;
        input.event(event, outer_pos.x, outer_pos.y, window_height);
        match event {
            WinitEvent::Destroyed | WinitEvent::CloseRequested => {
                event_loop.exit();
            }
            WinitEvent::MouseInput { state, button, .. } => {
                if *button == Mouse::Left && state.is_pressed() {
                    _ = window.drag_window();
                }
            }
            _ => {}
        }
    }
}

fn main() -> ResultAny {
    let mut app = App::new();
    app.add_plugins(Engine)
        .insert_resource(EngineConfig::default())
        .add_systems(Startup, init)
        .add_systems(Update, update)
        .add_observer(on_event);
    app.run();

    Ok(())
}
