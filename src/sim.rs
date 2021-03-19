use std::{ collections::HashMap, hash::Hash };
use crate::{ math::*, output::*, constants::* };

#[derive(Debug, Clone)]
pub struct PhysicsBody {
    pub mass: f64, // kg
    pub bounding_radius: f64, // m
    pub position: Vec3, // m
    pub velocity: Vec3, // m/s
    pub acceleration: Vec3, // m/s^2
    pub net_force: Vec3, // N
    pub orientation: Quat, // quat
    pub angular_velocity: Vec3, // rad/s
    pub gravitational_parameter: f64, // m^3 * s^-2
    pub physics_category: PhysicsCategory, // todo: refactor this out into the simulation
}

impl PhysicsBody {
    fn new() -> PhysicsBody {
        PhysicsBody {
            mass: 1.0f64,
            bounding_radius: 0.0f64,
            position: Vec3::zero(),
            velocity: Vec3::zero(),
            acceleration: Vec3::zero(),
            net_force: Vec3::zero(),
            orientation: Quat::zero(),
            angular_velocity: Vec3::zero(),
            gravitational_parameter: 0.0,
            physics_category: PhysicsCategory::Dynamic,
        }
    }

    pub fn kinetic_energy(&self) -> f64 {
        let velocity = self.velocity.magnitude();
        0.5 * self.mass * velocity * velocity
    }

    pub fn bounding_distance_to(&self, other: &PhysicsBody) -> f64 {
        self.position.length_to(&other.position) - self.bounding_radius - other.bounding_radius
    }
}

#[derive(Debug, Clone)]
pub struct PhysicsFrame {
    time: f64,
    named: HashMap<String, Vec<usize>>,
    objects: Vec<PhysicsBody>,
    frame_number: usize,
}

impl PhysicsFrame {
    pub fn new(t: f64) -> Self {
        PhysicsFrame {
            time: t,
            objects: Vec::new(),
            named: HashMap::new(),
            frame_number: 0,
        }
    }

    pub fn frame_number(&self) -> usize {
        self.frame_number
    }

    pub fn frame_time(&self) -> f64 {
        self.time
    }

    pub fn bodies(&self) -> &Vec<PhysicsBody> {
        &self.objects
    }

    pub fn add_object(&mut self, obj: PhysicsBody, name: Option<&str>) -> usize {
        let id = self.objects.len();
        self.objects.push(obj);
        
        if let Some(name) = name {
            self.named.entry(name.to_ascii_uppercase()).or_default().push(id);
        }
        return id;
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum PhysicsCategory {
    Gravitational, // generally large bodies, affected by gravity, and also affect everything else with gravity
    Dynamic, // objects which are affected by gravity but do not affect other objects with gravity
}

pub struct PhysicsBodyBuilder<'a> {
    __sim: &'a mut Simulation,
    _template: Option<PhysicsBody>,
    _category: Option<PhysicsCategory>,
    _name: Option<String>,
    _vel: Option<Vec3>,
    _pos: Option<Vec3>,
    _rot: Option<Vec3>,
    _mass: Option<f64>,
    _radius: Option<f64>,
    _grav_param: Option<f64>,
    _relative_body_id: Option<usize>,
    _with_relative_rotation: bool,
}

impl<'a> PhysicsBodyBuilder<'a> {
    pub fn named(mut self, name: &str) -> Self {
        self._name = Some(String::from(name));
        self
    }

    pub fn with_transform(mut self, pos: Vec3, rot: Option<Vec3>) -> Self {
        self._pos = Some(pos);
        self._rot = rot;
        self
    }

    pub fn with_velocity(mut self, vel: Vec3) -> Self {
        self._vel = Some(vel);
        self
    }

    pub fn relative_to(mut self, id: usize) -> Self {
        self._relative_body_id = Some(id);
        self
    }

