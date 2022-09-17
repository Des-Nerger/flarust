#![warn(clippy::pedantic, elided_lifetimes_in_paths, explicit_outlives_requirements)]
#![allow(non_snake_case)]

macro_rules! mods {
	($( $i:ident ),+) => {
		$(
			mod $i;
		)+
	};
}
mods!(flaretriiablo, glad);

use {
	const_format::concatcp,
	core::{
		ffi::c_void,
		mem::size_of_val,
		ptr::{null, null_mut},
	},
	flaretriiablo::*,
	glad::gl::*,
	libc::{fprintf, FILE},
	rust_libretro_sys::{
		retro_hw_context_type::*,
		retro_log_level::{RETRO_LOG_ERROR as ERR, RETRO_LOG_INFO as INFO},
		retro_pixel_format::*,
		*,
	},
	std::{
		fs::File,
		io::Read,
		os::raw::{c_char, c_uint},
	},
};

extern "C" {
	static stderr: *mut FILE;
}

#[repr(C)]
struct RetroHwRenderCallback {
	context_type: retro_hw_context_type,
	context_reset: unsafe extern "C" fn(),
	get_current_framebuffer: unsafe extern "C" fn() -> usize,
	get_proc_address: unsafe extern "C" fn(sym: *const c_char) -> *const c_void,
	depth: bool,
	stencil: bool,
	bottom_left_origin: bool,
	version_major: c_uint,
	version_minor: c_uint,
	cache_context: bool,
	context_destroy: unsafe extern "C" fn(),
	debug_context: bool,
}

unsafe extern "C" fn env_cb(_: c_uint, _: *mut c_void) -> bool {
	unreachable!()
}
unsafe extern "C" fn video_cb(_: *const c_void, _: c_uint, _: c_uint, _: size_t) {
	unreachable!()
}
unsafe extern "C" fn input_poll_cb() {
	unreachable!()
}
unsafe extern "C" fn input_state_cb(_: c_uint, _: c_uint, _: c_uint, _: c_uint) -> i16 {
	unreachable!()
}
unsafe extern "C" fn audio_cb(_: i16, _: i16) {
	unreachable!()
}
unsafe extern "C" fn audio_batch_cb(_: *const i16, _: size_t) -> size_t {
	unreachable!()
}
unsafe extern "C" fn get_current_framebuffer() -> usize {
	unreachable!()
}
unsafe extern "C" fn get_proc_address(_: *const c_char) -> *const c_void {
	unreachable!()
}

static mut LOG_CB: retro_log_printf_t = None;
static mut ENV_CB: unsafe extern "C" fn(c_uint, *mut c_void) -> bool = env_cb;
static mut VIDEO_CB: unsafe extern "C" fn(*const c_void, c_uint, c_uint, size_t) = video_cb;
static mut INPUT_POLL_CB: unsafe extern "C" fn() = input_poll_cb;
static mut INPUT_STATE_CB: unsafe extern "C" fn(c_uint, c_uint, c_uint, c_uint) -> i16 = input_state_cb;
static mut AUDIO_CB: unsafe extern "C" fn(i16, i16) = audio_cb;
static mut AUDIO_BATCH_CB: unsafe extern "C" fn(*const i16, size_t) -> size_t = audio_batch_cb;
static mut HW_RENDER: RetroHwRenderCallback = RetroHwRenderCallback {
	context_type: RETRO_HW_CONTEXT_OPENGL,
	version_major: 2,
	version_minor: 1,
	depth: false,
	stencil: false,
	bottom_left_origin: true,
	cache_context: false,
	debug_context: false,
	get_current_framebuffer,
	get_proc_address,
	context_reset,
	context_destroy,
};
const INVALID_OBJ: GLuint = GLuint::MAX;
static mut SHAD_PROG: GLuint = INVALID_OBJ;
static mut ATTR_COORD2D: GLuint = INVALID_OBJ;
static mut VBO_TRIANGLE: GLuint = INVALID_OBJ;

