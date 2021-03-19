use std::ops::{ Add, AddAssign, Deref, SubAssign };
use clap::ArgMatches;
use crate::sim::*;

#[derive(Debug, Clone)]
enum OutputTarget {
    Console,
    File,
}

impl Default for OutputTarget { fn default() -> Self { Self::Console } }

#[derive(Debug, Clone)]
enum OutputFormat {
    Pretty,
    Csv,
}

impl Default for OutputFormat { fn default() -> Self { Self::Pretty } }

#[derive(Debug, Clone)]
/// Represnets a specific field to include in the output data
enum OutputField {
    Frames,
    TotalEnergy,
    KineticEnergy,
    PotentialEnergy,
    Position,
    Velocity,
    Acceleration,
    Time,
}

#[derive(Debug, Clone, Default)]
pub struct OutputDevice {
    target: OutputTarget,
    format: OutputFormat,
    tracked_bodies: Vec<(String, usize, Vec<OutputField>)>,
    global_fields: Vec<OutputField>,
}

// system energy in J/kg = (system_kinetic_energy + system_potential_energy) / system_total_mass

impl OutputDevice {

    // TODO: ISOLATE CLI STUFF TO CLI.RS
    pub fn from_cli_config(sim: &Simulation, matches: &clap::ArgMatches) -> OutputDevice {
        let mut device = OutputDevice::default();
        if let Some(matches) = matches.subcommand_matches("simparams") {
            if let Some(matches) = matches.subcommand_matches("output") {
                device.target = matches.value_of("target").map(|t| match t.to_ascii_uppercase().as_str() {
                    "CONSOLE" => OutputTarget::Console,
                    "FILE" => OutputTarget::File,
                    _ => OutputTarget::Console,
                }).unwrap_or(OutputTarget::default());

                device.format = matches.value_of("format").map(|f| match f.to_ascii_uppercase().as_str() {
                    "PRETTY" => OutputFormat::Pretty,
                    "CSV" => OutputFormat::Csv,
                    _ => OutputFormat::Pretty,
                }).unwrap_or(OutputFormat::default());

                if matches.is_present("totalenergy") { device.global_fields.push(OutputField::TotalEnergy); }
                if matches.is_present("kineticenergy") { device.global_fields.push(OutputField::KineticEnergy); }
                if matches.is_present("potentialenergy") { device.global_fields.push(OutputField::PotentialEnergy); }
                if matches.is_present("frames") { device.global_fields.push(OutputField::Frames); }
                if matches.is_present("time") { device.global_fields.push(OutputField::Time); }

                if let Some(matches) = matches.subcommand_matches("track") {
                    let mut tracked_fields = Vec::new();

                    if matches.is_present("kineticenergy") { tracked_fields.push(OutputField::KineticEnergy); }
                    if matches.is_present("position") { tracked_fields.push(OutputField::Position); }
                    if matches.is_present("velocity") { tracked_fields.push(OutputField::Velocity); }
                    if matches.is_present("acceleration") { tracked_fields.push(OutputField::Acceleration); }

                    if let Some(targets) = matches.values_of("target") {
                        for target in targets {
                            for body in sim.bodies_with_name(target) {
                                device.tracked_bodies.push((target.to_ascii_uppercase(), body.0, tracked_fields.clone()));
                            }
                        }
                    }
                }
            }
        }
        device
    }

