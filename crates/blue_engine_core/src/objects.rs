/*
 * Blue Engine by Elham Aryanpur
 *
 * The license is same as the one on the root.
*/

use crate::utils::default_resources::{DEFAULT_SHADER, DEFAULT_TEXTURE};
use crate::{
    Matrix4, Pipeline, PipelineData, Quaternion, Renderer, ShaderSettings, StringBuffer,
    TextureData, TextureMode, Textures, UnsignedIntType, Vector3, Vector4, Vertex,
};

/// Objects make it easier to work with Blue Engine, it automates most of work needed for
/// creating 3D objects and showing them on screen. A range of default objects are available
/// as well as ability to customize each of them and even create your own! You can also
/// customize almost everything there is about them!
pub struct Object {
    /// Give your object a name, which can help later on for debugging.
    pub name: std::sync::Arc<str>,
    /// A list of Vertex
    pub vertices: Vec<Vertex>,
    /// A list of indices that dictates the order that vertices appear
    pub indices: Vec<UnsignedIntType>,
    /// Describes how to uniform buffer is structures
    pub uniform_layout: wgpu::BindGroupLayout,
    /// Pipeline holds all the data that is sent to GPU, including shaders and textures
    pub pipeline: Pipeline,
    /// List of instances of this object
    pub instances: Vec<Instance>,
    /// instance buffer
    pub instance_buffer: wgpu::Buffer,
    /// Dictates the size of your object in relation to the world
    pub size: Vector3,
    /// Dictates the position of your object in pixels
    pub position: Vector3,
    /// Dictates the rotation of your object
    pub rotation: Vector3,
    // flags the object to be updated until next frame
    pub(crate) changed: bool,
    /// Transformation matrices helps to apply changes to your object, including position, orientation, ...
    /// Best choice is to let the Object system handle it
    pub translation_matrix: Matrix4,
    /// Transformation matrices helps to apply changes to your object, including position, orientation, ...
    /// Best choice is to let the Object system handle it
    pub scale_matrix: Matrix4,
    /// Transformation matrices helps to apply changes to your object, including position, orientation, ...
    /// Best choice is to let the Object system handle it
    pub rotation_quaternion: Quaternion,
    /// Transformation matrix, but inversed
    pub inverse_transformation_matrix: Matrix4,
    /// The main color of your object
    pub color: Vector4,
    /// A struct making it easier to manipulate specific parts of shader
    pub shader_builder: crate::objects::ShaderBuilder,
    /// Shader settings
    pub shader_settings: ShaderSettings,
    /// Camera have any effect on the object?
    pub camera_effect: Option<std::sync::Arc<str>>,
    /// Uniform Buffers to be sent to GPU. These are raw and not compiled for GPU yet
    pub uniform_buffers: Vec<wgpu::Buffer>,
    /// Should be rendered or not
    pub is_visible: bool,
    /// Objects with higher number get rendered later and appear "on top" when occupying the same space
    pub render_order: usize,
}
unsafe impl Send for Object {}
unsafe impl Sync for Object {}

/// Extra settings to customize objects on time of creation
#[derive(Debug, Clone)]
pub struct ObjectSettings {
    /// Should it be affected by camera?
    pub camera_effect: Option<std::sync::Arc<str>>,
    /// Shader Settings
    pub shader_settings: ShaderSettings,
}
impl Default for ObjectSettings {
    fn default() -> Self {
        Self {
            camera_effect: Some("main".into()),
            shader_settings: ShaderSettings::default(),
        }
    }
}
unsafe impl Send for ObjectSettings {}
unsafe impl Sync for ObjectSettings {}

/// A unified way to handle objects
///
/// This is a container for objects that is used to apply different operations on the objects at the same time.
/// It can deref to the object hashmap itself when needed.
pub struct ObjectStorage(std::collections::HashMap<String, Object>);
impl ObjectStorage {
    /// Creates a new object storage
    pub fn new() -> Self {
        ObjectStorage(std::collections::HashMap::new())
    }
}
impl Default for ObjectStorage {
    fn default() -> Self {
        Self::new()
    }
}
unsafe impl Send for ObjectStorage {}
unsafe impl Sync for ObjectStorage {}
crate::macros::impl_deref!(ObjectStorage, std::collections::HashMap<String, Object>);

