fn main() -> sled::Result<()> {
    let db = sled::open("./db")?;

    for tree_name in db.tree_names() {
        println!("{}", String::from_utf8_lossy(&tree_name));
    }

    Ok(())
}
