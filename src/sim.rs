// temporary to suppress compiler warnings
#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_mut)]

use std::{collections::HashMap, hash::Hash, iter::Zip, slice::{Iter, IterMut}};
use crate::{ math::*, output::*, constants::* };

#[derive(Debug, Clone, Default)]
pub struct PhysKinematic {
    _physcategory: PhysicsCategory, // the physics processing category
    _position: DVec3, // position in 3D space
    _velocity: SVec3, // velocity in 3D space
    _acceleration: SVec3, // acceleration in 3D space
    _radius: f32, // minimum bounding radius of the body
    _mass: f64, // mass in kg
}

impl PhysKinematic {
    fn translate(&mut self, translation: DVec3) {
        self._position += translation
    }
    
    fn time_adjusted_bounding_radius(&self, dt: f32) -> f64 {
        ((2.0 * self._radius) + (self._velocity.magnitude() * dt) + (self._acceleration.magnitude() * dt * dt * 0.5)) as f64
    }
}

#[derive(Debug, Clone, Default)]
pub struct PhysDynamic {
    _f_independent: SVec3, // spatially independent forces
    _f_spatially_dep: SVec3, // position dependent forces
    _f_velocity_dep: SVec3, // velocity dependent forces
    _f_torque: SVec3, // torque
    _grav_param: f64, // standard gravitational param
}

impl PhysDynamic {
    pub fn fnet(&self) -> SVec3 {
        self._f_independent + self._f_spatially_dep + self._f_velocity_dep
    }
}

#[derive(Debug, Clone, Default)]
pub struct PhysRotational {
    _orientation: Quat, // orientation of the body
    _angular_velocity: SVec3, // angular velocity
    _angular_acceleration: SVec3, // angular velocity
    _inertia_tensor: SMatrix3x3,
}

#[derive(Debug, Clone, Default)]
pub struct PhysCollision {
}

/// A convenience type which bundles a bodies disjoint physics data
#[derive(Debug, Clone)]
pub struct PhysicsBodyRef<'a> {
    _kinematic: &'a PhysKinematic,
    _dynamic: &'a PhysDynamic,
    _rotation: &'a PhysRotational,
    _collision: &'a PhysCollision,
    _id: usize,
}

impl<'a> PhysicsBodyRef<'a> {
    pub fn id(&self) -> usize {
        self._id
    }

    pub fn mass(&self) -> f64 {
        self._kinematic._mass
    }

    pub fn position(&self) -> DVec3 {
        self._kinematic._position
    }

    pub fn velocity(&self) -> SVec3 {
        self._kinematic._velocity
    }

    pub fn acceleration(&self) -> SVec3 {
        self._kinematic._acceleration
    }

    pub fn momentum(&self) -> f64 {
        self._kinematic._velocity.magnitude() as f64 * self._kinematic._mass
    }

    pub fn kinetic_energy(&self) -> f64 {
        let velocity = self._kinematic._velocity.magnitude() as f64;
        0.5 * self._kinematic._mass * velocity * velocity
    }
    
    pub fn physics_category(&self) -> PhysicsCategory {
        self._kinematic._physcategory
    }

    pub fn bounding_distance_to(&self, other: &PhysicsBodyRef) -> f64 {
        let p_this = self._kinematic._position;
        let r_this = other._kinematic._radius as f64;
        let p_other = other._kinematic._position;
        let r_other = other._kinematic._radius as f64;
        p_this.length_to(&p_other) - r_this - r_other
    }

    pub fn centers_distance_to(&self, other: &PhysicsBodyRef) -> f64 {
        self._kinematic._position.length_to(&other._kinematic._position)
    }
}

#[derive(Debug, Clone)]
pub struct PhysicsFrame {
    spatial: Vec<PhysKinematic>,
    forces: Vec<PhysDynamic>,
    rotations: Vec<PhysRotational>,
    collisions: Vec<PhysCollision>,
    name_index: HashMap<String, Vec<usize>>,
    frame_number: usize,
    simtime: f64,
    timestep: f64,

    // implement some spatial partitioning structure when necessary
}

impl PhysicsFrame {
    pub fn new() -> Self {
        PhysicsFrame {
            spatial: Vec::new(),
            forces: Vec::new(),
            rotations: Vec::new(),
            collisions: Vec::new(),
            name_index: HashMap::new(),
            frame_number: 0,
            simtime: 0.0,
            timestep: 0.0,
        }
    }

    pub fn frame_number(&self) -> usize {
        self.frame_number
    }

    pub fn sim_time(&self) -> f64 {
        self.simtime
    }

