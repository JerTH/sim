
pub mod math;
pub mod sim;
pub mod output;
pub mod cli;
pub mod constants;

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////
//
// Diary
// 
// Mar 14th
// 1. [x] Clean up some code paths that are unimplemented or could use additional checks/fix naked unwraps/expects
// 2. [x] Start refactoring the main simulation loop into more manageable bits
// 3. [x] Add simple command line argument parsing
// 4. [x] Improve data display
// 5. [x] Add command line option to track a named body
// 
// Mar 15th
// 1. [x] Refactor data output into its own state-holding struct
// 2. [x] Refactor and improve CLI parsing using the clap crate
// 
// Mar 18th
// 1. [x] Handle named objects better. Can more than one object have the same name? Probably
// 2. [x] Improve OutputDevice console output
// 3. [x] Refactor project files
// 
// Mar 19th
// 1. [ ] Improve physics body construction with more uniform interface, less clutter
// 
// 
// 
///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////
// 
// Notes
// 
// Idea: "Event Tape", a timestamped linear tape of every event that happens in combat, with tags for filtering
// Idea: "Adaptive Time Collision", starting with a basis timestep, test if two objects will pass through, reset
//        everything and then run the frame again with this halved timestep, repeat until collisions can be resolved
//
// Some numbers:
// Rg_muzzle: 37400m/s
// Pd_muzzle: 1120m/s
// T_accel: 196m/s2 
// Earth_orbit: 148.56 million km
//
// Coordinate system:
//  Looking at the solar system from the top, +/-Z is in/out, +/-X is left/right, +/-Y is up/down
//
// Collision Algorithm:
//  1. Test continuous sphere-sphere bounding collisions as well as continuous sphere-ray bounding collisions
//  2. If a sphere collision is detected, roll back the frame and half the time-step, then re-run the frame
//  3. Repeat step 2 until no bounding collisions are detected
//  4. Re-run the frame with the newly discovered timestep using linear sweep OBB intersection tests
//  5. If no collision is detected, roll back, increment the timestep, and repeat step 4
//  6. Compare the number of collisions solved in step 4 to the number found in step 1 to set timestep for next frame
//
// Collision Algorithm 2:
//  Using an objects bounding radius, and an assumption about the total possible volume it may move into in the next timestep
//  project a cone into space. Any cones which intersect indicate a possible collision, to be more accurately resolved
//
// Collision Algorithm 3 (From Web):
//  1. World state is defined such that you can extrapolate perfectly how things would happen in absence of collisions or other actions
//  2. You predict all collisions and put them in a min-oriented priority queue based on ETA
//  3. At each frame you remove the first collision and see if it’s been invalidated (by storing the last processed
//     action’s timestamp in each object). If it’s still legit, you update the state for the colliding objects
//     (meaning you also put all their collisions with other objects in the queue). Repeat until you’ve caught up with the present.

