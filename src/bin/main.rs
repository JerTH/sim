extern crate ssim;
use ssim::sim::{PhysicsCategory, Simulation, TerminationCondition};
use ssim::cli;
use ssim::math::{ DVec3, SVec3 };
use ssim::constants::*;
use ssim::output::OutputDevice;

fn main() {
    // add an option to print a trace to the console
    let mut sim = Simulation::new();

    let sol = sim.make_physics_body()
        .named("Sol")
        .with_physics_category(PhysicsCategory::Gravitational)
        .with_mass(SOL_MASS)
        .with_bounding_radius(SOL_RADIUS)
        .with_grav_param(SOL_GRAV_PARAM)
        .add();
    
    let _earth = sim.make_physics_body()
        .named("Earth")
        .with_transform(DVec3::new(EARTH_DIST_TO_SOL, 0.0, 0.0), None)
        .with_velocity(SVec3::new(0.0, EARTH_SOL_ORBIT_VEL, 0.0))
        .with_mass(EARTH_MASS)
        .with_bounding_radius(EARTH_RADIUS)
        .with_grav_param(EARTH_GRAV_PARAM)
        .relative_to(sol)
        .with_physics_category(PhysicsCategory::Gravitational)
        .add();

    //let _stone = sim.make_physics_body_from_template(STONE)
    //    .named("Stone")
    //    .with_transform(DVec3::new(0.0, EARTH_RADIUS + 200.0 * 1000.0, 0.0), None)
    //    .with_velocity(DVec3::new(-EARTH_LEO_ORBIT_VEL, 0.0, 0.0))
    //    .relative_to(earth)
    //    .add();

    let cli_matches = cli::parse_command_line();
    sim.set_termination_condition(TerminationCondition::ElapsedTime(60.0 * 60.0));
    sim.set_output_device(OutputDevice::from_cli_config(&sim, &cli_matches));
    sim.run();
}






