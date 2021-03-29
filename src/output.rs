use std::ops::{ AddAssign, SubAssign };
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
    MemoryUse,
}

#[derive(Debug, Clone)]
enum OutputFrequency {
    EveryFrame,
    Hertz(f64),
    Frames(f64),
    SimTime(f64),
}

impl Default for OutputFrequency {
    fn default() -> Self {
        OutputFrequency::EveryFrame
    }
}

#[derive(Debug, Clone, Default)]
pub struct OutputDevice {
    target: OutputTarget,
    format: OutputFormat,
    tracked_bodies: Vec<(String, usize, Vec<OutputField>)>,
    global_fields: Vec<OutputField>,
    frequency: OutputFrequency,
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
                if matches.is_present("memoryuse") { device.global_fields.push(OutputField::MemoryUse); }

                if let Some(matches) = matches.subcommand_matches("track") {
                    let mut tracked_fields = Vec::new();

                    if matches.is_present("kineticenergy") { tracked_fields.push(OutputField::KineticEnergy); }
                    if matches.is_present("position") { tracked_fields.push(OutputField::Position); }
                    if matches.is_present("velocity") { tracked_fields.push(OutputField::Velocity); }
                    if matches.is_present("acceleration") { tracked_fields.push(OutputField::Acceleration); }

                    if let Some(targets) = matches.values_of("target") {
                        for target in targets {
                            for body in sim.present().get_named_bodies(target) {
                                println!("{:#?}", body);
                                device.tracked_bodies.push((target.to_ascii_uppercase(), body.id(), tracked_fields.clone()));
                            }
                        }
                    }
                }
            }
        }
        device
    }
    
    pub fn output(&self, sim: &Simulation) {
        match self.frequency {
            OutputFrequency::EveryFrame => {
                // do nothing
            },
            OutputFrequency::Hertz(f) => {
                unimplemented!();
            },
            OutputFrequency::Frames(f) => {
                if sim.present().frame_number() % f as usize > 0 {
                    return
                }
            },
            OutputFrequency::SimTime(f) => {
                if sim.present().sim_time() % f >= (sim.present().time_step()) {
                    return
                }
            }
        }

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
                    println!("{}Frame: {}", indent_str.repeat(indent), sim.present().frame_number());
                },
                OutputField::MemoryUse => {
                    let (m, mp) = format_mem_value(sim.memory_use());
                    println!("{}Memory Use: {:.04}{}B", indent_str.repeat(indent), m, mp);
                },
                _ => {
                    continue; // unhandled/not applicable field type
                }
            }
        }

        println!("{}Tracked Bodies:", indent_str.repeat(indent));
        indent.add_assign(3);
        for (name, body_id, fields) in self.tracked_bodies.iter() {
            print!("{}{}", indent_str.repeat(indent), name);
            
            if let Some(body) = sim.present().get_body_ref(*body_id) {
                println!(" ({:?})", body.physics_category());
                
                if fields.is_empty() {
                    println!("No Fields Tracked");
                } else {
                    indent.add_assign(1);
                    for field in fields.iter() {
                        let i = indent_str.repeat(indent);
                        match field {
                            OutputField::KineticEnergy => {
                                let (e, ep) = format_si_value(body.kinetic_energy() / body.mass());
                                println!("{}KIN={:+09.04}{}J/kg", i, e, ep);
                            },
                            OutputField::Position => {
                                let (x, xp) = format_si_value(body.position().x);
                                let (y, yp) = format_si_value(body.position().y);
                                let (z, zp) = format_si_value(body.position().z);
                                println!("{}POS={:+09.04}{}m, {:+09.04}{}m, {:+09.04}{}m", i, x, xp, y, yp, z, zp);
                            },
                            OutputField::Velocity => {
                                let (x, xp) = format_si_value(body.velocity().x as f64);
                                let (y, yp) = format_si_value(body.velocity().y as f64);
                                let (z, zp) = format_si_value(body.velocity().z as f64);
                                println!("{}VEL={:+09.04}{}m/s, {:+09.04}{}m/s, {:+09.04}{}m/s", i, x, xp, y, yp, z, zp);
                            },
                            OutputField::Acceleration => {
                                let (x, xp) = format_si_value(body.acceleration().x as f64);
                                let (y, yp) = format_si_value(body.acceleration().y as f64);
                                let (z, zp) = format_si_value(body.acceleration().z as f64);
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

fn format_mem_value(n: usize) -> (f64, &'static str) {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;
    const TB: f64 = GB * 1024.0;

    if n == 0 {
        return (0.0, "")
    } else {
        match n as f64 {
            x if (0.0..KB).contains(&x) => return (x, ""),
            x if (KB..MB).contains(&x) => return (x / KB, "Ki"),
            x if (MB..GB).contains(&x) => return (x / MB, "Mi"),
            x if (GB..TB).contains(&x) => return (x / GB, "Gi"),
            x => return (x, "")
        }
    }
}

impl MemUse for OutputDevice {
    fn memory_use(&self) -> usize {
        let mut total = 0;
        total += ::std::mem::size_of_val(&self.target);
        total += ::std::mem::size_of_val(&self.format);
        total += ::std::mem::size_of_val(&self.tracked_bodies);
        total += ::std::mem::size_of_val(&self.global_fields);
        total
    }
}

pub trait MemUse: Sized {
    fn memory_use(&self) -> usize {
        ::std::mem::size_of::<Self>()
    }
}
