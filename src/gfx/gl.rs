use anyhow::Result;
use glow::HasContext;

pub fn compile_shader(
    gl: &glow::Context,
    shader_type: u32,
    source: &str,
) -> Result<glow::Shader> {
    unsafe {
        let shader = gl.create_shader(shader_type)
            .map_err(|e| anyhow::anyhow!("Failed to create shader: {}", e))?;
        gl.shader_source(shader, source);
        gl.compile_shader(shader);

        if !gl.get_shader_compile_status(shader) {
            let info = gl.get_shader_info_log(shader);
            gl.delete_shader(shader);
            anyhow::bail!("Shader compilation failed: {}", info);
        }

        Ok(shader)
    }
}

pub fn link_program(
    gl: &glow::Context,
    vertex_shader: glow::Shader,
    fragment_shader: glow::Shader,
) -> Result<glow::Program> {
    unsafe {
        let program = gl.create_program()
            .map_err(|e| anyhow::anyhow!("Failed to create program: {}", e))?;
        gl.attach_shader(program, vertex_shader);
        gl.attach_shader(program, fragment_shader);
        gl.link_program(program);

        if !gl.get_program_link_status(program) {
            let info = gl.get_program_info_log(program);
            gl.delete_program(program);
            anyhow::bail!("Program linking failed: {}", info);
        }

        gl.detach_shader(program, vertex_shader);
        gl.detach_shader(program, fragment_shader);

        Ok(program)
    }
}

pub fn load_shader_program(gl: &glow::Context, vert_src: &str, frag_src: &str) -> Result<glow::Program> {
    let vertex_shader = compile_shader(gl, glow::VERTEX_SHADER, vert_src)?;
    let fragment_shader = compile_shader(gl, glow::FRAGMENT_SHADER, frag_src)?;
    let program = link_program(gl, vertex_shader, fragment_shader)?;

    unsafe {
        gl.delete_shader(vertex_shader);
        gl.delete_shader(fragment_shader);
    }

    Ok(program)
}