    pub fn time_step(&self) -> f64 {
        self.timestep
    }

    pub fn bodies(&self) -> &Vec<PhysicsBodyRef> {
        unimplemented!() // previous implementation removed for now, turn this into an iterator???
    }
    
    pub fn get_body_ref(&self, id: usize) -> Option<PhysicsBodyRef> {
        if let (Some(s), Some(f), Some(r), Some(c)) = self.physics_data_from_id(id) {
            Some(PhysicsBodyRef {
                _kinematic: s,
                _dynamic: f,
                _rotation: r,
                _collision: c,
                _id: id,
            })
        } else {
            None
        }
    }
    
    pub fn get_named_bodies(&self, name: &str) -> Vec<PhysicsBodyRef> {
        let name = name.to_ascii_uppercase();
        if let Some(ids) = self.name_index.get(name.as_str()) {
            let mut result = Vec::new();
            for id in ids {
                if let Some(body_ref) = self.get_body_ref(*id) {
                    result.push(body_ref);
                }
            }
            result
        } else {
            Vec::default()
        }
    }

    pub fn kinematic_data(&self) -> std::slice::Iter<PhysKinematic> {
        self.spatial.iter()
    }

    pub fn dynamic_data(&self) -> std::slice::Iter<PhysDynamic> {
        self.forces.iter()
    }

    pub fn dynamic_data_mut(&mut self) -> std::slice::IterMut<PhysDynamic> {
        self.forces.iter_mut()
    }

    pub fn rotation_data(&self) -> std::slice::Iter<PhysRotational> {
        self.rotations.iter()
    }

    pub fn rotation_data_mut(&mut self) -> std::slice::IterMut<PhysRotational> {
        self.rotations.iter_mut()
    }

    pub fn collision_data(&self) -> std::slice::Iter<PhysCollision> {
        self.collisions.iter()
    }

    pub fn collision_data_mut(&mut self) -> std::slice::IterMut<PhysCollision> {
        self.collisions.iter_mut()
    }

    pub fn spatial_data_mut(&mut self) -> std::slice::IterMut<PhysKinematic> {
        self.spatial.iter_mut()
    }
    
    pub fn dynamic_integration_data(&self) -> Zip<Iter<PhysKinematic>, Iter<PhysDynamic>> {
        self.spatial.iter().zip(self.forces.iter())
    }

    pub fn dynamic_integration_data_mut(&mut self) -> Zip<IterMut<PhysKinematic>, IterMut<PhysDynamic>> {
        self.spatial.iter_mut().zip(self.forces.iter_mut())
    }

    pub fn make_physics_body(&mut self) -> PhysicsBodyBuilder {
        PhysicsBodyBuilder {
            _reference_frame: self,
            _template: None,
            _physics_category: None,
            _name: None,
            _velocity: None,
            _position: None,
            _orientation: None,
            _angular_velocity: None,
            _mass: None,
            _bounding_radius: None,
            _grav_param: None,
            _relative_body_id: None,
            _with_relative_rotation: false,
        }
    }

    pub fn add_named_physics_body(&mut self, body: PhysicsBodyRef, name: String) -> usize {
        let id = self.add_physics_body(body);
        let name = name.to_ascii_uppercase();
        self.name_index.entry(name).or_default().push(id);
        return id
    }

    pub fn add_physics_body(&mut self, body: PhysicsBodyRef) -> usize {
        let id = self.spatial.len();
        assert_eq!(id, self.forces.len());
        assert_eq!(id, self.rotations.len());
        assert_eq!(id, self.collisions.len()); // until we have a more sophisticaed impl, just assert that our arrays stay the same length

        self.spatial.push(body._kinematic.clone());
        self.forces.push(body._dynamic.clone());
        self.rotations.push(body._rotation.clone());
        self.collisions.push(body._collision.clone());
        
        return id;
    }

    pub fn physics_data_from_id(&self, id: usize) -> (Option<&PhysKinematic>, Option<&PhysDynamic>, Option<&PhysRotational>, Option<&PhysCollision>) {
        (
            self.spatial.get(id),
            self.forces.get(id),
            self.rotations.get(id), // cant do this due to RefCell
            self.collisions.get(id),
        )
    }

    pub fn translate_origin(&mut self, translation: DVec3) {
        for spatial_data in self.spatial_data_mut() {
            spatial_data.translate(translation);
        }
    }

    pub fn get_data<'a, T: UpdateData<'a>>() -> (<T as UpdateData<'a>>::Reads, <T as UpdateData<'a>>::Writes) {
        unimplemented!()
    }
}

