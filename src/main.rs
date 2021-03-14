
/// Idea: "Event Tape", a timestamped linear tape of every event that happens in combat, with tags for filtering
/// Idea: "Adaptive Time Collision", starting with a basis timestep, test if two objects will pass through, reset
///        everything and then run the frame again with this halved timestep, repeat until collisions can be resolved

/// Some numbers:
/// Rg_muzzle: 37400m/s
/// Pd_muzzle: 1120m/s
/// T_accel: 196m/s2 
/// Earth_orbit: 148.56 million km

/// Coordinate system:
///  Looking at the solar system from the top, +/-Z is in/out, +/-X is left/right, +/-Y is up/down

/// Collision Algorithm:
///  1. Test continuous sphere-sphere bounding collisions as well as continuous sphere-ray bounding collisions
///  2. If a sphere collision is detected, roll back the frame and half the time-step, then re-run the frame
///  3. Repeat step 2 until no bounding collisions are detected
///  4. Re-run the frame with the newly discovered timestep using linear sweep OBB intersection tests
///  5. If no collision is detected, roll back, increment the timestep, and repeat step 4
///  6. Compare the number of collisions solved in step 4 to the number found in step 1 to set timestep for next frame

/// Collision Algorithm 2:
///  Using an objects bounding radius, and an assumption about the total possible volume it may move into in the next timestep
///  project a cone into space. Any cones which intersect indicate a possible collision, to be more accurately resolved

/// Collision Algorithm 3 (From Web):
///  1. World state is defined such that you can extrapolate perfectly how things would happen in absence of collisions or other actions
///  2. You predict all collisions and put them in a min-oriented priority queue based on ETA
///  3. At each frame you remove the first collision and see if it’s been invalidated (by storing the last processed
///     action’s timestamp in each object). If it’s still legit, you update the state for the colliding objects
///     (meaning you also put all their collisions with other objects in the queue). Repeat until you’ve caught up with the present.

use std::{collections::HashMap, hash::Hash };

#[derive(Debug, Clone, Copy)]
struct Vec3 {
    x: f64,
    y: f64,
    z: f64
}

impl std::ops::Mul<f64> for Vec3 {
    type Output = Self;
    fn mul(self, other: f64) -> Self {
        Self { x: self.x * other, y: self.y * other, z: self.z * other }
    }
}

impl std::ops::Mul<Vec3> for f64 {
    type Output = Vec3;
    fn mul(self, other: Vec3) -> Vec3 {
        Vec3 { x: self * other.x, y: self * other.y, z: self * other.z }
    }
}

impl std::ops::Div<f64> for Vec3 {
    type Output = Self;
    fn div(self, other: f64) -> Self {
        Self { x: self.x / other, y: self.y / other, z: self.z / other }
    }
}

impl std::ops::Sub for Vec3 {
    type Output = Vec3;
    fn sub(self, other: Self) -> Self {
        Vec3 { x: self.x - other.x, y: self.y - other.y, z: self.z - other.z }
    }
}

impl<'a, 'b> std::ops::Sub<&'b Vec3> for &'a Vec3 {
    type Output = Vec3;
    fn sub(self, other: &'b Vec3) -> Vec3 {
        Vec3 { x: self.x - other.x, y: self.y - other.y, z: self.z - other.z }
    }
}

impl std::ops::Add for Vec3 {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        Self { x: self.x + other.x, y: self.y + other.y, z: self.z + other.z }
    }
}

impl std::ops::AddAssign for Vec3 {
    fn add_assign(&mut self, other: Self) {
        *self = Self { x: self.x + other.x, y: self.y + other.y, z: self.z + other.z }
    }
}

impl std::convert::From<(f64, f64, f64)> for Vec3 {
    fn from(tuple: (f64, f64, f64)) -> Vec3 {
        Vec3 { x: tuple.0, y: tuple.1, z: tuple.2 }
    }
}