macro_rules! ptr {
	($e: expr) => {
		$e as *const _ as _
	};
}

macro_rules! cstr {
	($e: expr) => {
		ptr!(concatcp!($e, "\0"))
	};
}

/*
macro_rules! log {
	( $level:expr, $fmt:expr $(, $arg:expr)* $(,)? ) => {
		if let Some(logCb) = LOG_CB {
			let fmt: &str = &format!("{}\0", format_args!($fmt, $( $arg ),*));
			logCb($level, ptr!(fmt), $( $arg ),*);
		} else {
			eprint!($fmt, $( $arg ),*);
		}
	};
}
*/

macro_rules! logf {
	( $level:expr, $fmt:expr $(, $arg:expr)* $(,)? ) => {
		{
			let fmtPtr: *const c_char = cstr!($fmt);
			if let Some(logCb) = LOG_CB {
				logCb($level, fmtPtr, $( $arg ),*);
			} else {
				fprintf(stderr, fmtPtr, $( $arg ),*);
			}
		}
	};
}

const CARGO_PKG_NAME: &str = env!("CARGO_PKG_NAME");
const CARGO_PKG_VERSION: &str = env!("CARGO_PKG_VERSION");
const VIDEO_WIDTH: u32 = 664;
const VIDEO_HEIGHT: u32 = 360;

unsafe extern "C" fn context_reset() {
	gl_load(|s| (HW_RENDER.get_proc_address)(ptr!(s)));
	static TRIANGLE_VERTICES: &[f32] = &[0.0, 1.0, -1.0, -1.0, 1.0, -1.0];
	glGenBuffers(1, &mut VBO_TRIANGLE);
	glBindBuffer(GL_ARRAY_BUFFER, VBO_TRIANGLE);
	glBufferData(
		GL_ARRAY_BUFFER,
		size_of_val(TRIANGLE_VERTICES) as _,
		ptr!(TRIANGLE_VERTICES),
		GL_STATIC_DRAW,
	);
	glBindBuffer(GL_ARRAY_BUFFER, 0);
	SHAD_PROG = glCreateProgram();

	unsafe fn gl_err(obj: GLuint) -> String {
		let (getiv, getInfoLog): (unsafe fn(_, _, _), unsafe fn(_, _, _, _)) = if glIsShader(obj) == GL_TRUE
		{
			(glGetShaderiv, glGetShaderInfoLog)
		} else if glIsProgram(obj) == GL_TRUE {
			(glGetProgramiv, glGetProgramInfoLog)
		} else {
			return String::from("not a shader or a program");
		};
		let mut logLen: GLint = 0;
		getiv(obj, GL_INFO_LOG_LENGTH, &mut logLen);
		let mut log = Vec::with_capacity(logLen as _);
		getInfoLog(obj, logLen, null_mut(), log.as_mut_ptr() as _);
		log.set_len((logLen - 1) as _);
		String::from_utf8_unchecked(log)
	}
	const VER_LINE: &[u8] = b"#version 120\n";
	const PREFIX: &str = "Default/shaders/_.glsl";
	{
		let src = &mut Vec::from(VER_LINE);
		for path in [concatcp!(PREFIX, "f"), concatcp!(PREFIX, "v")] {
			src.truncate(VER_LINE.len());
			{
				let mut file = File::open(path).unwrap_or_else(|err| panic!("{path:?}: {err}"));
				file.read_to_end(src).unwrap();
			}
			match src.len() - 1 {
				usize::MAX => {}
				lastIdx => {
					if src[lastIdx] == b'\n' {
						src.truncate(lastIdx);
					}
				}
			}
			src.push(b'\0');
			let (pathLastByte, srcPtr) = (path.as_bytes()[path.len() - 1], src.as_ptr());
			logf!(INFO, "%c = ```\n%s\n```\n", pathLastByte as c_uint, srcPtr);
			let sh = glCreateShader(if pathLastByte == b'v' { GL_VERTEX_SHADER } else { GL_FRAGMENT_SHADER });
			glShaderSource(sh, 1, ptr!(&srcPtr), null());
			glCompileShader(sh);
			let compileOk = false;
			glGetShaderiv(sh, GL_COMPILE_STATUS, ptr!(&compileOk));
			if !compileOk {
				panic!("error in {} shader: {}", char::from(pathLastByte), gl_err(sh));
			}
			glAttachShader(SHAD_PROG, sh);
		}
	}
	glLinkProgram(SHAD_PROG);
	let linkOk = false;
	glGetProgramiv(SHAD_PROG, GL_LINK_STATUS, ptr!(&linkOk));
	if !linkOk {
		panic!("error in glLinkProgram: {}", gl_err(SHAD_PROG));
	}
	const ATTR_NAME: &str = "coord2d";
	ATTR_COORD2D = glGetAttribLocation(SHAD_PROG, cstr!(ATTR_NAME))
		.try_into()
		.unwrap_or_else(|err| panic!("could not bind attribute {ATTR_NAME:?}: {err}"));
}