/// Defines how the rotation axis is
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RotateAxis {
    #[doc(hidden)]
    X,
    #[doc(hidden)]
    Y,
    #[doc(hidden)]
    Z,
}
unsafe impl Send for RotateAxis {}
unsafe impl Sync for RotateAxis {}

/// Defines how the rotation amount is
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RotateAmount {
    #[doc(hidden)]
    Radians(f32),
    #[doc(hidden)]
    Degrees(f32),
}
unsafe impl Send for RotateAmount {}
unsafe impl Sync for RotateAmount {}

/// Defines full axes rotation information

impl ObjectStorage {
    /// Creates a new object
    #[deprecated]
    pub fn new_object(
        &mut self,
        name: impl StringBuffer,
        vertices: Vec<Vertex>,
        indices: Vec<UnsignedIntType>,
        settings: ObjectSettings,
        renderer: &mut Renderer,
    ) {
        match Object::new(name.clone(), vertices, indices, settings, renderer) {
            Ok(object) => {
                self.insert(name.as_string(), object);
            }
            Err(e) => {
                eprintln!("Could not create a new Object: {e:#?}");
            }
        }
    }

    /// Adds an object to the storage
    #[deprecated]
    pub fn add_object(&mut self, key: impl StringBuffer, object: Object) {
        fn add_object_inner(object_storage: &mut ObjectStorage, key: String, object: Object) {
            object_storage.insert(key, object);
        }
        add_object_inner(self, key.as_string(), object);
    }

    /// Allows for safe update of objects
    #[deprecated]
    pub fn update_object<T: Fn(&mut Object)>(&mut self, key: impl StringBuffer, callback: T) {
        fn update_object_inner<T: Fn(&mut Object)>(
            object_storage: &mut ObjectStorage,
            key: String,
            callback: T,
        ) {
            let object = object_storage.get_mut(&key);
            if let Some(object) = object {
                callback(object);
            }
        }
        update_object_inner(self, key.as_string(), callback);
    }
}

impl Object {
    /// Creates a new object
    ///
    /// Is used to define a new object and add it to the storage. This offers full customizability
    /// and a framework for in-engine shapes to be developed.
    pub fn new(
        name: impl StringBuffer,
        vertices: Vec<Vertex>,
        indices: Vec<UnsignedIntType>,
        settings: ObjectSettings,
        renderer: &mut Renderer,
    ) -> Result<Object, crate::error::Error> {
        let vertex_buffer = renderer.build_vertex_buffer(&vertices, &indices);

        let uniform = renderer.build_uniform_buffer(&vec![
            renderer.build_uniform_buffer_part("Transformation Matrix", Matrix4::IDENTITY),
            renderer
                .build_uniform_buffer_part("Color", crate::utils::default_resources::DEFAULT_COLOR),
        ]);

        let shader_source =
            ShaderBuilder::new(DEFAULT_SHADER.to_string(), settings.camera_effect.clone());
        let shader = renderer.build_shader(
            name.as_str(),
            shader_source.shader.clone(),
            Some(&uniform.1),
            settings.shader_settings,
        );

        let texture = renderer.build_texture(
            "Default Texture",
            TextureData::Bytes(DEFAULT_TEXTURE.to_vec()),
            crate::prelude::TextureMode::Clamp,
            //crate::prelude::TextureFormat::PNG
        )?;

        let instance = Instance::default();
        let instance_buffer = renderer.build_instance(vec![instance.build()]);

        Ok(Object {
            name: name.as_arc(),
            vertices,
            indices,
            pipeline: Pipeline {
                vertex_buffer: PipelineData::Data(vertex_buffer),
                shader: PipelineData::Data(shader),
                texture: PipelineData::Data(texture),
                uniform: PipelineData::Data(Some(uniform.0)),
            },
            instances: vec![instance],
            instance_buffer,
            uniform_layout: uniform.1,
            size: Vector3::ONE,
            position: Vector3::ZERO,
            rotation: Vector3::ZERO,
            changed: false,
            translation_matrix: Matrix4::IDENTITY,
            scale_matrix: Matrix4::IDENTITY,
            rotation_quaternion: Quaternion::IDENTITY,
            inverse_transformation_matrix: Matrix4::transpose(&Matrix4::inverse(
                &Matrix4::IDENTITY,
            )),
            color: crate::utils::default_resources::DEFAULT_COLOR,
            shader_builder: shader_source,
            shader_settings: settings.shader_settings,
            camera_effect: settings.camera_effect,
            uniform_buffers: vec![
                renderer.build_uniform_buffer_part("Transformation Matrix", Matrix4::IDENTITY),
                renderer.build_uniform_buffer_part(
                    "Color",
                    crate::utils::default_resources::DEFAULT_COLOR,
                ),
            ],
            is_visible: true,
            render_order: 0,
        })
    }

