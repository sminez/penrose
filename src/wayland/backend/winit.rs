use crate::wayland::{
    state::{CalloopData, SEAT_NAME},
    WaylandState,
};
use smithay::{
    backend::{
        renderer::{
            damage::OutputDamageTracker, element::surface::WaylandSurfaceRenderElement,
            gles::GlesRenderer,
        },
        winit::{self, WinitEvent, WinitGraphicsBackend},
    },
    desktop::space::render_output,
    output::{Mode, Output, PhysicalProperties, Subpixel},
    reexports::{calloop::EventLoop, wayland_server::DisplayHandle},
    utils::{Physical, Rectangle, Size, Transform},
};
use std::{env, time::Duration};

const REFRESH_RATE_MHZ: i32 = 60_000;
const WAYLAND_DISPLAY: &str = "WAYLAND_DISPLAY";

pub fn init_winit_backend(
    event_loop: &mut EventLoop<'_, CalloopData>,
    data: &mut CalloopData,
) -> Result<(), Box<dyn std::error::Error>> {
    let display_handle = &mut data.display_handle;
    let state = &mut data.state;

    let (mut backend, winit) = winit::init()?;
    let mode = Mode {
        size: backend.window_size(),
        refresh: REFRESH_RATE_MHZ,
    };

    let output = Output::new(
        SEAT_NAME.to_string(),
        PhysicalProperties {
            size: (0, 0).into(),
            subpixel: Subpixel::Unknown,
            make: "Penrose".into(),
            model: "Winit".into(),
        },
    );

    let _global = output.create_global::<WaylandState>(display_handle);
    output.change_current_state(
        Some(mode),
        Some(Transform::Flipped180),
        None,
        Some((0, 0).into()),
    );
    output.set_preferred(mode);
    state.space.map_output(&output, (0, 0));

    let mut damage_tracker = OutputDamageTracker::from_output(&output);

    env::set_var(WAYLAND_DISPLAY, &state.socket_name);

    event_loop
        .handle()
        .insert_source(winit, move |event, _, data| {
            use WinitEvent::*;

            let display = &mut data.display_handle;
            let state = &mut data.state;

            match event {
                Redraw => handle_redraw(state, &mut backend, &output, display, &mut damage_tracker),
                Resized { size, .. } => handle_resized(&output, size),
                Input(event) => state.process_input_event(event),
                CloseRequested => state.loop_signal.stop(),
                _ => (),
            };
        })?;

    Ok(())
}

fn handle_resized(output: &Output, size: Size<i32, Physical>) {
    output.change_current_state(
        Some(Mode {
            size,
            refresh: 60_000,
        }),
        None,
        None,
        None,
    );
}

fn handle_redraw(
    state: &mut WaylandState,
    backend: &mut WinitGraphicsBackend<GlesRenderer>,
    output: &Output,
    display: &mut DisplayHandle,
    damage_tracker: &mut OutputDamageTracker,
) {
    let size = backend.window_size();
    let damage = Rectangle::from_loc_and_size((0, 0), size);
    backend.bind().unwrap();

    let custom_elements: &[WaylandSurfaceRenderElement<GlesRenderer>] = &[];
    let alpha = 1.0;
    let age = 0;
    let clear_color = [0.1, 0.1, 0.1, 1.0];

    render_output(
        output,
        backend.renderer(),
        alpha,
        age,
        [&state.space],
        custom_elements,
        damage_tracker,
        clear_color,
    )
    .unwrap();

    backend.submit(Some(&[damage])).unwrap();

    state.space.elements().for_each(|window| {
        window.send_frame(
            output,
            state.start_time.elapsed(),
            Some(Duration::ZERO),
            |_, _| Some(output.clone()),
        )
    });

    state.space.refresh();
    state.smithay_state.popups.cleanup();
    let _ = display.flush_clients();

    // Ask for redraw to schedule new frame.
    backend.window().request_redraw();
}