unsafe extern "C" fn context_destroy() {
	glDeleteProgram(SHAD_PROG);
	glDeleteBuffers(1, &VBO_TRIANGLE);
}

#[no_mangle]
pub unsafe extern "C" fn retro_init() {}

#[no_mangle]
pub unsafe extern "C" fn retro_deinit() {}

#[no_mangle]
pub unsafe extern "C" fn retro_api_version() -> c_uint {
	RETRO_API_VERSION
}

#[no_mangle]
pub unsafe extern "C" fn retro_get_system_info(info: *mut retro_system_info) {
	*info = retro_system_info {
		library_name: cstr!(CARGO_PKG_NAME),
		library_version: cstr!(CARGO_PKG_VERSION),
		valid_extensions: null(),
		need_fullpath: true,
		block_extract: true,
	};
}

#[no_mangle]
pub unsafe extern "C" fn retro_get_system_av_info(info: *mut retro_system_av_info) {
	*info = retro_system_av_info {
		timing: retro_system_timing { fps: 60.0, sample_rate: 0.0 },
		geometry: retro_game_geometry {
			base_width: VIDEO_WIDTH,
			base_height: VIDEO_HEIGHT,
			max_width: VIDEO_WIDTH,
			max_height: VIDEO_HEIGHT,
			aspect_ratio: 0.0,
		},
	};
}

#[no_mangle]
pub unsafe extern "C" fn retro_set_environment(cb: retro_environment_t) {
	ENV_CB = cb.unwrap();
	ENV_CB(RETRO_ENVIRONMENT_SET_SUPPORT_NO_GAME, ptr!(&true));
	let logging = retro_log_callback { log: None };
	LOG_CB = if ENV_CB(RETRO_ENVIRONMENT_GET_LOG_INTERFACE, ptr!(&logging)) { logging.log } else { None };
}

#[no_mangle]
pub unsafe extern "C" fn retro_set_video_refresh(cb: retro_video_refresh_t) {
	VIDEO_CB = cb.unwrap();
}

#[no_mangle]
pub unsafe extern "C" fn retro_set_audio_sample(cb: retro_audio_sample_t) {
	AUDIO_CB = cb.unwrap();
}

#[no_mangle]
pub unsafe extern "C" fn retro_set_audio_sample_batch(cb: retro_audio_sample_batch_t) {
	AUDIO_BATCH_CB = cb.unwrap();
}

#[no_mangle]
pub unsafe extern "C" fn retro_set_input_poll(cb: retro_input_poll_t) {
	INPUT_POLL_CB = cb.unwrap();
}

#[no_mangle]
pub unsafe extern "C" fn retro_set_input_state(cb: retro_input_state_t) {
	INPUT_STATE_CB = cb.unwrap();
}

#[no_mangle]
pub unsafe extern "C" fn retro_set_controller_port_device(_port: c_uint, _device: c_uint) {}

