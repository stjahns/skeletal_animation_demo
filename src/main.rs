extern crate gfx_gl as gl;
extern crate camera_controllers;
extern crate collada;
extern crate env_logger;
extern crate gfx;
extern crate gfx_debug_draw;
extern crate gfx_device_gl;
extern crate piston;
extern crate sdl2;
extern crate sdl2_window;
extern crate shader_version;
extern crate skeletal_animation;
extern crate vecmath;

use gfx::traits::*;
use gfx_debug_draw::DebugRenderer;

use gl::Gl;

use std::path::Path;
use std::rc::Rc;
use std::cell::RefCell;

use piston::window::{
    WindowSettings,
    OpenGLWindow,
};

use piston::event::*;

use vecmath::{Matrix4, mat4_id};

use sdl2_window::Sdl2Window;

use camera_controllers::{
    OrbitZoomCamera,
    OrbitZoomCameraSettings,
    CameraPerspective,
    model_view_projection
};

use skeletal_animation::*;
use collada::document::ColladaDocument;

mod menu;

pub struct Settings {
    pub draw_skeleton: bool,
    pub draw_labels: bool,
    pub draw_mesh: bool,
    pub playback_speed: f32,

    pub param_1: f32,
    pub param_2: f32,
    pub param_3: f32,
}