    // MARK: TRANSFORM

    /// Sets the name of the object
    pub fn set_name(&mut self, name: impl StringBuffer) -> &mut Self {
        self.name = name.as_arc();

        self
    }

    /// Scales an object. e.g. 2.0 doubles the size and 0.5 halves
    pub fn set_scale(&mut self, scale: impl Into<Vector3>) -> &mut Self {
        let scale = scale.into();
        self.size *= scale;

        let transformation_matrix = self.scale_matrix;
        let result = transformation_matrix * Matrix4::from_scale(scale);
        self.scale_matrix = result;
        self.inverse_matrices();

        self.changed = true;
        self
    }

    /// Resizes an object in pixels which are relative to the window
    pub fn resize(&mut self, size: impl Into<Vector3>) -> &mut Self {
        let size = size.into();
        self.size = size;
        self.scale_matrix = Matrix4::IDENTITY;

        self.set_scale(size)
    }

    /// Sets the rotation of the object in the axis you specify
    ///
    /// This function does NOT normalize the rotation.
    pub fn set_rotation(&mut self, rotation: impl Into<Vector3>) -> &mut Self {
        let rotation = rotation.into();
        self.rotation = rotation;
        self.rotation_quaternion = Quaternion::from_rotation_x(rotation.x)
            * Quaternion::from_rotation_y(rotation.y)
            * Quaternion::from_rotation_z(rotation.z);
        self.inverse_matrices();

        self.changed = true;
        self
    }

    /// Rotates the object in the axis you specify
    pub fn rotate(&mut self, amount: RotateAmount, axis: RotateAxis) -> &mut Self {
        let amount_radians = match amount {
            RotateAmount::Radians(amount) => amount,
            RotateAmount::Degrees(amount) => amount.to_radians(),
        };

        let axis = match axis {
            RotateAxis::X => {
                self.rotation.x += amount_radians;
                Quaternion::from_rotation_x(amount_radians)
            }
            RotateAxis::Y => {
                self.rotation.y += amount_radians;
                Quaternion::from_rotation_y(amount_radians)
            }
            RotateAxis::Z => {
                self.rotation.z += amount_radians;
                Quaternion::from_rotation_z(amount_radians)
            }
        };

        self.rotation_quaternion *= axis;
        self.inverse_matrices();

        self.changed = true;
        self
    }

    /// Moves the object by the amount you specify in the axis you specify
    #[deprecated]
    pub fn set_translation(&mut self, new_pos: impl Into<Vector3>) -> &mut Self {
        self.position -= new_pos.into();
        self.translation_matrix *= Matrix4::from_translation(self.position);

        self.inverse_matrices();
        self.changed = true;
        self
    }

    /// Moves the object by the amount you specify in the axis you specify
    pub fn translate(&mut self, new_pos: impl Into<Vector3>) -> &mut Self {
        self.position -= new_pos.into();
        self.translation_matrix *= Matrix4::from_translation(self.position);

        self.inverse_matrices();
        self.changed = true;
        self
    }
    /// Moves the object by the amount you specify in the axis you specify

    /// Sets the position of the object in 3D space relative to the window
    pub fn set_position(&mut self, new_pos: impl Into<Vector3>) -> &mut Self {
        let new_pos = new_pos.into();
        self.position = new_pos;
        self.translation_matrix = Matrix4::IDENTITY;

        self.translate(new_pos)
    }

    /// Changes the color of the object. If textures exist, the color of textures will change
    pub fn set_color(&mut self, red: f32, green: f32, blue: f32, alpha: f32) -> &mut Self {
        self.color = Vector4::new(red, green, blue, alpha);
        self.changed = true;
        self
    }

