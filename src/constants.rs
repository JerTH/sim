use crate::{ math::*, sim::* };

pub const SOL_GRAV_PARAM: f64 = 132712440018000000000.0;
pub const SOL_MASS: f64 = 1989000000000000000000000000000.0; // kg
pub const SOL_RADIUS: f64 = 696340000.0; // m

pub const EARTH_GRAV_PARAM: f64 = 398600441800000.0;
pub const EARTH_MASS: f64 = 5972000000000000000000000.0; // kg
pub const EARTH_RADIUS: f64 = 6371000.0;
pub const EARTH_DIST_TO_SOL: f64 = 149600000000.0; // m
pub const EARTH_SOL_ORBIT_VEL: f64 = 29780.0;
pub const EARTH_LEO_ORBIT_VEL: f64 = 7788.25;

pub const G: f64 = 0.0000000000667408;

pub const EARTH: PhysicsBody = PhysicsBody {
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

pub const STONE: PhysicsBody = PhysicsBody {
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

pub const SOL: PhysicsBody = PhysicsBody {
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