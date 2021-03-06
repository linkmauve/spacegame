use crate::render::path::debug::DebugQueue;
use crate::resources::Resources;
use luminance::context::GraphicsContext;
use luminance::pipeline::PipelineError;
use luminance::render_state::RenderState;
use luminance::shader::{Program, Uniform};
use luminance::shading_gate::ShadingGate;
use luminance::tess::{Mode, Tess};
use luminance_derive::{Semantics, UniformInterface, Vertex};
use luminance_gl::GL33;

pub mod debug;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Semantics)]
pub enum VertexSemantics {
    #[sem(name = "position", repr = "[f32; 2]", wrapper = "Position")]
    Position,

    #[sem(name = "color", repr = "[f32; 4]", wrapper = "Color")]
    Color,
}

#[allow(dead_code)]
#[repr(C)]
#[derive(Vertex, Copy, Debug, Clone)]
#[vertex(sem = "VertexSemantics")]
pub struct Vertex {
    position: Position,
    color: Color,
}

const VS: &'static str = include_str!("path-vs.glsl");
const FS: &'static str = include_str!("path-fs.glsl");

#[derive(UniformInterface)]
pub struct ShaderUniform {
    /// PROJECTION matrix in MVP
    projection: Uniform<[[f32; 4]; 4]>,
    /// VIEW matrix in MVP
    view: Uniform<[[f32; 4]; 4]>,
}

pub fn new_shader<B>(surface: &mut B) -> Program<GL33, VertexSemantics, (), ShaderUniform>
where
    B: GraphicsContext<Backend = GL33>,
{
    surface
        .new_shader_program::<VertexSemantics, (), ShaderUniform>()
        .from_strings(VS, None, None, FS)
        .expect("Program creation")
        .ignore_warnings()
}

pub struct PathRenderer<S>
where
    S: GraphicsContext<Backend = GL33>,
{
    tesses: Vec<Tess<S::Backend, Vertex, u16>>,
    shader: Program<S::Backend, VertexSemantics, (), ShaderUniform>,
}

impl<S> PathRenderer<S>
where
    S: GraphicsContext<Backend = GL33>,
{
    pub fn new(surface: &mut S) -> Self {
        let shader = new_shader(surface);
        Self {
            shader,
            tesses: vec![],
        }
    }

    pub fn prepare(&mut self, surface: &mut S, resources: &Resources) {
        self.tesses.clear();

        if let Some(mut debug_queue) = resources.fetch_mut::<DebugQueue>() {
            for debug_primitive in debug_queue.drain() {
                let tess = surface
                    .new_tess()
                    .set_mode(Mode::Triangle)
                    .set_vertices(debug_primitive.0)
                    .set_indices(debug_primitive.1)
                    .build()
                    .unwrap();
                self.tesses.push(tess);
            }
        }
    }

    pub fn render(
        &mut self,
        projection: &glam::Mat4,
        view: &glam::Mat4,
        shd_gate: &mut ShadingGate<S::Backend>,
    ) -> Result<(), PipelineError> {
        let tesses = &self.tesses;
        let render_state = &RenderState::default();

        shd_gate.shade(&mut self.shader, |mut iface, uni, mut rdr_gate| {
            iface.set(&uni.view, view.to_cols_array_2d());
            iface.set(&uni.projection, projection.to_cols_array_2d());
            for tess in tesses {
                rdr_gate.render(render_state, |mut tess_gate| tess_gate.render(tess))?;
            }
            Ok(())
        })
    }
}