    /// Constructs then validates and adds a new PhysicsBody to the simulation
    pub fn add(self) -> usize {
        let template = self._template.clone().unwrap_or(PhysicsBody::new());
        let mut body = template.clone();
        
        body.mass = self._mass.unwrap_or(template.mass);
        body.bounding_radius = self._radius.unwrap_or(template.bounding_radius);
        body.position = self._pos.unwrap_or(template.position);
        body.velocity = self._vel.unwrap_or(template.velocity);

        // add the relative bodies transform and velocity
        if let Some(id) = self._relative_body_id {
            if let Some(relative_body) = self.__sim.body_from_id(id) {
                body.position += relative_body.position;
                body.velocity += relative_body.velocity;

                if self._with_relative_rotation {
                    unimplemented!("Relative rotation is not implemented");
                }
            }
        }
        
        body.gravitational_parameter = if let Some(grav_param) = self._grav_param {
            grav_param
        } else {
            body.mass * G
        };

        // TODO HERE: VALIDATE THE BODY PARAMETERS
        
        // are we inserting into a running simulation?
        if self.__sim.present_state.time > 0.0 {
            unimplemented!("Adding physics bodies during a running simulation is not implemented")
        } else {
            let id = self.__sim.initial_state.add_object(body, self._name.as_deref());
            self.__sim.present_state = self.__sim.initial_state.clone();
            self.__sim.previous_state = self.__sim.initial_state.clone();
            return id;
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
    initial_state: PhysicsFrame,
    present_state: PhysicsFrame,
    previous_state: PhysicsFrame,
    timestep: f64, // seconds
    integration_method: IntegrationMethod,
    termination_conditions: Vec<TerminationCondition>,
    output_device: Option<OutputDevice>,
}

impl Simulation {
    pub fn new() -> Simulation {
        Simulation {
            initial_state: PhysicsFrame::new(0.0),
            present_state: PhysicsFrame::new(0.0),
            previous_state: PhysicsFrame::new(0.0),
            timestep: 1.0f64,
            integration_method: IntegrationMethod::VelocityVerlet,
            termination_conditions: Vec::new(),
            output_device: None,
        }
    }

    pub fn present_frame(&self) -> &PhysicsFrame {
        &self.present_state
    }

    pub fn body_from_id(&self, id: usize) -> Option<&PhysicsBody> {
        self.present_state.objects.get(id)
    }

    pub fn bodies_with_name(&self, name: &str) -> Vec<(usize, &PhysicsBody)> {
        let mut result = Vec::new();
        if let Some(body_ids) = self.present_state.named.get(name.to_ascii_uppercase().as_str()) {
            for id in body_ids {
                if let Some(body) = self.present_state.objects.get(*id) {
                    result.push((*id, body));
                }
            }
        }
        result
    }

    fn set_global_timestep(&mut self, t: f64) {
        self.timestep = t;
    }

    pub fn set_output_device(&mut self, device: OutputDevice) {
        self.output_device = Some(device)
    }
    
    pub fn make_physics_body_from_template(&mut self, template: PhysicsBody) -> PhysicsBodyBuilder {
        PhysicsBodyBuilder {
            __sim: self,
            _template: Some(template),
            _category: None,
            _name: None,
            _vel: None,
            _pos: None,
            _rot: None,
            _mass: None,
            _radius: None,
            _grav_param: None,
            _relative_body_id: None,
            _with_relative_rotation: false,
        }
    }

    pub fn system_kinetic_energy(&self) -> f64 {
        let mut sum = 0.0;
        for body in self.present_state.objects.iter() {
            let v = body.velocity.magnitude();
            sum += (body.mass / 2.0) * (v * v);
        }
        sum
    }

    pub fn system_potential_energy(&self) -> f64 {
        let mut sum = 0.0;
        // calculate gravitational potential energy
        for (i, body) in self.present_state.objects.iter().enumerate() {
            for (k, other) in self.present_state.objects.iter().enumerate() {
                if i != k {
                    sum += -(G * other.mass * body.mass) / other.position.length_to(&body.position);
                }
            }
        }
        sum
    }

    pub fn system_total_mass(&self) -> f64 {
        let mut sum = 0.0;
        for body in self.present_state.objects.iter() {
            sum += body.mass;
        }
        sum
    }

    pub fn calculate_gravitational_forces(&mut self) {
        let bodies_other = &self.present_state.objects.clone();
        let bodies_now = &mut self.present_state.objects;

        for (i, body) in bodies_now.iter_mut().enumerate() {
            for (k, body_other) in bodies_other.iter().enumerate() {
                // only collect gravitational influences from gravitational bodies
                if body_other.physics_category == PhysicsCategory::Gravitational {
                    // don't impart forces on yourself
                    if i != k { 
                        // F = G ((m1 * m2) / r^2)
                        let r = Vec3::length_to(&body_other.position, &body.position);
                        let m2 = body.mass;
                        let f = (body_other.gravitational_parameter * m2) / (r * r);
                        body.net_force += body.position.normal_vector_toward(&body_other.position) * f;
                    }
                }
            }
        }
    }

    fn clear_forces(&mut self) {
        for mut body in &mut self.present_state.objects {
            body.net_force = Vec3::zero();
        }
    }

    fn clear_accelerations_and_forces(&mut self) {
        for mut body in &mut self.present_state.objects {
            body.acceleration = Vec3::zero();
            body.net_force = Vec3::zero();
        }
    }

    pub fn set_termination_condition(&mut self, condition: TerminationCondition) {
        self.termination_conditions.push(condition); // TODO: This is dumb too, test for duplicates and sanity check

        // TODO: Sort the conditions
    }

    pub fn step_simulation(&mut self) {
        // step 1: compute possible collisions and the exact time/position they occur
        //         treat acceleration as being constant during this step. quadratic root finding



        // step 2: integrate accelerations and velocities
        match self.integration_method {
            IntegrationMethod::Euler => {
                self.clear_accelerations_and_forces();
                self.calculate_gravitational_forces();

                for body in &mut self.present_state.objects {
                    body.acceleration = body.net_force / body.mass;
                    body.position += body.velocity * self.timestep; // position then velocity
                    body.velocity += body.acceleration * self.timestep;
                }
            }

            IntegrationMethod::SemiImplicitEuler => {
                self.clear_accelerations_and_forces();
                self.calculate_gravitational_forces();

                for body in &mut self.present_state.objects {
                    body.acceleration = body.net_force / body.mass;
                    body.velocity += body.acceleration * self.timestep; // velocity then position
                    body.position += body.velocity * self.timestep;
                }
            }

            IntegrationMethod::VelocityVerlet => {
                self.clear_accelerations_and_forces();
                self.calculate_gravitational_forces();

                // integrate velocities first
                for body in &mut self.present_state.objects {
                    let dt = self.timestep; // dT
                    let p = body.position; // p(T)
                    let v = body.velocity; // v(T)
                    let a = body.net_force / body.mass; // a(T)

                    body.position = p + (v * dt) + 0.5 * a * (dt * dt);
                    body.acceleration = a;
                }

                self.clear_forces();
                self.calculate_gravitational_forces(); // recalculate forces for new accelerations

                // integrate new accelerations sampled at the beginning and end of the timestep
                for body in &mut self.present_state.objects {
                    let dt = self.timestep; // dT
                    let v = body.velocity; // v(T)
                    let a = body.acceleration; // a(T) // we saved the accelerations we calculated initially here
                    let aa = body.net_force / body.mass; // a(T + dT) // we already changed p(T) to p(T + dT), so we have new p(T) accelerations

                    body.velocity = v + 0.5 * (a + aa) * dt;
                    body.acceleration = aa;
                }
            }
        }

        self.present_state.time += self.timestep;
        self.present_state.frame_number += 1;
        self.previous_state = self.present_state.clone();
    }
    
    fn test_termination_conditions(&self) -> bool {
        // return true if we are still running, false if a condition is met
        if self.termination_conditions.is_empty() {
            return true
        } else {
            for condition in self.termination_conditions.iter() {
                match condition {
                    TerminationCondition::ElapsedTime(t) => {
                        return self.present_state.time < *t
                    },
                    _ => {
                        return true
                    }
                }
            }
            unreachable!()
        }
    }

    // run the simulation for the desired time in seconds
    pub fn run(&mut self) {
        while self.test_termination_conditions() {
            self.step_simulation();
            if let Some(output) = &self.output_device {
                output.output(&self);
            }
        }
    }
}


