use std::error::Error;

use migratour::cmd_run;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    cmd_run().await?;

    Ok(())
}