fn main() {

    env_logger::init().unwrap();

    let (win_width, win_height) = (640, 480);

    let mut window = Sdl2Window::new(
        shader_version::OpenGL::_3_2,
        WindowSettings::new(
            "Animation Viewer".to_string(),
            piston::window::Size { width: 640, height: 480 }
        ).exit_on_esc(true)
    );

    let mut graphics = gfx_device_gl::create(|s| window.get_proc_address(s)).into_graphics();

    let mut frame = gfx::Frame::new(win_width as u16, win_height as u16);

    let window = Rc::new(RefCell::new(window));

    let clear = gfx::ClearData {
        color: [0.3, 0.3, 0.3, 1.0],
        depth: 1.0,
        stencil: 0
    };

    let mut debug_renderer = DebugRenderer::new(&mut graphics, [frame.width as u32, frame.height as u32], 64, None, None).ok().unwrap();

    // TODO - these are (usually) available in the COLLADA file, associated with a <mesh> element in a somewhat convoluted way
    let texture_paths = vec![
        "assets/young_lightskinned_male_diffuse.png",
        "assets/suit01lres_diffuse.png",
        "assets/male02_diffuse_black.png",
        "assets/brown_eye.png",
        "assets/eyebrow009.png",
        "assets/eyelashes01.png",
    ];

    let collada_document = ColladaDocument::from_path(&Path::new("assets/suit_guy.dae")).unwrap();

    let skeleton = {
        let skeleton_set = collada_document.get_skeletons().unwrap();
        skeleton_set[0].clone()
    };

    let skeleton = Rc::new(RefCell::new(skeleton));

    let mut asset_manager = AssetManager::new();
    asset_manager.load_animations("assets/clips.json");

    let blend_tree = BlendTreeNode::from_def(
        BlendTreeNodeDef::from_path("assets/walking_blend_tree.json").unwrap(),
        &asset_manager.animation_clips,
    );

    let mut controller = AnimationController::new(skeleton.clone(), blend_tree);

    //let mut skinned_renderer = SkinnedRenderer::from_collada(&mut graphics, collada_document, texture_paths).unwrap();

    let model = mat4_id();
    let mut projection = CameraPerspective {
        fov: 90.0f32,
        near_clip: 0.1,
        far_clip: 1000.0,
        aspect_ratio: (win_width as f32) / (win_height as f32)
    }.projection();

    let mut orbit_zoom_camera: OrbitZoomCamera<f32> = OrbitZoomCamera::new(
        [0.0, 0.0, 0.0],
        OrbitZoomCameraSettings::default()
    );

    // Start event loop

    Gl::load_with(|s| unsafe {
        std::mem::transmute(sdl2::video::gl_get_proc_address(s))
    });

    let mut elapsed_time = 0f64;

    let mut settings = Settings {

        draw_skeleton: true,
        draw_labels: true,
        draw_mesh: true,
        playback_speed: 1.0,

        param_1: 0.0,
        param_2: 0.0,
        param_3: 0.0,
    };

    let mut menu = menu::Menu::<Settings>::new();

    menu.add_item(menu::MenuItem::action_item(
        "Toggle Skeleton",
        Box::new( |ref mut settings| { settings.draw_skeleton = !settings.draw_skeleton; })
    ));

    menu.add_item(menu::MenuItem::action_item(
        "Toggle Joint Labels",
        Box::new( |ref mut settings| { settings.draw_labels = !settings.draw_labels; })
    ));

    menu.add_item(menu::MenuItem::action_item(
        "Toggle Mesh",
        Box::new( |ref mut settings| { settings.draw_mesh = !settings.draw_mesh; })
    ));

    menu.add_item(menu::MenuItem::slider_item(
        "Playback Speed = ",
        [0.0, 5.0],
        0.1,
        Box::new( |ref settings| { settings.playback_speed }),
        Box::new( |ref mut settings, value| { settings.playback_speed = value }),
    ));

    menu.add_item(menu::MenuItem::slider_item(
        "Param 1 = ",
        [0.0, 1.0],
        0.01,
        Box::new( |ref settings| { settings.param_1 }),
        Box::new( |ref mut settings, value| { settings.param_1 = value }),
    ));

    menu.add_item(menu::MenuItem::slider_item(
        "Param 2 = ",
        [0.0, 1.0],
        0.01,
        Box::new( |ref settings| { settings.param_2 }),
        Box::new( |ref mut settings, value| { settings.param_2 = value }),
    ));

    menu.add_item(menu::MenuItem::slider_item(
        "Param 3 = ",
        [0.0, 1.0],
        0.01,
        Box::new( |ref settings| { settings.param_3 }),
        Box::new( |ref mut settings, value| { settings.param_3 = value }),
    ));

    for e in window.events() {

        e.resize(|width, height| {
            debug_renderer.resize(width, height);

            // Update frame
            frame.width = width as u16;
            frame.height = height as u16;

            // Update projection matrix
            projection = CameraPerspective {
                fov: 90.0f32,
                near_clip: 0.1,
                far_clip: 1000.0,
                aspect_ratio: (width as f32) / (height as f32)
            }.projection();
        });

        orbit_zoom_camera.event(&e);

        menu.event(&e, &mut settings);

        if let Some(args) = e.render_args() {
            graphics.clear(clear, gfx::COLOR | gfx::DEPTH, &frame);

            let camera_view = orbit_zoom_camera.camera(args.ext_dt).orthogonal();

            let camera_projection = model_view_projection(
                model,
                camera_view,
                projection
            );

            // Draw axes
            debug_renderer.draw_line([0.0, 0.0, 0.0], [5.0, 0.0, 0.0], [1.0, 0.0, 0.0, 1.0]);
            debug_renderer.draw_line([0.0, 0.0, 0.0], [0.0, 5.0, 0.0], [0.0, 1.0, 0.0, 1.0]);
            debug_renderer.draw_line([0.0, 0.0, 0.0], [0.0, 0.0, 5.0], [0.0, 0.0, 1.0, 1.0]);

            debug_renderer.draw_text_at_position(
                "X",
                [6.0, 0.0, 0.0],
                [1.0, 0.0, 0.0, 1.0],
            );

            debug_renderer.draw_text_at_position(
                "Y",
                [0.0, 6.0, 0.0],
                [0.0, 1.0, 0.0, 1.0],
            );

            debug_renderer.draw_text_at_position(
                "Z",
                [0.0, 0.0, 6.0],
                [0.0, 0.0, 1.0, 1.0],
            );

            elapsed_time = elapsed_time + 0.01 * settings.playback_speed as f64;

            let mut global_poses: [Matrix4<f32>; 64] = [ mat4_id(); 64 ];

            controller.set_param(0, settings.param_1);
            controller.set_param(1, settings.param_2);
            controller.set_param(2, settings.param_3);

            controller.get_output_pose(elapsed_time as f32, &mut global_poses[0 .. skeleton.borrow().joints.len()]);

            if settings.draw_mesh {
                //skinned_renderer.render(&mut graphics, &frame, camera_view, camera_projection, &global_poses);
            }

            if settings.draw_skeleton {
                draw_skeleton(skeleton.clone(), &global_poses, &mut debug_renderer, settings.draw_labels);
            }

            menu.draw(&settings, &mut debug_renderer);

            debug_renderer.render(&mut graphics, &frame, camera_projection);


            graphics.end_frame();
        }
    }
}
