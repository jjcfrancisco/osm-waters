use geo::{*};
use shapefile;
use shapefile::PolygonRing::{Outer, Inner};
use postgres::{Client, NoTls};
use std::env;
use wkt;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;

fn to_geo_poly(polygon: shapefile::Polygon) -> geo::Polygon {

    let mut x: f64;
    let mut y: f64;
    let mut outer_placeholder: Vec<(f64,f64)> = Vec::new();
    let mut inner_placeholder: Vec<geo::LineString> = Vec::new();

    for ring_type in polygon.rings() {
        match ring_type {
            Outer(o) => {
                //Gather all outer rings
                for point in o {
                    x = point.x;
                    y = point.y;
                    outer_placeholder.push((x,y));
                }
            },
            Inner(i) => {
                //Gather all inners
                let mut single_inner_placeholder: Vec<(f64,f64)> = Vec::new();
                for point in i {
                    x = point.x;
                    y = point.y;
                    single_inner_placeholder.push((x,y));
                }
                let ls = geo::LineString::from(single_inner_placeholder);
                inner_placeholder.push(ls);
            },
        }
    }
    
    let ext_ring = geo::LineString::from(outer_placeholder);
    if inner_placeholder.is_empty() {
        geo::Polygon::new(ext_ring, vec![])
    } else {
        geo::Polygon::new(ext_ring, inner_placeholder)
    }

}

#[derive(Debug)]
struct Feature {
    name: String,
    geom: geo::Polygon,
}

fn postgis_data(query: String) -> Vec<Feature> {
    
    let pgcon = env::var("PGCON").expect("$PGCON is not set");
    let mut client = Client::connect(&pgcon, NoTls).unwrap();
    let mut features: Vec<Feature> = Vec::new();
    for row in &client.query(&query, &[]).unwrap() {
        let wkt_geom: String = row.get("geom");
        let result =  wkt::TryFromWkt::try_from_wkt_str(&wkt_geom);
        if result.is_ok() {
            let geom: geo::Polygon = result.unwrap();
            features.push(Feature{
                name: row.get("project_name"),
                geom,
            });
        }
    }
    features
}

// Goes over interesects
fn intersects(polys:Vec<geo::Polygon>, targets:Vec<Feature>) -> Vec<geo::Polygon> {

    let mut intersects:Vec<geo::Polygon> = Vec::new();
    for poly in polys.iter() {
        for target in targets.iter() {
            if poly.intersects(&target.geom) {
               intersects.push(poly.to_owned()); 
            }
        }
    }
    intersects

}

// Reads file
fn read_file(filepath: &str) -> String {

    let path = Path::new(filepath);
    let mut file = match File::open(&path) {
        Err(why) => panic!("couldn't open: {}", why),
        Ok(file) => file,
    };

    let mut s = String::new();
    match file.read_to_string(&mut s) {
        Err(why) => panic!("couldn't read: {}", why),
        Ok(_) => s,
    }
}

// Read shapefile
fn read_shapefile(filepath: &str) -> Vec<geo::Polygon> {

    let mut polys:Vec<geo::Polygon> = Vec::new();
    let reader = shapefile::Reader::from_path(filepath);
    if reader.is_ok() {
        let mut content = reader.unwrap();
        let shapes = content.iter_shapes_and_records_as::<shapefile::Polygon, shapefile::dbase::Record>();
        for shape in shapes {
            if shape.is_ok() {
                // Polygon shape only, record ignored
                let (polygon, _) = shape.unwrap();
                let poly = to_geo_poly(polygon);
                polys.push(poly); 
            }
        }
    }
    polys

}

fn main() {

    let query = read_file("src/query.sql");
    let regions = postgis_data(query);
    let filepath = "/Users/frankjimenez/tests/water/shp/water_polygons.shp";
    let polygons = read_shapefile(filepath);
    let result = intersects(polygons, regions);

}