pub trait UpdateData<'a> {
    type Reads;
    type Writes;

    fn update(reads: Self::Reads, writes: Self::Writes);
}

struct TranslationSystem;
impl<'a> UpdateData<'a> for TranslationSystem {
    type Reads = ();
    type Writes = (&'a mut PhysKinematic, &'a mut PhysDynamic);

    fn update(reads: Self::Reads, data: Self::Writes) {
        unimplemented!()
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum PhysicsCategory {
    Gravitational, // generally large bodies, affected by gravity, and also affect everything else with gravity
    Dynamic, // objects which are affected by gravity but do not have a gravitational influence of their own
}

impl Default for PhysicsCategory {
    fn default() -> Self {
        Self::Dynamic
    }
}

pub struct PhysicsBodyBuilder<'a> {
    _reference_frame: &'a mut PhysicsFrame,
    _template: Option<PhysicsBodyRef<'a>>,
    _physics_category: Option<PhysicsCategory>,
    _name: Option<String>,
    _velocity: Option<SVec3>,
    _position: Option<DVec3>,
    _orientation: Option<Quat>,
    _angular_velocity: Option<SVec3>,
    _mass: Option<f64>,
    _bounding_radius: Option<f32>,
    _grav_param: Option<f64>,
    _relative_body_id: Option<usize>,
    _with_relative_rotation: bool,
}

impl<'a> PhysicsBodyBuilder<'a> {
    pub fn named(mut self, name: &str) -> Self {
        self._name = Some(String::from(name));
        self
    }

    pub fn with_transform(mut self, pos: DVec3, rot: Option<Quat>) -> Self {
        self._position = Some(pos);
        self._orientation = rot;
        self
    }

    pub fn with_velocity(mut self, vel: SVec3) -> Self {
        self._velocity = Some(vel);
        self
    }

    pub fn with_mass(mut self, mass: f64) -> Self {
        self._mass = Some(mass);
        self
    }

    pub fn with_grav_param(mut self, standard_gravitational_parameter: f64) -> Self {
        self._grav_param = Some(standard_gravitational_parameter);
        self
    }

    pub fn with_bounding_radius(mut self, bounding_radius: f32) -> Self {
        self._bounding_radius = Some(bounding_radius);
        self
    }

    pub fn with_physics_category(mut self, physics_category: PhysicsCategory) -> Self {
        self._physics_category = Some(physics_category);
        self
    }

    pub fn relative_to(mut self, id: usize) -> Self {
        self._relative_body_id = Some(id);
        self
    }