    pub fn output(&self, sim: &Simulation) {
        println!("------------------------------------------");
        let mut indent = 0;
        let indent_str = "  ";
        for (i, field) in self.global_fields.iter().enumerate() {
            match field {
                OutputField::TotalEnergy => {
                    let (e, ep) = format_si_value((sim.system_kinetic_energy() + sim.system_potential_energy()) / sim.system_total_mass());
                    println!("{}System Total Energy: {:09.04}{}J/Kg", indent_str.repeat(indent), e, ep);
                },
                OutputField::Frames => {
                    println!("{}Frame: {}", indent_str.repeat(indent), sim.present_frame().frame_number());
                },
                _ => {
                    continue; // unhandled/not applicable field type
                }
            }
        }

        println!("{}Tracked Bodies:", indent_str.repeat(indent));
        indent.add_assign(3);
        for (i, tracked_body) in self.tracked_bodies.iter().enumerate() {
            println!("{}{}", indent_str.repeat(indent), tracked_body.0);
            
            if let Some(body) = sim.body_from_id(tracked_body.1) {
                if tracked_body.2.is_empty() {
                    println!("No Fields Tracked");
                } else {
                    indent.add_assign(1);
                    for field in tracked_body.2.iter() {
                        let i = indent_str.repeat(indent);
                        match field {
                            OutputField::KineticEnergy => {
                                let (e, ep) = format_si_value(body.kinetic_energy() / body.mass);
                                println!("{}KIN={:+09.04}{}J/kg", i, e, ep);
                            },
                            OutputField::Position => {
                                let (x, xp) = format_si_value(body.position.x);
                                let (y, yp) = format_si_value(body.position.y);
                                let (z, zp) = format_si_value(body.position.z);
                                println!("{}POS={:+09.04}{}m, {:+09.04}{}m, {:+09.04}{}m", i, x, xp, y, yp, z, zp);
                            },
                            OutputField::Velocity => {
                                let (x, xp) = format_si_value(body.velocity.x);
                                let (y, yp) = format_si_value(body.velocity.y);
                                let (z, zp) = format_si_value(body.velocity.z);
                                println!("{}VEL={:+09.04}{}m/s, {:+09.04}{}m/s, {:+09.04}{}m/s", i, x, xp, y, yp, z, zp);
                            },
                            OutputField::Acceleration => {
                                let (x, xp) = format_si_value(body.acceleration.x);
                                let (y, yp) = format_si_value(body.acceleration.y);
                                let (z, zp) = format_si_value(body.acceleration.z);
                                println!("{}ACC={:+09.04}{}m/s^2, {:+09.04}{}m/s^2, {:+09.04}{}m/s^2", i, x, xp, y, yp, z, zp);
                            },
                            _ => {
                                continue; // unhandled/not applicable field type
                            }
                        }
                    }
                    indent.sub_assign(1);
                }
            }
        }
        indent.sub_assign(3);
        println!();
    }
}

fn format_si_value(n: f64) -> (f64, &'static str) {
    if n == 0.0 {
        return (0.0, "")
    } else if n.is_nan() {
        return (f64::NAN, "")
    } else if n.is_infinite() {
        return (f64::INFINITY, "")
    } else {
        match n {
            x if x.abs() < 0.0000000001 => return (x * 1000000000.0, "n"),
            x if (0.0000001..0.0000000001).contains(&x.abs()) => return (x * 1000000000.0, "n"),
            x if (0.0001..0.0000001).contains(&x.abs()) => return (x * 1000000.0, "u"),
            x if (0.0..0.0001).contains(&x.abs()) => return (x * 1000.0, "m"),
            x if (0.0..1000.0).contains(&x.abs()) => return (x, ""),
            x if (1000.0..1000000.0).contains(&x.abs()) => return (x / 1000.0, "K"),
            x if (1000000.0..1000000000.0).contains(&x.abs()) => return (x / 1000000.0, "M"),
            x if (1000000000.0..1000000000000.0).contains(&x.abs()) => return (x / 1000000000.0, "G"),
            x if (1000000000000.0..1000000000000000.0).contains(&x.abs()) => return (x / 1000000000000.0, "T"),
            x if (1000000000000000.0..1000000000000000000.0).contains(&x.abs()) => return (x / 1000000000000000.0, "P"),
            x if (1000000000000000000.0..1000000000000000000000.0).contains(&x.abs()) => return (x / 1000000000000000000.0, "E"),
            x if x.abs() > 1000000000000000000000.0 => return (x / 1000000000000000000.0, "E"),
            x => return (x, "")
        }
    }
}