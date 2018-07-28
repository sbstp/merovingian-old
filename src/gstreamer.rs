// extern crate gstreamer;
// extern crate gstreamer_pbutils;

// use gstreamer::ClockTime;
// use gstreamer_pbutils::{Discoverer, DiscovererInfoExt, DiscovererStreamInfoExt};
// use std::env;
// use std::path::Path;

// gstreamer::init();

// let args: Vec<_> = env::args().collect();
// let x = Discoverer::new(ClockTime::from_seconds(60)).expect("cannot build");
// let uri = &format!(
//     "file://{}",
//     Path::new(&args[1]).canonicalize().unwrap().display()
// );

// println!("uri {}", uri);

// let info = x.discover_uri(&uri).expect("cannot discover");

// println!("duration {}m", info.get_duration().minutes().unwrap());
// println!("{} video streams", info.get_video_streams().len());
// println!("{} audio streams", info.get_audio_streams().len());
// for stream in info.get_audio_streams() {
//     println!("-> {}", stream.get_tags().unwrap().to_string());
// }