impl Vec3 {
    fn new(x: f64, y: f64, z: f64) -> Vec3 {
        Vec3 {
            x: x,
            y: y,
            z: z
        }
    }
    
    const fn zero() -> Vec3 {
        Vec3 { x: 0.0, y: 0.0, z: 0.0 }
    }

    pub fn length_to(&self, other: &Self) -> f64 {
        // ((x2 - x1)2 + (y2 - y1)2 + (z2 - z1)2)1/2
        ( (self.x - other.x) * (self.x - other.x)
        + (self.y - other.y) * (self.y - other.y)
        + (self.z - other.z) * (self.z - other.z)
        ).powf(0.5)
    }

    pub fn magnitude(&self) -> f64 {
        (self.x * self.x + self.y * self.y + self.z * self.z).sqrt()
    }

    pub fn normalize(&self) -> Self {
        let m = self.magnitude();
        Vec3 { x: self.x / m, y: self.y / m, z: self.z / m }
    }

    pub fn normal_vector_toward(&self, other: &Vec3) -> Vec3 {
        (other - self).normalize()
    }
}

#[derive(Debug, Clone, Copy)]
struct Quat {
    x: f32,
    y: f32,
    z: f32,
    w: f32
}

impl Quat {
    const fn zero() -> Quat {
        Quat { x: 0.0, y: 0.0, z: 0.0, w: 0.0 }
    }
}

#[derive(Debug, Clone)]
struct PhysicsBody {
    mass: f64, // kg
    bounding_radius: f64, // m
    position: Vec3, // m
    velocity: Vec3, // m/s
    acceleration: Vec3, // m/s^2
    net_force: Vec3, // N
    orientation: Quat, // quat
    angular_velocity: Vec3, // rad/s
    gravitational_parameter: f64, // m^3 * s^-2
    physics_category: PhysicsCategory, // todo: refactor this out into the simulation
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

    //fn k_energy(&self) -> f64 {
    //    let v = self.velocity.magnitude();
    //    (self.mass / 2.0) * (v * v)
    //}
    
    fn bounding_distance_to(&self, other: &PhysicsBody) -> f64 {
        self.position.length_to(&other.position) - self.bounding_radius - other.bounding_radius
    }
}

#[derive(Debug, Clone)]
struct PhysicsFrame {
    time: f64,
    named: HashMap<String, usize>,
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

    pub fn add_object(&mut self, obj: PhysicsBody, name: Option<&str>) -> i32 {
        let id = self.objects.len();
        self.objects.push(obj);
        
        if let Some(name) = name {
            self.named.insert(String::from(name), id);
        }
        return id as i32;
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
enum PhysicsCategory {
    Gravitational, // generally large bodies, affected by gravity, and also affect everything else with gravity
    Dynamic, // objects which are affected by gravity but do not affect other objects with gravity
}

struct PhysicsBodyBuilder<'a> {
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
    _relative_body_id: Option<i32>,
    _with_relative_rotation: bool,
}

impl<'a> PhysicsBodyBuilder<'a> {
    fn named(mut self, name: &str) -> Self {
        self._name = Some(String::from(name));
        self
    }

    fn with_transform(mut self, pos: Vec3, rot: Option<Vec3>) -> Self {
        self._pos = Some(pos);
        self._rot = rot;
        self
    }

    fn with_velocity(mut self, vel: Vec3) -> Self {
        self._vel = Some(vel);
        self
    }

    fn relative_to(mut self, id: i32) -> Self {
        self._relative_body_id = Some(id);
        self
    }