#[no_mangle]
pub unsafe extern "C" fn retro_reset() {}

#[no_mangle]
pub unsafe extern "C" fn retro_run() {
	INPUT_POLL_CB();
	glBindFramebuffer(GL_FRAMEBUFFER, (HW_RENDER.get_current_framebuffer)() as _);
	static mut FRAME_COUNT: u8 = 0;
	let f: f32 = if FRAME_COUNT <= 127 { 0.33 } else { 0.67 };
	FRAME_COUNT += 2;
	glClearColor(f, f, f, f);
	glViewport(0, 0, VIDEO_WIDTH as _, VIDEO_HEIGHT as _);
	glClear(GL_COLOR_BUFFER_BIT);
	glUseProgram(SHAD_PROG);
	glEnableVertexAttribArray(ATTR_COORD2D);
	glBindBuffer(GL_ARRAY_BUFFER, VBO_TRIANGLE);
	glVertexAttribPointer(ATTR_COORD2D, 2, GL_FLOAT, GL_FALSE, 0, null());
	glDrawArrays(GL_TRIANGLES, 0, 3);
	glBindBuffer(GL_ARRAY_BUFFER, 0);
	glDisableVertexAttribArray(ATTR_COORD2D);
	glUseProgram(0);
	const IRRELEVANT: size_t = size_t::MAX;
	VIDEO_CB(RETRO_HW_FRAME_BUFFER_VALID, VIDEO_WIDTH as _, VIDEO_HEIGHT as _, IRRELEVANT);
}

#[no_mangle]
pub unsafe extern "C" fn retro_serialize_size() -> size_t {
	0
}

#[no_mangle]
pub unsafe extern "C" fn retro_serialize(_data: *mut c_void, _size: size_t) -> bool {
	false
}

#[no_mangle]
pub unsafe extern "C" fn retro_unserialize(_data: *const c_void, _size: size_t) -> bool {
	false
}

#[no_mangle]
pub unsafe extern "C" fn retro_cheat_reset() {}

#[no_mangle]
pub unsafe extern "C" fn retro_cheat_set(_index: c_uint, _enabled: bool, _code: *const c_char) {}

#[no_mangle]
pub unsafe extern "C" fn retro_load_game(info: *const retro_game_info) -> bool {
	dt1::load("D2sw/mpqs_data/global/tiles/ACT1/Town/Floor.dt1");
	match info.as_ref() {
		Some(&retro_game_info { path, .. }) if path != null() => {
			logf!(
				ERR,
				concatcp!(
					"\n  This core doesn't support specifying content files / paths explicitly.",
					"\n  Please remove the \"%s\" argument.\n",
				),
				path,
			);
			return false;
		}
		_ => {}
	}
	if !ENV_CB(RETRO_ENVIRONMENT_SET_PIXEL_FORMAT, ptr!(&RETRO_PIXEL_FORMAT_XRGB8888)) {
		logf!(ERR, "XRGB8888 is not supported.\n");
		return false;
	}
	if !ENV_CB(RETRO_ENVIRONMENT_SET_HW_RENDER, ptr!(&HW_RENDER)) {
		logf!(ERR, "HW Context could not be initialized.\n");
		return false;
	}
	true
}

#[no_mangle]
pub unsafe extern "C" fn retro_load_game_special(
	_type: c_uint,
	_info: *const retro_game_info,
	_num: size_t,
) -> bool {
	false
}

#[no_mangle]
pub unsafe extern "C" fn retro_unload_game() {}

#[no_mangle]
pub unsafe extern "C" fn retro_get_region() -> c_uint {
	RETRO_REGION_NTSC
}

#[no_mangle]
pub unsafe extern "C" fn retro_get_memory_data(_id: c_uint) -> *mut c_void {
	null_mut()
}

#[no_mangle]
pub unsafe extern "C" fn retro_get_memory_size(_id: c_uint) -> size_t {
	0
}