    /// Changes the render order of the Object.
    ///
    /// Objects with higher number get rendered later and appear "on top" when occupying the same space
    pub fn set_render_order(&mut self, render_order: usize) -> &mut Self {
        self.render_order = render_order;

        self
    }

    /// Replaces the object's texture with provided one
    ///
    /// This function previously served the role of [crate::Object::set_texture_raw]
    pub fn set_texture(
        &mut self,
        name: impl StringBuffer,
        texture_data: TextureData,
        texture_mode: TextureMode,
        renderer: &mut Renderer,
    ) -> Result<&mut Self, crate::error::Error> {
        let texture = renderer.build_texture(name, texture_data, texture_mode)?;
        Ok(self.set_texture_raw(texture))
    }

    /// Replaces the object's texture with provided one
    pub fn set_texture_raw(&mut self, texture: Textures) -> &mut Self {
        self.pipeline.texture = PipelineData::Data(texture);
        self.changed = true;

        self
    }

    /// This will flag object as changed and altered, leading to rebuilding parts, or entirety on next frame.
    /// Best used if you directly altered fields of the object. The functions normally flag the object as
    /// changed on every call anyways. But this function is to manually flag it yourself.
    pub fn flag_as_changed(&mut self, is_changed: bool) {
        self.changed = is_changed;
    }

    /// Sets if the object will be rendered or not
    pub fn set_visibility(&mut self, is_visible: bool) {
        self.is_visible = is_visible;
    }

    /// build an inverse of the transformation matrix to be sent to the gpu for lighting and other things.
    pub fn inverse_matrices(&mut self) {
        self.inverse_transformation_matrix = Matrix4::transpose(&Matrix4::inverse(
            &(self.translation_matrix
                * Matrix4::from_quat(self.rotation_quaternion)
                * self.scale_matrix),
        ));
    }
}
// MARK: UPDATE
// ============================= FOR UPDATING THE PIPELINE =============================
impl Object {
    /// Update and apply changes done to an object
    pub fn update(&mut self, renderer: &mut Renderer) {
        self.update_vertex_buffer(renderer);
        self.update_uniform_buffer(renderer);
        self.update_shader(renderer);
        self.update_instance_buffer(renderer);
        self.changed = false;
    }

    /// Update and apply changes done to an object and returns a pipeline
    pub fn update_and_return(
        &mut self,
        renderer: &mut Renderer,
    ) -> (crate::VertexBuffers, crate::UniformBuffers, crate::Shaders) {
        let vertex_buffer = self.update_vertex_buffer_and_return(renderer);
        let uniform_buffer = self.update_uniform_buffer_and_return(renderer);
        let shader = self.update_shader_and_return(renderer);
        self.changed = false;
        (vertex_buffer, uniform_buffer, shader)
    }

    /// Update and apply changes done to the vertex buffer
    pub fn update_vertex_buffer(&mut self, renderer: &mut Renderer) {
        let updated_buffer = renderer.build_vertex_buffer(&self.vertices, &self.indices);
        self.pipeline.vertex_buffer = PipelineData::Data(updated_buffer);
    }

    /// Returns the buffer with ownership
    pub fn update_vertex_buffer_and_return(
        &mut self,
        renderer: &mut Renderer,
    ) -> crate::VertexBuffers {
        let updated_buffer = renderer.build_vertex_buffer(&self.vertices, &self.indices);
        let updated_buffer_2 = renderer.build_vertex_buffer(&self.vertices, &self.indices);
        self.pipeline.vertex_buffer = PipelineData::Data(updated_buffer);

        updated_buffer_2
    }

    /// Update and apply changes done to the shader
    pub fn update_shader(&mut self, renderer: &mut Renderer) {
        let updated_shader = renderer.build_shader(
            self.name.as_ref(),
            self.shader_builder.shader.clone(),
            Some(&self.uniform_layout),
            self.shader_settings,
        );
        self.pipeline.shader = PipelineData::Data(updated_shader);
    }

    /// Returns the buffer with ownership
    pub fn update_shader_and_return(&mut self, renderer: &mut Renderer) -> crate::Shaders {
        let updated_shader = renderer.build_shader(
            self.name.as_ref(),
            self.shader_builder.shader.clone(),
            Some(&self.uniform_layout),
            self.shader_settings,
        );
        let updated_shader2 = renderer.build_shader(
            self.name.as_ref(),
            self.shader_builder.shader.clone(),
            Some(&self.uniform_layout),
            self.shader_settings,
        );
        self.pipeline.shader = PipelineData::Data(updated_shader);

        updated_shader2
    }

