use bevy::{
    ecs::system::Resource,
    math::Vec3,
    prelude::{Deref, DerefMut},
    reflect::Reflect,
};
use bevy_inspector_egui::{inspector_options::ReflectInspectorOptions, InspectorOptions};
use rand::Rng;

#[derive(Debug, Clone, Resource, InspectorOptions, Deref, DerefMut, Default, Reflect)]
#[reflect(InspectorOptions)]
pub struct SpawnProperty(Vec<Vec3>);

impl SpawnProperty {
    pub fn new<T: IntoVec3Vec>(spawn_points: T) -> Self {
        Self(spawn_points.into_vec3_vec())
    }

    #[allow(dead_code)]
    pub fn empty() -> Self {
        Self(Vec::new())
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    #[allow(dead_code)]
    pub fn points(&self) -> &[Vec3] {
        &self.0
    }

    pub fn random_point(&self) -> Vec3 {
        let mut rng = rand::thread_rng();
        let index = rng.gen_range(0..self.0.len());
        self.0[index]
    }
}

pub trait IntoVec3Vec {
    fn into_vec3_vec(self) -> Vec<Vec3>;
}

impl IntoVec3Vec for Vec3 {
    fn into_vec3_vec(self) -> Vec<Vec3> {
        vec![self]
    }
}

impl IntoVec3Vec for (Vec3, Vec3) {
    fn into_vec3_vec(self) -> Vec<Vec3> {
        vec![self.0, self.1]
    }
}

impl IntoVec3Vec for (Vec3, Vec3, Vec3) {
    fn into_vec3_vec(self) -> Vec<Vec3> {
        vec![self.0, self.1, self.2]
    }
}

impl IntoVec3Vec for (Vec3, Vec3, Vec3, Vec3) {
    fn into_vec3_vec(self) -> Vec<Vec3> {
        vec![self.0, self.1, self.2, self.3]
    }
}

impl IntoVec3Vec for (Vec3, Vec3, Vec3, Vec3, Vec3) {
    fn into_vec3_vec(self) -> Vec<Vec3> {
        vec![self.0, self.1, self.2, self.3, self.4]
    }
}

impl IntoVec3Vec for (Vec3, Vec3, Vec3, Vec3, Vec3, Vec3) {
    fn into_vec3_vec(self) -> Vec<Vec3> {
        vec![self.0, self.1, self.2, self.3, self.4, self.5]
    }
}
