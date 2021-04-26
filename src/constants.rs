pub const SOL_GRAV_PARAM: f64 = 132712440018000000000.0;
pub const SOL_MASS: f64 = 1989000000000000000000000000000.0; // kg
pub const SOL_RADIUS: f32 = 696340000.0; // m

pub const EARTH_GRAV_PARAM: f64 = 398600441800000.0;
pub const EARTH_MASS: f64 = 5972000000000000000000000.0; // kg
pub const EARTH_RADIUS: f32 = 6371000.0;
pub const EARTH_DIST_TO_SOL: f64 = 149600000000.0; // m
pub const EARTH_SOL_ORBIT_VEL: f32 = 29780.0;
pub const EARTH_LEO_ORBIT_VEL: f32 = 7788.25;

pub const G: f64 = 0.0000000000667408;

//pub const EARTH: PhysicsBodyRef = PhysicsBodyRef {
//    bounding_radius: EARTH_RADIUS,
//    position: DVec3::zero(),
//    velocity: DVec3::zero(),
//    acceleration: DVec3::zero(),
//    net_force: DVec3::zero(),
//    orientation: Quat::zero(),
//    angular_velocity: DVec3::zero(),
//    mass: EARTH_MASS,
//    gravitational_parameter: EARTH_GRAV_PARAM,
//    physics_category: PhysicsCategory::Gravitational,
//};
//
//pub const STONE: PhysicsBodyRef = PhysicsBodyRef {
//    bounding_radius: 0.2,
//    position: DVec3::zero(),
//    velocity: DVec3::zero(),
//    acceleration: DVec3::zero(),
//    net_force: DVec3::zero(),
//    orientation: Quat::zero(),
//    angular_velocity: DVec3::zero(),
//    mass: 1.7,
//    gravitational_parameter: 0.0,
//    physics_category: PhysicsCategory::Gravitational,
//};
//
//pub const SOL: PhysicsBodyRef = PhysicsBodyRef {
//    bounding_radius: SOL_RADIUS,
//    position: DVec3::zero(),
//    velocity: DVec3::zero(),
//    acceleration: DVec3::zero(),
//    net_force: DVec3::zero(),
//    orientation: Quat::zero(),
//    angular_velocity: DVec3::zero(),
//    mass: SOL_MASS,
//    gravitational_parameter: SOL_GRAV_PARAM,
//    physics_category: PhysicsCategory::Gravitational,
//};