    /// Constructs then validates and adds a new PhysicsBody to the simulation
    fn add(self) -> i32 {
        let template = self._template.clone().unwrap_or(PhysicsBody::new());
        let mut body = template.clone();
        
        body.mass = self._mass.unwrap_or(template.mass);
        body.bounding_radius = self._radius.unwrap_or(template.bounding_radius);
        body.position = self._pos.unwrap_or(template.position);
        body.velocity = self._vel.unwrap_or(template.velocity);

        // add the relative bodies transform and velocity
        if let Some(id) = self._relative_body_id {
            if let Some(relative_body) = self.__sim.get_body_from_id(id) {
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
enum IntegrationMethod {
    Euler,
    SemiImplicitEuler,
    VelocityVerlet,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum OutputDirective {
    ConsoleOut,
    Frequency(f64),
    SystemEnergy,
    FrameNumber,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum TerminationCondition {
    ElapsedTime(f64),
}

struct Simulation {
    initial_state: PhysicsFrame,
    present_state: PhysicsFrame,
    previous_state: PhysicsFrame,
    timestep: f64, // seconds
    integration_method: IntegrationMethod,
    termination_conditions: Vec<TerminationCondition>, 
    output_directives: Vec<OutputDirective>,
}

fn format_si_value(n: f64) -> String {
    match n {
        x if x.abs() < 0.0000000001 => return format!("{}n", x * 1000000000.0),
        x if (0.0000001..0.0000000001).contains(&x.abs()) => return format!("{}n", x * 1000000000.0),
        x if (0.0001..0.0000001).contains(&x.abs()) => return format!("{}u", x * 1000000.0),
        x if (0.0..0.0001).contains(&x.abs()) => return format!("{}m", x * 1000.0),
        x if (0.0..1000.0).contains(&x.abs()) => return format!("{}", x),
        x if (1000.0..1000000.0).contains(&x.abs()) => return format!("{}K", x / 1000.0),
        x if (1000000.0..1000000000.0).contains(&x.abs()) => return format!("{}M", x / 1000000.0),
        x if (1000000000.0..1000000000000.0).contains(&x.abs()) => return format!("{}G", x / 1000000000.0),
        x if (1000000000000.0..1000000000000000.0).contains(&x.abs()) => return format!("{}T", x / 1000000000000.0),
        x if (1000000000000000.0..1000000000000000000.0).contains(&x.abs()) => return format!("{}P", x / 1000000000000000.0),
        x if (1000000000000000000.0..1000000000000000000000.0).contains(&x.abs()) => return format!("{}E", x / 1000000000000000000.0),
        x if x.abs() > 1000000000000000000000.0 => return format!("{}E", x / 1000000000000000000.0),
        x => return format!("{}", x)
    }
}

impl Simulation {
    fn new() -> Simulation {
        Simulation {
            initial_state: PhysicsFrame::new(0.0),
            present_state: PhysicsFrame::new(0.0),
            previous_state: PhysicsFrame::new(0.0),
            timestep: 1.0f64,
            integration_method: IntegrationMethod::VelocityVerlet,
            termination_conditions: Vec::new(),
            output_directives: Vec::new(),
        }
    }

    fn get_body_from_id(&self, id: i32) -> Option<&PhysicsBody> {
        self.present_state.objects.get(id as usize)
    }

    fn get_named_body(&self, name: &str) -> Option<&PhysicsBody> {
        self.present_state.named
            .get(&String::from(name))
            .map(|id| &self.present_state.objects[*id])
    }

    fn set_global_timestep(&mut self, t: f64) {
        self.timestep = t;
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
    
    pub fn output_console_data(&self) {
        let mut output = Vec::new();
        for directive in self.output_directives.iter() {
            match directive {
                OutputDirective::FrameNumber => {
                    output.push(format!("{:<10}", self.present_state.frame_number));
                },
                OutputDirective::SystemEnergy => {
                    let e = (self.system_kinetic_energy() + self.system_potential_energy()) / self.system_total_mass();
                    output.push(format!("System Energy: {:.10}J/kg", format_si_value(e)));
                },
                _ => {
                    continue; // do nothing
                }
            }
        }
        output.push(String::from("\n"));

        for item in output {
            print!("{}", item);
        }
    }

    pub fn output_data(&mut self) {
        for directive in self.output_directives.iter() {
            match directive {
                OutputDirective::Frequency(f) => {
                    if self.present_state.time % f > (self.timestep / 2.0) {
                        return; // short circuit
                    }
                },
                _ => {
                    continue
                }
            }
        }

        for directive in self.output_directives.iter() {
            match directive {
                OutputDirective::ConsoleOut => {
                    self.output_console_data();
                },
                _ => {
                    continue; // do nothing
                }
            }
        }
        
        // display some numbers n times
        //let n = 10.0;
        //let t = self.present_state.time;
        //if (t % (run_time / n)) < self.timestep {
        //    let earth = self.get_named_body("Earth");
        //    let stone = self.get_named_body("Stone");
        //    
        //    if let (Some(earth), Some(stone)) = (earth, stone) {
        //        const G: f64 = 0.0000000000667408;
        //        let e = self.present_state.time / 60.0 / 60.0 / 24.0;
        //        let d = stone.bounding_distance_to(earth) / 1000.0;
        //        let v = (stone.velocity - earth.velocity).magnitude();
        //        let k = (stone.mass / 2.0) * (v * v);
        //        let p = -(G * earth.mass * stone.mass) / earth.position.length_to(&stone.position);
        //        let t = k + p;
        //        println!("{:<7.2} alt: {:8.1}km   V: {:8.2}m/s   KE: {:8.2}MJ   PE: {:8.2}MJ   TE: {:14.4}J", e, d, v, k / 1000000.0, p / 1000000.0, t);
        //    }                
        //}
    }

    pub fn set_output_directive(&mut self, directive: OutputDirective) {
        self.output_directives.push(directive); // TODO: This is dumb, test for duplicate directives/sanity check what we're outputting
    }

    pub fn set_termination_condition(&mut self, condition: TerminationCondition) {
        self.termination_conditions.push(condition); // TODO: This is dumb too, test for duplicates and sanity check 
    }

    pub fn step_simulation(&mut self) {
        // integrate
        match self.integration_method {
            IntegrationMethod::Euler => {
                self.clear_accelerations_and_forces();
                self.calculate_gravitational_forces();

                // integrate velocities and accelerations
                for body in &mut self.present_state.objects {
                    body.acceleration = body.net_force / body.mass;
                    body.position += body.velocity * self.timestep; // position then velocity
                    body.velocity += body.acceleration * self.timestep;
                }
            }

            IntegrationMethod::SemiImplicitEuler => {
                self.clear_accelerations_and_forces();
                self.calculate_gravitational_forces();

                // integrate velocities and accelerations
                for body in &mut self.present_state.objects {
                    body.acceleration = body.net_force / body.mass;
                    body.velocity += body.acceleration * self.timestep; // velocity then position
                    body.position += body.velocity * self.timestep;
                }
            }

            IntegrationMethod::VelocityVerlet => {
                self.clear_accelerations_and_forces();
                self.calculate_gravitational_forces();

                // integrate velocities
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

                // integrate new accelerations
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
            self.output_data();
            self.step_simulation();
        }
    }
}

#[derive(Debug, Clone, Default)]
struct ParsedArguments {
    timestep: Option<f64>,
}

fn parse_command_arguments() -> ParsedArguments {
    let mut result = ParsedArguments::default();

    for arg in std::env::args() {
        let sub_arg: Vec<&str> = arg.split("=").collect();
        if sub_arg.len() == 2 {
            match sub_arg[0] {
                "dt" => {
                    let v = sub_arg[1].parse();
                    result.timestep = v.ok();
                }
                _ => {
                    continue;
                }
            }
        }
    }

    unimplemented!("clean up the above, remove below directive");
    #[allow(unreachable_code)]
    result
}

fn main() {
    let mut sim = Simulation::new();    
    let day = 60.0 * 60.0 * 24.0;
    sim.set_termination_condition(TerminationCondition::ElapsedTime(day * 365.0 * 5.0));
    sim.set_output_directive(OutputDirective::ConsoleOut);
    sim.set_output_directive(OutputDirective::FrameNumber);
    sim.set_output_directive(OutputDirective::Frequency(day * 365.0 / 4.0));
    sim.set_output_directive(OutputDirective::SystemEnergy);
    sim.set_global_timestep(60.0);

    let sol = sim.make_physics_body_from_template(SOL)
        .named("Sol")
        .add();

    let earth = sim.make_physics_body_from_template(EARTH)
        .named("Earth")
        .with_transform(Vec3::new(EARTH_DIST_TO_SOL, 0.0, 0.0), None)
        .with_velocity(Vec3::new(0.0, EARTH_SOL_ORBIT_VEL, 0.0))
        .relative_to(sol)
        .add();

    let _stone = sim.make_physics_body_from_template(STONE)
        .named("Stone")
        .with_transform(Vec3::new(0.0, EARTH_RADIUS + 200.0 * 1000.0, 0.0), None)
        .with_velocity(Vec3::new(-EARTH_LEO_ORBIT_VEL, 0.0, 0.0))
        .relative_to(earth)
        .add();

    sim.run();
}

const EARTH: PhysicsBody = PhysicsBody {
    bounding_radius: EARTH_RADIUS,
    position: Vec3::zero(),
    velocity: Vec3::zero(),
    acceleration: Vec3::zero(),
    net_force: Vec3::zero(),
    orientation: Quat::zero(),
    angular_velocity: Vec3::zero(),
    mass: EARTH_MASS,
    gravitational_parameter: EARTH_GRAV_PARAM,
    physics_category: PhysicsCategory::Gravitational,
};

const STONE: PhysicsBody = PhysicsBody {
    bounding_radius: 0.2,
    position: Vec3::zero(),
    velocity: Vec3::zero(),
    acceleration: Vec3::zero(),
    net_force: Vec3::zero(),
    orientation: Quat::zero(),
    angular_velocity: Vec3::zero(),
    mass: 1.7,
    gravitational_parameter: 0.0,
    physics_category: PhysicsCategory::Gravitational,
};

const SOL: PhysicsBody = PhysicsBody {
    bounding_radius: SOL_RADIUS,
    position: Vec3::zero(),
    velocity: Vec3::zero(),
    acceleration: Vec3::zero(),
    net_force: Vec3::zero(),
    orientation: Quat::zero(),
    angular_velocity: Vec3::zero(),
    mass: SOL_MASS,
    gravitational_parameter: SOL_GRAV_PARAM,
    physics_category: PhysicsCategory::Gravitational,
};

// some useful constants
const SOL_GRAV_PARAM: f64 = 132712440018000000000.0;
const SOL_MASS: f64 = 1989000000000000000000000000000.0; // kg
const SOL_RADIUS: f64 = 696340000.0; // m

const EARTH_GRAV_PARAM: f64 = 398600441800000.0;
const EARTH_MASS: f64 = 5972000000000000000000000.0; // kg
const EARTH_RADIUS: f64 = 6371000.0;
const EARTH_DIST_TO_SOL: f64 = 149600000000.0; // m
const EARTH_SOL_ORBIT_VEL: f64 = 29780.0;
const EARTH_LEO_ORBIT_VEL: f64 = 7788.25;

const G: f64 = 0.0000000000667408;

// Diary
// 
// Mar 14th
// 1. [ ] Clean up at least 2 code paths that are unimplemented or could use additional checks/fix naked unwraps/expects
//         1.1 get_body_from_id, get_named_body
//         1.2 
// 2. [ ] Start refactoring the main simulation loop into more manageable bits
// 3. [x] Add simple command line argument parsing
// 4. [ ] Improve data display, add command line option to track a named body