    /// Constructs then validates and adds a new PhysicsBody to the simulation
    pub fn add(self) -> usize {
        let mut frame = self._reference_frame;
        
        let mut kinematic = PhysKinematic {
            _physcategory: self._physics_category.unwrap_or(PhysicsCategory::default()),
            _radius: self._bounding_radius.unwrap_or(0.0f32),
            _position: self._position.unwrap_or(DVec3::default()),
            _velocity: self._velocity.unwrap_or(SVec3::default()),
            _acceleration: SVec3::default(),
            _mass: self._mass.unwrap_or(1.0f64),
        };
        let mut dynamic = PhysDynamic {
            _grav_param: self._grav_param.unwrap_or(self._mass.unwrap_or(1.0f64) * G),
            _f_independent: SVec3::default(),
            _f_spatially_dep: SVec3::default(),
            _f_velocity_dep: SVec3::default(),
            _f_torque: SVec3::default(),
        };
        let mut rotation = PhysRotational {
            _orientation: self._orientation.unwrap_or(Quat::default()),
            _angular_velocity: self._angular_velocity.unwrap_or(SVec3::default()),
            _angular_acceleration: SVec3::default(),
            _inertia_tensor: SMatrix3x3::default(),
        };
        let mut collision = PhysCollision::default();
        
        // add the relative bodies transform and velocity
        if let Some(id) = self._relative_body_id {
            if let (Some(relative_body_spatial), _, _, _) = frame.physics_data_from_id(id) {
                kinematic._position += relative_body_spatial._position;
                kinematic._velocity += relative_body_spatial._velocity;

                if self._with_relative_rotation {
                    unimplemented!("Relative rotation is not implemented");
                }
            }
        }
        
        // TODO HERE: VALIDATE THE BODY PARAMETERS
        
        let body = PhysicsBodyRef {
            _kinematic: &kinematic,
            _dynamic: &dynamic,
            _rotation: &rotation,
            _collision: &collision,
            _id: 0usize,
        };

        
        if let Some(name) = self._name {
            return frame.add_named_physics_body(body, name)
        } else {
            return frame.add_physics_body(body)
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum IntegrationMethod {
    Euler,
    SemiImplicitEuler,
    VelocityVerlet,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TerminationCondition {
    ElapsedTime(f64),
}

#[derive(Debug)]
pub struct Simulation {
    present_state: PhysicsFrame,
    timestep: f64, // seconds
    integration_method: IntegrationMethod,
    termination_conditions: Vec<TerminationCondition>,
    output_device: Option<OutputDevice>,
}

impl Simulation {
    pub fn new() -> Simulation {
        Simulation {
            present_state: PhysicsFrame::new(),
            timestep: 1.0f64,
            integration_method: IntegrationMethod::VelocityVerlet,
            termination_conditions: Vec::new(),
            output_device: None,
        }
    }

    pub fn make_physics_body(&mut self) -> PhysicsBodyBuilder {
        self.present_state.make_physics_body()
    }

    pub fn present(&self) -> &PhysicsFrame {
        &self.present_state
    }

    pub fn set_output_device(&mut self, device: OutputDevice) {
        self.output_device = Some(device)
    }

    pub fn system_kinetic_energy(&self) -> f64 {
        let mut sum = 0.0;
        let data = self.present().kinematic_data();

        for body in data {
            let v = body._velocity.magnitude() as f64;
            sum += (body._mass / 2.0) * (v * v);
        }
        sum
    }
    
    pub fn system_potential_energy(&self) -> f64 {
        let mut sum = 0.0;
        // calculate gravitational potential energy
        
        for (i, body) in self.present().kinematic_data().enumerate() {
            for (k, other) in self.present().kinematic_data().enumerate() {
                if i != k {
                    sum += -(G * other._mass * body._mass) / other._position.length_to(&body._position);
                }
            }
        }
        sum
    }

    pub fn system_total_mass(&self) -> f64 {
        let mut sum = 0.0;
        for body in self.present().kinematic_data() {
            sum += body._mass;
        }
        sum
    }
    
    // TODO: DIVIDE AND CONQUER FORCE CALCULATIONS WHERE POSSIBLE
    pub fn calculate_independent_forces(&self) {
        unimplemented!();
    }

    pub fn calculate_spatially_dependent_forces(&self, frame: &mut PhysicsFrame) {
        self.calculate_gravitational_forces(frame);
    }

    pub fn calculate_velocity_dependent_forces(&self) {
        unimplemented!();
    }
    
    pub fn calculate_gravitational_forces(&self, frame: &mut PhysicsFrame) {
        let new_data = frame.dynamic_integration_data_mut();        
        for (i, (new_body_kinematic, new_body_dynamic)) in new_data.enumerate() {

            let old_data = self.present().dynamic_integration_data();
            for (k, (old_body_kinematic, old_body_dynamic)) in old_data.enumerate() {
                
                // only collect gravitational influences from gravitational bodies
                if old_body_kinematic._physcategory == PhysicsCategory::Gravitational {
                    // don't impart forces on yourself
                    if i != k { 
                        // F = G ((m1 * m2) / r^2)
                        let r = DVec3::length_to(&old_body_kinematic._position, &new_body_kinematic._position);
                        let m2 = new_body_kinematic._mass;
                        let f = (old_body_dynamic._grav_param * m2) / (r * r);
                        new_body_dynamic._f_spatially_dep += (new_body_kinematic._position.normal_vector_toward(&old_body_kinematic._position) * f).into();
                    }
                }
            }
        }
    }
    
    fn clear_spatially_dependent_forces(&self, frame: &mut PhysicsFrame) {
        for body in frame.dynamic_data_mut() {
            body._f_spatially_dep = SVec3::zero();
            body._f_velocity_dep = SVec3::zero();
        }
    }

    fn clear_accelerations_and_spatially_dependent_forces(&self, frame: &mut PhysicsFrame) {
        let data = frame.dynamic_integration_data_mut();

        for (body_kinematic, body_dynamic) in data {
            body_kinematic._acceleration = SVec3::zero();
            body_dynamic._f_spatially_dep = SVec3::zero();
            body_dynamic._f_velocity_dep = SVec3::zero();
        }
    }

    pub fn set_termination_condition(&mut self, condition: TerminationCondition) {
        self.termination_conditions.push(condition); // TODO: This is dumb too, test for duplicates and sanity check

        // TODO: Sort the conditions
    }

    pub fn step_simulation(&mut self) {
        // step 1: compute possible collisions and the exact time/position they occur
        //         treat acceleration as being constant during this step. quadratic root finding
        let mut frame = self.present().clone();

        // step 2: integrate accelerations and velocities
        match self.integration_method {
            IntegrationMethod::Euler => {
                self.clear_accelerations_and_spatially_dependent_forces(&mut frame);
                self.calculate_spatially_dependent_forces(&mut frame);

                for (body_kinematic, body_dynamic) in frame.dynamic_integration_data_mut() {
                    body_kinematic._acceleration = body_dynamic.fnet() / body_kinematic._mass;
                    body_kinematic._position += (body_kinematic._velocity * self.timestep).into(); // position then velocity
                    body_kinematic._velocity += body_kinematic._acceleration * self.timestep;
                }
            }

            IntegrationMethod::SemiImplicitEuler => {
                self.clear_accelerations_and_spatially_dependent_forces(&mut frame);
                self.calculate_spatially_dependent_forces(&mut frame);

                for (body_kinematic, body_dynamic) in frame.dynamic_integration_data_mut() {
                    body_kinematic._acceleration = body_dynamic.fnet() / body_kinematic._mass;
                    body_kinematic._velocity += body_kinematic._acceleration * self.timestep;
                    body_kinematic._position += (body_kinematic._velocity * self.timestep).into(); // velocity then position
                }
            }
            
            IntegrationMethod::VelocityVerlet => {
                self.clear_accelerations_and_spatially_dependent_forces(&mut frame);
                self.calculate_spatially_dependent_forces(&mut frame);

                // integrate velocities first
                for (body_kinematic, body_dynamic) in frame.dynamic_integration_data_mut() {
                    let dt = self.timestep; // dT
                    let p = body_kinematic._position; // p(T)
                    let v = body_kinematic._velocity; // v(T)
                    let a = body_dynamic.fnet() / body_kinematic._mass; // a(T)

                    body_kinematic._position = p + (v * dt) + 0.5 * a * (dt * dt);
                    body_kinematic._acceleration = a;
                }
                
                self.clear_spatially_dependent_forces(&mut frame);
                self.calculate_spatially_dependent_forces(&mut frame); // recalculate forces for new accelerations

                // integrate new accelerations sampled at the beginning and end of the timestep
                for (body_kinematic, body_dynamic) in frame.dynamic_integration_data_mut() {
                    let dt = self.timestep; // dT
                    let v = body_kinematic._velocity; // v(T)
                    let a = body_kinematic._acceleration; // a(T) // we saved the accelerations we calculated initially here
                    let b = body_dynamic.fnet() / body_kinematic._mass; // a(T + dT) // we already changed p(T) to p(T + dT), so we have new p(T) accelerations

                    body_kinematic._velocity = v + 0.5 * (a + b) * dt;
                    body_kinematic._acceleration = b;
                }
            }
        }
        
        frame.timestep = self.timestep;
        frame.simtime += self.timestep;
        frame.frame_number += 1;
        self.present_state = frame;
    }
    
    fn test_termination_conditions(&self) -> bool {
        // return true if we are still running, false if a condition is met
        if self.termination_conditions.is_empty() {
            return true
        } else {
            for condition in self.termination_conditions.iter() {
                match condition {
                    TerminationCondition::ElapsedTime(t) => {
                        return self.present().simtime < *t
                    },
                }
            }
            unreachable!()
        }
    }

    // run the simulation for the desired time in seconds
    pub fn run(&mut self) {
        while self.test_termination_conditions() {
            self.step_simulation();
            if let Some(output) = self.output_device.as_ref() {
                output.output(&self);
            }
        }
    }
}

impl MemUse for PhysicsFrame {
    fn memory_use(&self) -> usize {
        let mut total = 0;
        total += ::std::mem::size_of_val(&self.spatial);
        total += ::std::mem::size_of_val(&self.forces);
        total += ::std::mem::size_of_val(&self.rotations);
        total += ::std::mem::size_of_val(&self.collisions);
        total += ::std::mem::size_of_val(&self.name_index);
        total += ::std::mem::size_of_val(&self.frame_number);
        total += ::std::mem::size_of_val(&self.simtime);
        total
    }
}

impl MemUse for Simulation {
    fn memory_use(&self) -> usize {
        let mut total = 0;
        total += self.present_state.memory_use();
        total += ::std::mem::size_of_val(&self.timestep);
        total += ::std::mem::size_of_val(&self.integration_method);
        total += ::std::mem::size_of_val(&self.termination_conditions);
        total += if let Some(device) = &self.output_device { device.memory_use() } else { ::std::mem::size_of_val(&self.output_device) };
        total
    }    
}
