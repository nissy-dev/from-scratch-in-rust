mod btree;

fn main() {
    let tree = btree::BTree::new();
    println!("BTree: {:?}", tree);
}
