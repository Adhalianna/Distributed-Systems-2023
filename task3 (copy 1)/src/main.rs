pub mod tree;
pub use tree::TreeNode;
pub mod system;
use system::SystemNodeRunner;

fn main() {
    let (tree, t0) = SystemNodeRunner::new();
    let (ch1, t1) = SystemNodeRunner::new_under(&tree);
    let (ch2, t2) = SystemNodeRunner::new_under(&tree);
    let (ch3, t3) = SystemNodeRunner::new_under(&ch1);
    let (ch4, t4) = SystemNodeRunner::new_under(&ch3);
    let (ch5, t5) = SystemNodeRunner::new_under(&ch3);

    tree.start();
    ch1.start();
    ch2.start();
    ch3.start();
    ch4.start();
    ch5.start();

    while !([&t0, &t1, &t2, &t3, &t4, &t5]
        .iter()
        .map(|t| t.is_finished())
        .any(|b| b))
    {}
}
