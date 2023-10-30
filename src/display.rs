use sdl2::pixels::Color;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::rect;

pub fn display_netpbm(data: &Vec<u16>, width: usize, height: usize, max_value: u16) {
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();

    let window = video_subsystem.window("netpbm", width as u32, height as u32)
        .position_centered()
        .build()
        .expect("Could not build window to draw image");

    let mut canvas = window.into_canvas().build().expect("Could not build canvas to show image!");

    for y in 0..height {
        for x in 0..width {
            let gray = f32::from(data[y * width + x]) / f32::from(max_value);
            let gray = (f32::from(u8::MAX) * gray) as u8;
            canvas.set_draw_color(Color::RGB(gray, gray, gray));
            let point = rect::Point::new(x as i32, y as i32);
            canvas.draw_point(point).expect("Could not draw point");
        }
    }
    canvas.present();
    let mut event_pump = sdl_context.event_pump().expect("Could not get event_pump!");
    'showing: loop {
        let event = event_pump.wait_event();
        match event {
            Event::Quit { .. } |
            Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                break 'showing;
            }
            _ => {}
        }
    }
}