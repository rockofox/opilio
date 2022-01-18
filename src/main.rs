#[macro_use]
extern crate html5ever;
extern crate markup5ever_rcdom as rcdom;
extern crate sdl2;

use std::cell::RefCell;
use std::io::{self};
use std::ops::RangeBounds;
use std::path::Path;
use std::rc::Rc;

use html5ever::parse_document;
use html5ever::tendril::TendrilSink;

use rcdom::RcDom;
use sdl2::event::{Event, WindowEvent};
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::{TextureCreator, WindowCanvas};
use sdl2::ttf::{FontStyle, Sdl2TtfContext};
use sdl2::video::WindowContext;

use std::default::Default;
use std::iter::repeat;
use std::string::String;

use rcdom::{Handle, NodeData};

static SCREEN_WIDTH: u32 = 800;
static SCREEN_HEIGHT: u32 = 600;
static BG_COLOR: Color = Color::WHITE;
static FG_COLOR: Color = Color::BLACK;

// handle the annoying Rect i32
macro_rules! rect(
    ($x:expr, $y:expr, $w:expr, $h:expr) => (
        Rect::new($x as i32, $y as i32, $w as u32, $h as u32)
    )
);

// Scale fonts to a reasonable size when they're too big (though they might look less smooth)
fn get_centered_rect(rect_width: u32, rect_height: u32, cons_width: u32, cons_height: u32) -> Rect {
    let wr = rect_width as f32 / cons_width as f32;
    let hr = rect_height as f32 / cons_height as f32;

    let (w, h) = if wr > 1f32 || hr > 1f32 {
        if wr > hr {
            println!("Scaling down! The text will look worse!");
            let h = (rect_height as f32 / wr) as i32;
            (cons_width as i32, h)
        } else {
            println!("Scaling down! The text will look worse!");
            let w = (rect_width as f32 / hr) as i32;
            (w, cons_height as i32)
        }
    } else {
        (rect_width as i32, rect_height as i32)
    };

    let cx = (SCREEN_WIDTH as i32 - w) / 2;
    let cy = (SCREEN_HEIGHT as i32 - h) / 2;
    rect!(cx, cy, w, h)
}
fn render<'a>(
    indent: usize,
    handle: &Handle,
    tag_name: &str,
    text_index: &mut u32,
    context: &'a mut RendererContext,
) {
    let node = handle;
    // FIXME: don't allocate
    // print!("{}", repeat(" ").take(indent).collect::<String>());
    // Load a font
    let mut next_tag_name = "";
    let invisible_tags = ["style", "script", "head", "title", "meta", "link"];
    // render a surface, and convert it to a texture bound to the canvas

    match node.data {
        NodeData::Document => {
            // println!("#Document")
        }

        NodeData::Doctype {
            ref name,
            ref public_id,
            ref system_id,
        } => {
            // println!("<!DOCTYPE {} \"{}\" \"{}\">", name, public_id, system_id)
        }

        NodeData::Text { ref contents } => {
            if &contents.borrow().trim().len() != &0 && !invisible_tags.contains(&tag_name) {
                let surface = context
                    .font
                    .borrow()
                    .render(&contents.borrow())
                    .blended(FG_COLOR)
                    .map_err(|e| e.to_string())
                    .unwrap();
                let texture = context
                    .texture_creator
                    .create_texture_from_surface(&surface)
                    .map_err(|e| e.to_string())
                    .unwrap();
                let (mut width, mut height) = surface.size();

                if tag_name.starts_with("h") {
                    let font_sizes = [32, 24, 19, 16, 13, 11];
                    let font_size = font_sizes[tag_name
                        .chars()
                        .nth(1)
                        .unwrap_or('1')
                        .to_string()
                        .parse::<usize>()
                        .unwrap()
                        - 1]
                        * context.scaling_factor;

                    let ratio = font_size as f32 / height as f32;
                    context.font.borrow_mut().set_style(FontStyle::BOLD);
                    width = (width as f32 * ratio).ceil() as u32;
                    height = font_size;
                }
                context
                    .canvas
                    .borrow_mut()
                    .copy(&texture, None, rect!(0, *text_index, width, height))
                    .unwrap();
                context.font.borrow_mut().set_style(FontStyle::NORMAL);
                // println!("{} {} {}", tag_name, indent, contents.borrow());
                *text_index += height as u32;
            }
            // println!("#text: {}", contents.borrow().escape_default())
        }

        NodeData::Comment { ref contents } => {
            // println!("<!-- {} -->", contents.escape_default())
        }

        NodeData::Element {
            ref name,
            ref attrs,
            ..
        } => {
            // print!("<{}", name.local);
            next_tag_name = &name.local;
            for attr in attrs.borrow().iter() {
                // print!(" {}=\"{}\"", attr.name.local, attr.value);
            }
            // println!(">");
        }

        NodeData::ProcessingInstruction { .. } => unreachable!(),
    }
    for child in node.children.borrow().iter() {
        render(indent + 1, child, next_tag_name, text_index, context);
    }
}
struct RendererContext<'a> {
    canvas: Rc<RefCell<WindowCanvas>>,
    font: Rc<RefCell<sdl2::ttf::Font<'a, 'a>>>,
    texture_creator: Rc<TextureCreator<WindowContext>>,
    scaling_factor: u32,
}
fn main() -> Result<(), String> {
    let sdl_context = sdl2::init()?;
    let video_subsys = sdl_context.video()?;
    let ttf_context = sdl2::ttf::init().map_err(|e| e.to_string())?;

    let window = video_subsys
        .window("SDL2_TTF Example", SCREEN_WIDTH, SCREEN_HEIGHT)
        .position_centered()
        .resizable()
        .vulkan()
        .allow_highdpi()
        .build()
        .map_err(|e| e.to_string())?;

    let mut canvas = window.into_canvas().build().map_err(|e| e.to_string())?;
    let texture_creator = canvas.texture_creator();

    canvas.set_draw_color(BG_COLOR);
    canvas.clear();

    let stdin = io::stdin();
    let dom = parse_document(RcDom::default(), Default::default())
        .from_utf8()
        .read_from(&mut stdin.lock())
        .unwrap();

    let mut font_path = {
        if Path::new("/usr/share/fonts/TTF/Times.TTF").exists() {
            "/usr/share/fonts/TTF/Times.TTF"
        } else {
            "assets/trim.ttf"
        }
    };
    let sf = canvas.output_size().unwrap().0 / canvas.window().size().0;
    let mut font = ttf_context
        .load_font("/usr/share/fonts/TTF/Times.TTF", 12 * sf as u16)
        .unwrap_or_else(|_| {
            ttf_context
                .load_font("assets/trim.ttf", 12 * sf as u16)
                .expect("Could neither load system font nor fallback!")
        });
    let mut rc = RendererContext {
        canvas: Rc::new(RefCell::new(canvas)),
        font: Rc::new(RefCell::new(font)),
        texture_creator: Rc::new(texture_creator),
        scaling_factor: sf,
    };
    let mut text_index: u32 = 0;
    rc.font.borrow_mut().set_style(sdl2::ttf::FontStyle::NORMAL);

    rc.canvas.borrow_mut().present();

    'mainloop: loop {
        for event in sdl_context.event_pump()?.poll_iter() {
            match event {
                Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                }
                | Event::Quit { .. } => break 'mainloop,
                Event::Window {
                    win_event: WindowEvent::Resized(w, h),
                    ..
                } => {
                    rc.canvas
                        .borrow_mut()
                        .window_mut()
                        .set_size(w as u32, h as u32)
                        .unwrap();
                    rc.canvas.borrow_mut().set_draw_color(BG_COLOR);
                    rc.canvas.borrow_mut().clear();

                    render(0, &dom.document, "", &mut text_index, &mut rc);
                    rc.canvas.borrow_mut().set_draw_color(Color::RED);

                    // canvas.draw_rect(rect!(0, 0, w as u32, h as u32)).unwrap();
                    rc.canvas.borrow_mut().present();
                    text_index = 0;
                }
                Event::Window { .. } => {
                    // println!("Window moved to ({}, {})", x, y);
                    if rc.canvas.borrow().output_size().unwrap().0
                        / rc.canvas.borrow().window().size().0
                        != rc.scaling_factor
                    {
                        // println!("Scale factor changed!");
                        rc.scaling_factor = rc.canvas.borrow().output_size().unwrap().0
                            / rc.canvas.borrow().window().size().0;
                        rc.font = Rc::new(RefCell::new(
                            ttf_context
                                .load_font(
                                    "/usr/share/fonts/TTF/Times.TTF",
                                    12 * rc.scaling_factor as u16,
                                )
                                .unwrap_or_else(|_| {
                                    ttf_context
                                        .load_font("assets/trim.ttf", 12 * rc.scaling_factor as u16)
                                        .expect("Could neither load system font nor fallback!")
                                }),
                        ));
                    }
                }
                _ => {}
            }
        }
    }

    Ok(())
}
