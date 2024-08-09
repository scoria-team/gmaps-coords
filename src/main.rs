//! Consumes a CSV or GeoJSON containing Google Maps URLs, finds the coordinates
//! of each place, and outputs a GeoJSON file with the coordinate data.

#[tokio::main]
async fn main() {
    gmaps_coords::run().await;
}