    fn update_uniform_buffer_inner(
        &mut self,
        renderer: &mut Renderer,
    ) -> (crate::UniformBuffers, wgpu::BindGroupLayout) {
        self.uniform_buffers[0] = renderer.build_uniform_buffer_part(
            "Transformation Matrix",
            self.translation_matrix
                * Matrix4::from_quat(self.rotation_quaternion)
                * self.scale_matrix,
        );
        self.uniform_buffers[1] = renderer.build_uniform_buffer_part("Color", self.color);

        let updated_buffer = renderer.build_uniform_buffer(&self.uniform_buffers);

        updated_buffer
    }

    /// Update and apply changes done to the uniform buffer
    pub fn update_uniform_buffer(&mut self, renderer: &mut Renderer) {
        let updated_buffer = self.update_uniform_buffer_inner(renderer);

        self.pipeline.uniform = PipelineData::Data(Some(updated_buffer.0));
        self.uniform_layout = updated_buffer.1;
    }

    /// Update and apply changes done to the uniform buffer and returns it
    pub fn update_uniform_buffer_and_return(
        &mut self,
        renderer: &mut Renderer,
    ) -> crate::UniformBuffers {
        let updated_buffer = self.update_uniform_buffer_inner(renderer);
        let updated_buffer2 = updated_buffer.clone();

        self.pipeline.uniform = PipelineData::Data(Some(updated_buffer.0));
        self.uniform_layout = updated_buffer.1;

        updated_buffer2.0
    }

    /// Updates the instance buffer
    pub fn update_instance_buffer(&mut self, renderer: &mut Renderer) {
        let instance_data = self
            .instances
            .iter()
            .map(Instance::build)
            .collect::<Vec<_>>();
        let instance_buffer = renderer.build_instance(instance_data);
        self.instance_buffer = instance_buffer;
    }

    /// Returns the buffer with ownership
    pub fn update_instance_buffer_and_return(&mut self, renderer: &mut Renderer) -> wgpu::Buffer {
        let instance_data = self
            .instances
            .iter()
            .map(Instance::build)
            .collect::<Vec<_>>();
        let instance_buffer = renderer.build_instance(instance_data.clone());
        let instance_buffer2 = renderer.build_instance(instance_data);

        self.instance_buffer = instance_buffer;
        instance_buffer2
    }
}
// MARK: REFERENCE
// ============================= FOR COPY OF PIPELINES =============================
impl Object {
    /// References another object's vertices
    pub fn reference_vertices(&mut self, object_id: impl StringBuffer) -> &mut Self {
        self.pipeline.vertex_buffer = PipelineData::Copy(object_id.as_string());
        self
    }

    /// References another object's shader
    pub fn reference_shader(&mut self, object_id: impl StringBuffer) -> &mut Self {
        self.pipeline.shader = PipelineData::Copy(object_id.as_string());
        self
    }

    /// References another object's texture
    pub fn reference_texture(&mut self, object_id: impl StringBuffer) -> &mut Self {
        self.pipeline.texture = PipelineData::Copy(object_id.as_string());
        self
    }

    /// References another object's uniform buffer
    pub fn reference_uniform_buffer(&mut self, object_id: impl StringBuffer) -> &mut Self {
        self.pipeline.uniform = PipelineData::Copy(object_id.as_string());
        self
    }

    // ============================= Instances =============================
    /// Add an instance to the object
    pub fn add_instance(&mut self, instance: Instance) -> &mut Self {
        self.instances.push(instance);
        self.changed = true;
        self
    }
}

// MARK: SHADER CONFIG

/// Configuration type for ShaderBuilder
pub type ShaderConfigs = Vec<(String, Box<dyn Fn(Option<std::sync::Arc<str>>) -> String>)>;

/// Helps with building and updating shader code
pub struct ShaderBuilder {
    /// the shader itself
    pub shader: String,
    /// Should the camera effect be applied
    pub camera_effect: Option<std::sync::Arc<str>>,
    /// configurations to be applied to the shader
    pub configs: ShaderConfigs,
}

