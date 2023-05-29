use system::SystemNodeRunner;

pub mod system;
pub mod tree;

fn main() {
    let r1 = SystemNodeRunner::new();
    let r2 = SystemNodeRunner::new_under(&r1);
    let r3 = SystemNodeRunner::new_under(&r1);
    let _r4 = SystemNodeRunner::new_under(&r1);
    let _r5 = SystemNodeRunner::new_under(&r3);
    let _r6 = SystemNodeRunner::new_under(&r3);
    let _r7 = SystemNodeRunner::new_under(&r2);

    loop {}
}
