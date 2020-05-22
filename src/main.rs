fn main() {
    println!("Hello, world!");

    // 1. scan fs to determine local state of the world
    // 2. download metadata from the GP API to determine remote state of the world
    // 3. diff 1. and 2. to determine what to download
    // 4. start downloading
    // 5. respect API limits
    // 6. do this once every X hours
}
