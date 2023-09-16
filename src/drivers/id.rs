// use spin::Mutex;

// use super::DriverId;

// static GEN: Mutex<IdGenerator> = Mutex::new(IdGenerator {current:0});
// struct IdGenerator {
//     current: usize,
// }
// //Returns a unique id for a driver
// pub fn get_id() -> DriverId {
//     get_ids(1)
// }
// //Returns the start unique id for a driver
// //DriverId..DriverId+amount are allocated for the driver
// pub fn get_ids(amount: usize) -> DriverId {
//     let prev = GEN.lock().current;
//     GEN.lock().current += amount;
//     prev
// }