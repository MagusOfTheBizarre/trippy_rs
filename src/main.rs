/*
 * ----------------------------------------------------------------------------
 * "THE BEER-WARE LICENSE" (Revision 42):
 * <meernik@live.com> wrote this file.  As long as you retain this notice you
 * can do whatever you want with this stuff. If we meet some day, and you think
 * this stuff is worth it, you can buy me a beer in return. Landon Meernik
 * ----------------------------------------------------------------------------
 */

extern crate sdl2;
extern crate nalgebra as na;
extern crate libc;
extern crate time;

use sdl2::keyboard::Keycode;
use sdl2::event::Event;
use gl::types::GLuint;
use gl::types::GLint;
use std::ffi::CString;
use std::f32::consts::PI;
use std::cmp::{min, max};


mod gl {
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}

pub struct Shader<'a> {
    pub id: GLuint,
    pub glctx: &'a gl::Gl,
}

pub struct Program<'a> {
    pub id: GLuint,
    pub glctx: &'a gl::Gl,
}

impl<'a> Drop for Program<'a> {
    fn drop(&mut self) {
        unsafe {
            self.glctx.DeleteProgram(self.id);
        }
    }
}
impl<'a> Program<'a> {
    fn new(glctx: &'a gl::Gl, shaders: &[Shader]) -> Result<Program<'a>, String> {
        let p = Program {
            id: unsafe { glctx.CreateProgram() },
            glctx: glctx,
        };
        let successful: bool = unsafe {
            for s in shaders {
                glctx.AttachShader(p.id, s.id);
            }
            let mut result: GLint = 0;
            glctx.LinkProgram(p.id);
            glctx.GetProgramiv(p.id, gl::LINK_STATUS, &mut result);
            result == gl::TRUE as GLint
        };
        match successful {
            true => Ok(p),
            false => Err({
                let mut log_len = 0;
                unsafe {
                    glctx.GetProgramiv(p.id, gl::INFO_LOG_LENGTH, &mut log_len);
                }
                if log_len == 0 {
                    String::from("No program link log :|")
                } else {
                    let mut buf = Vec::with_capacity(log_len as usize);
                    let buf_ptr = buf.as_mut_ptr() as *mut gl::types::GLchar;
                    unsafe {
                        glctx.GetProgramInfoLog(p.id, log_len, std::ptr::null_mut(), buf_ptr);
                        buf.set_len(log_len as usize);
                    }
                    match String::from_utf8(buf) {
                        Ok(log) => format!("LINKFAIL: {}", log),
                        Err(vec) => format!("Could not decode shader log {}", vec)
                    }
                }
            })
        }
    }
}

impl<'a> Drop for Shader<'a> {
    fn drop(&mut self) {
        unsafe {
            self.glctx.DeleteShader(self.id);
        };
    }
}
impl<'a> Shader<'a> {
    fn new (glctx: &'a gl::Gl, typ: GLuint, source: &str) -> Result<Shader<'a>, String> {
        let s = Shader {
            id: unsafe { glctx.CreateShader(typ) },
            glctx: glctx,
        };
        let successful: bool = unsafe {
            let ptr: *const u8 = source.as_bytes().as_ptr();
            let ptr_i8: *const i8 = std::mem::transmute(ptr);
            let len = source.len() as GLint;
            glctx.ShaderSource(s.id, 1, &ptr_i8, &len);    
            glctx.CompileShader(s.id);
            let mut result: GLint = 0;
            glctx.GetShaderiv(s.id, gl::COMPILE_STATUS, &mut result);
            result == gl::TRUE as GLint
        };
        match successful {
            true => Ok(s),
            false => Err({
                let mut log_len = 0;
                unsafe { glctx.GetShaderiv(s.id, gl::INFO_LOG_LENGTH, &mut log_len) };
                if log_len <= 0 {
                    String::from("No shader info log?")
                } else {
                    let mut buf = Vec::with_capacity(log_len as usize);
                    let buf_ptr = buf.as_mut_ptr() as *mut gl::types::GLchar;
                    unsafe {
                        glctx.GetShaderInfoLog(s.id, log_len, std::ptr::null_mut(), buf_ptr);
                        buf.set_len(log_len as usize);
                    };

                    match String::from_utf8(buf) {
                        Ok(log) => format!("COMPILEFAIL: {}", log),
                        Err(vec) => format!("Could not convert compilation log from buffer: {}",vec),
                    }
                }
            })
        }
    }
}

fn main() {
    println!("OK let's do this!");
    let sctx = sdl2::init().unwrap();
    let vctx = sctx.video().unwrap();
    let wctx = vctx.window("Sufficiently Trippy", 1024, 1024)
        .opengl()
        .build()
        .unwrap();
    let sdl_glctx = wctx.gl_create_context().unwrap();
    let mut running = true;
    let mut event_pump = sctx.event_pump().unwrap();
    let glctx = &gl::Gl::load_with(|s| vctx.gl_get_proc_address(s));
    let mut vertices = [
        -1.5, -1.5,  0.0,
         1.5, -1.5,  0.0,
         1.5,  1.5,  0.0, 
        -1.5,  1.5,  0.0 as f32, ]; 

    let indices = [
        0 as u32, 1, 2, 
        0, 2, 3,];
    let coords = [
         -1.0,  -1.0,
         1.0,   -1.0,
         1.0,   1.0,
         -1.0,  1.0 as f32];
    let mut mvm: na::Mat4<f32> = na::new_identity(4);
    let translate = na::Mat4::<f32>::new(
        0.5, 0., 0., 0.0,
        0., 0.5, 0., 0.0,
        0., 0., 0.5, 5.,
        0., 0., 0., 1. );
    mvm = mvm * translate;
    let rot = na::to_homogeneous(& na::Rot3::<f32>::new_with_euler_angles(0., 0., 1.));
    mvm = mvm * rot;
    let pm = na::Persp3::<f32>::new(1., PI / 4., 1.0, 100. ).to_mat();
    
    let pr = unsafe { // Initialize opengl
        glctx.ClearColor(0.0, 0.0, 0.0, 1.0);
        glctx.ClearDepth(1.0);
        glctx.Enable(gl::DEPTH_TEST);
        glctx.DepthFunc(gl::LEQUAL);
        //glctx.ShadeModel(gl::SMOOTH);
        //glctx.Hint(gl::PERSPECTIVE_CORRECTION_HINT, gl::NICEST);

        let vs = Shader::new(glctx, gl::VERTEX_SHADER,
        r#"
            #define TAU (3.1415926535897932384626433832795 * 2)
            #define N 100.0

            attribute vec2 aCoord;
            attribute vec3 aPosition;
            uniform uBlock {
                mat4 uMVMatrix;
                mat4 uPMatrix;
                float t;
            };

            varying vec2 vCoord;
            varying float vTheta;

            mat3 rotationMatrix(vec3 axis, float angle)
            {
                axis = normalize(axis);
                float s = sin(angle);
                float c = cos(angle);
                float oc = 1.0 - c;
                
                return mat3(oc * axis.x * axis.x + c,           oc * axis.x * axis.y - axis.z * s,  oc * axis.z * axis.x + axis.y * s, 
                            oc * axis.x * axis.y + axis.z * s,  oc * axis.y * axis.y + c,           oc * axis.y * axis.z - axis.x * s, 
                            oc * axis.z * axis.x - axis.y * s,  oc * axis.y * axis.z + axis.x * s,  oc * axis.z * axis.z + c);
            }
            void main(void) {
                float theta = gl_InstanceID / N * TAU + t / 2;
                float fb1 = sin(t / 40);
                float fb1i = 1. - fb1;
                float fb2 = sin(t / 47 + sin(fb1) * 41) / 2 + 0.5;
                float fb2i = 1. - fb2;
                vec3 instancePosition = vec3(sin(theta * 30), 
                                             cos(theta * (29 + fb1)), 
                                             ((float)gl_InstanceID / N * 2 - 1) * fb2 + fb2i * sin(theta * (31 + fb1)));
                mat3 instanceRotation = rotationMatrix(vec3(sin(theta / 3 + t / 5), 
                                                            cos(theta / 5 + t / 7), 
                                                            sin(theta / 7 + t / 3)), 
                                                            t * 1.5);
                gl_Position = uPMatrix * uMVMatrix * vec4((instanceRotation * aPosition) / 10 + instancePosition, 1);
                vCoord = aCoord;
                vTheta = theta;
            }
        "#).unwrap();
        let fs = Shader::new(glctx, gl::FRAGMENT_SHADER,
        r#"
            #define TAU (3.1415926535897932384626433832795 * 2)
            #define N 100.0
            varying vec2 vCoord;
            varying float vTheta;
            float cmeander(float phase) {
                return (sin(2*phase)*cos(phase)-.3) * 2 ;
            }
            void main(void) {
                float dist = length(vCoord);

                if (abs(vCoord[0]) > 0.8 || abs(vCoord[1]) > 0.8) {
                    gl_FragColor = vec4(cmeander(vTheta), cmeander(vTheta * TAU / 3), cmeander((2 * vTheta) * (TAU / 3)) , 1);
                } else {
                    gl_FragColor = vec4(cmeander(vTheta / 19), cmeander(vTheta / 19 * TAU / 3), cmeander((2 * vTheta / 19) * (TAU / 3)) , 1);
                }
            }
        "#).unwrap();
        Program::new(glctx, &[fs, vs]).unwrap()
    };
    // Get attr and uniform locations
    let vloc = unsafe { glctx.GetProgramResourceIndex(pr.id, gl::PROGRAM_INPUT, CString::new("aPosition").unwrap().as_ptr())};
    let cloc = unsafe { glctx.GetProgramResourceIndex(pr.id, gl::PROGRAM_INPUT, CString::new("aCoord").unwrap().as_ptr())};
    let ubloc = unsafe { glctx.GetProgramResourceIndex(pr.id, gl::UNIFORM_BLOCK, CString::new("uBlock").unwrap().as_ptr())} as u32;
    println!("Coords loc: {}, Vertex loc: {}, ub loc: {}", cloc, vloc, ubloc);
    if cloc as i32 == -1 {
        panic!("couldn't find coords");
    }
    if vloc as i32 == -1 {
        panic!("couldn't find position");
    }
    if ubloc as i32 == -1 {
        panic!("couldn't find uniform block");
    }
    let bufs = unsafe {
        let mut bs: [u32;4] = [0, 0, 0, 0];
        glctx.CreateBuffers(4, bs.as_mut_ptr());
        bs
    };
    let vbuf = bufs[0];
    let cbuf = bufs[1];
    let ibuf = bufs[2];
    let ubbuf = bufs[3];
    if vbuf == 0 || cbuf == 0 || ibuf == 0 || ubbuf == 0 {
        panic!("Could not CreateBuffers, got v:{} c:{} i:{} ub:{}", vbuf, cbuf, ibuf, ubbuf);
    }
    println!("CreateBuffers'd, got v:{} c:{} i:{}", vbuf, cbuf, ibuf);
    let mflags = gl::MAP_WRITE_BIT | gl::MAP_PERSISTENT_BIT | gl::MAP_COHERENT_BIT;
    let sflags = mflags | gl::DYNAMIC_STORAGE_BIT;
    let mvmptr = unsafe {
        let mut data_size = 0 as GLint;
        glctx.GetProgramResourceiv(pr.id, gl::UNIFORM_BLOCK, ubloc, 1, &gl::BUFFER_DATA_SIZE, 1, std::ptr::null_mut(), &mut data_size as *mut GLint);
        println!("UB size: {}", data_size);
        glctx.NamedBufferStorage(ubbuf, data_size as i64, std::ptr::null(), sflags);
        let ptr = glctx.MapNamedBufferRange(ubbuf, 0, data_size as i64, mflags) as *mut f32;
        if ptr as u64 == 0 {
            panic!("Failed to map Uniform buffer {}", glctx.GetError());
        }

        println!("Ptr: {}, offset: {}", ptr as u64, ptr.offset(16) as u64);
        ptr as *mut na::Mat4<f32>
    };
    let pmptr = unsafe { mvmptr.offset(1)};
    let tptr = unsafe { mvmptr.offset(2) as *mut f32};
    println!("mvmptr: {}, pmptr: {}", mvmptr as u64, pmptr as u64);
    unsafe {
        std::ptr::write(mvmptr, mvm);
        std::ptr::write(pmptr, pm);
        std::ptr::write(tptr, 1.0 as f32);
    }
    let vptr = unsafe {
        // Map and fill the vertex buffer
        glctx.NamedBufferStorage(vbuf, vertices.len() as i64 * 4, std::ptr::null(), sflags);
        let ptr = glctx.MapNamedBufferRange(vbuf, 0, vertices.len() as i64 * 4, mflags) as *mut f32;
        if ptr as u64 == 0 {
            panic!("Failed to map vertex buffer {}", glctx.GetError());
        }
        //std::ptr::write(ptr as *mut [f32; 9], vertices);
        ptr as *mut f32
    };
    unsafe {std::ptr::copy_nonoverlapping(&vertices as *const f32, vptr, vertices.len())};
    println!("Mapped vertex buffer to {}", vptr as u64);
    let cptr = unsafe {
        // Map and fill the color buffer
        glctx.NamedBufferStorage(cbuf, coords.len() as i64 * 4, std::ptr::null(), sflags);
        let ptr = glctx.MapNamedBufferRange(cbuf, 0, coords.len() as i64 * 4, mflags);
        if ptr as u64 == 0 {
            panic!("Failed to map coord buffer {}", glctx.GetError());
        }
        ptr as *mut f32
    };
    unsafe {std::ptr::copy_nonoverlapping(&coords as *const f32, cptr, coords.len())};
    let iptr = unsafe {
        // Map and fill the index buffer
        glctx.NamedBufferStorage(ibuf, indices.len() as i64 * 4, std::ptr::null(), sflags);
        let ptr = glctx.MapNamedBufferRange(ibuf, 0, indices.len() as i64 * 4, mflags);
        if ptr as u64 == 0 {
            panic!("Failed to map index buffer {}", glctx.GetError());
        }
        ptr as *mut u32
    };
    unsafe {std::ptr::copy_nonoverlapping(&indices as *const u32, iptr, indices.len())};
    println!("Mapped color buffer to {}", cptr as u64);

    unsafe {
        glctx.UniformBlockBinding(pr.id, ubloc as u32, 1);
    };

    let vao = unsafe {
        // Create a Vertex Array Object and tie shit together;
        let mut v: u32 = 0;
        glctx.CreateVertexArrays(1, (&mut v));
        glctx.VertexArrayAttribFormat(v, vloc as u32, 3, gl::FLOAT, 0, 0);
        glctx.VertexArrayAttribFormat(v, cloc as u32, 2, gl::FLOAT, 0, 0); // ASO THIS FUCKING THING :<
        glctx.EnableVertexArrayAttrib(v, vloc as u32);
        glctx.EnableVertexArrayAttrib(v, cloc as u32);
        glctx.VertexArrayVertexBuffer(v, vloc as u32, vbuf, 0, 12); // THIS FUCKING THING :<
        glctx.VertexArrayVertexBuffer(v, cloc as u32, cbuf, 0, 8);
        glctx.VertexArrayElementBuffer(v, ibuf);
        v
    };
    if vao == 0 {
        panic!("Failed to CreateVertexArrays");
    }
    println!("Created VAO: {}", vao);

    let epoch = time::precise_time_s() as f32;
    let mut t = 0 as f32;
    let mut last_report = 0 as f32;
    let mut last_report_frame = 0;
    let mut frame = 0;
    unsafe {glctx.BindBufferBase(gl::UNIFORM_BUFFER, 1, ubbuf)};
    while running {
        t = time::precise_time_s() as f32 - epoch;

        if t - last_report > 1.0 {
            println!("{} FPS", (frame - last_report_frame) as f32 / (t - last_report));
            last_report = t;
            last_report_frame = frame;
        }
        frame += 1;
            
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit {..} | Event::KeyDown { keycode: Some(Keycode::Q), .. } => 
                    {running = false},
                Event::KeyDown { keycode: Some(kc), timestamp, .. } => {println!("Got a {} at {}", kc, timestamp)},
                _ => {println!("Unknown event")},
            }
        }
        unsafe {
            glctx.Clear(gl::COLOR_BUFFER_BIT|gl::DEPTH_BUFFER_BIT|gl::STENCIL_BUFFER_BIT);
            glctx.UseProgram(pr.id);
            glctx.BindVertexArray(vao);
            std::ptr::write(tptr, t / 4.);
            mvm = na::new_identity(4);
            let translate = na::Mat4::<f32>::new(
                1., 0., 0., 0.0,
                0., 1., 0., 0.0,
                0., 0., 1., (t/13.).sin() * 2. + 7.,
                0., 0., 0., 1. );
            mvm = mvm * translate;
            let rot = na::to_homogeneous(& na::Rot3::<f32>::new_with_euler_angles(t/11., t/9., t/7.));
            mvm = mvm * rot;
            std::ptr::write(mvmptr, mvm);
            glctx.MemoryBarrier(gl::CLIENT_MAPPED_BUFFER_BARRIER_BIT);
            glctx.DrawElementsInstanced(gl::TRIANGLES, indices.len() as i32, gl::UNSIGNED_INT, std::ptr::null(), 100);
            glctx.Finish();
        }
        wctx.gl_swap_window();
    }
}
