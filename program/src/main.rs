use hostbuilder::{
    HostedObject,
    HostedObjectTrait
};
use anyhow::Result;
use std::time::Instant;

#[tokio::main]
async fn main() -> Result<()> {
    let start_time = Instant::now();
    let mut engine = HostedObject::new().await?;
    engine.run().await?;
    let duration = start_time.elapsed();
    println!("Startup time: {:.2?} seconds", duration);
    Ok(())
}