impl ShaderBuilder {
    /// Creates a new shader builder
    pub fn new(shader_source: String, camera_effect: Option<std::sync::Arc<str>>) -> Self {
        let mut shader_builder = Self {
            shader: shader_source,
            camera_effect,
            configs: vec![
                (
                    "//@CAMERA_STRUCT".to_string(),
                    Box::new(|camera_effect| {
                        if camera_effect.is_some() {
                            r#"struct CameraUniforms {
                            camera_matrix: mat4x4<f32>,
                        };
                        @group(1) @binding(0)
                        var<uniform> camera_uniform: CameraUniforms;"#
                                .to_string()
                        } else {
                            "".to_string()
                        }
                    }),
                ),
                (
                    "//@CAMERA_VERTEX".to_string(),
                    Box::new(|camera_effect| {
                        if camera_effect.is_some() {
                            r#"out.position = camera_uniform.camera_matrix * model_matrix * (transform_uniform.transform_matrix * vec4<f32>(input.position, 1.0));"#
                        .to_string()
                        } else {
                            r#"out.position = model_matrix * (transform_uniform.transform_matrix * vec4<f32>(input.position, 1.0));"#.to_string()
                        }
                    }),
                ),
            ],
        };
        shader_builder.build();

        shader_builder
    }

    /// Sets the new shader
    pub fn set_shader(&mut self, new_shader: String) {
        self.shader = new_shader;
        self.build();
    }

    /// Builds the shader with the configuration defined
    pub fn build(&mut self) {
        for i in &self.configs {
            self.shader = self.shader.replace(&i.0, &i.1(self.camera_effect.clone()));
        }
    }
}

// MARK: Instance

/// Instance buffer data that is sent to GPU
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct InstanceRaw {
    /// The transformation matrix of the instance
    pub model: Matrix4,
}

/// Instance buffer data storage
#[derive(Debug, Clone, Copy)]
pub struct Instance {
    /// The position of the instance
    pub position: Vector3,
    /// The rotation of the instance
    pub rotation: Vector3,
    /// The scale of the instance
    pub scale: Vector3,
}

impl Instance {
    /// Creates a new instance
    #[deprecated]
    pub fn new(
        position: impl Into<Vector3>,
        rotation: impl Into<Vector3>,
        scale: impl Into<Vector3>,
    ) -> Self {
        Self {
            position: position.into(),
            rotation: rotation.into(),
            scale: scale.into(),
        }
    }

    /// Gathers all information and builds a Raw Instance to be sent to GPU
    pub fn build(&self) -> InstanceRaw {
        let position_matrix = Matrix4::IDENTITY * Matrix4::from_translation(self.position);
        let rotation_matrix = Matrix4::from_quat(
            Quaternion::from_rotation_x(self.rotation.x)
                * Quaternion::from_rotation_y(self.rotation.y)
                * Quaternion::from_rotation_z(self.rotation.z),
        );
        let scale_matrix = Matrix4::IDENTITY * Matrix4::from_scale(self.scale);
        InstanceRaw {
            model: position_matrix * rotation_matrix * scale_matrix,
        }
    }

    /// Sets the position
    pub fn set_position(&mut self, position: impl Into<Vector3>) {
        self.position = position.into();
    }

    /// Sets the rotation
    pub fn set_rotation(&mut self, rotation: impl Into<Vector3>) {
        self.rotation = rotation.into();
    }

    /// Sets the scale
    pub fn set_scale(&mut self, scale: impl Into<Vector3>) {
        self.scale = scale.into();
    }
}
impl Default for Instance {
    fn default() -> Self {
        Self {
            position: Vector3::ZERO,
            rotation: Vector3::ZERO,
            scale: Vector3::ONE,
        }
    }
}
impl InstanceRaw {
    /// Instance's layout description
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<InstanceRaw>() as wgpu::BufferAddress,
            // We need to switch from using a step mode of Vertex to Instance
            // This means that our shaders will only change to use the next
            // instance when the shader starts processing a new instance
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                // A mat4 takes up 4 vertex slots as it is technically 4 vec4s. We need to define a slot
                // for each vec4. We'll have to reassemble the mat4 in the shader.
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 3,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 4,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 8]>() as wgpu::BufferAddress,
                    shader_location: 5,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 12]>() as wgpu::BufferAddress,
                    shader_location: 6,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}
