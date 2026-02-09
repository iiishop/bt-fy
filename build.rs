fn main() {
    embuild::espidf::sysenv::output();

    // Include HTML file
    println!("cargo:rerun-if-changed=src/web/welcome.html");
}
