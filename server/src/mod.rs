//! This crate contains server-only components of Playform.

#![deny(missing_docs)]
#![deny(warnings)]

#![feature(core)]
#![feature(collections)]
#![feature(env)]
#![feature(hash)]
#![feature(io)]
#![feature(slicing_syntax)]
#![feature(std_misc)]
#![feature(test)]
#![feature(unboxed_closures)]
#![feature(unsafe_destructor)]

extern crate common;
extern crate env_logger;
#[macro_use]
extern crate log;
extern crate nalgebra;
extern crate nanomsg;
extern crate ncollide_entities;
extern crate ncollide_queries;
extern crate noise;
extern crate opencl;
extern crate rand;
extern crate "rustc-serialize" as rustc_serialize;
extern crate test;
extern crate time;

mod gaia_thread;
mod gaia_update;
mod in_progress_terrain;
mod init_mobs;
mod mob;
mod octree;
mod opencl_context;
mod physics;
mod player;
mod server;
mod server_thread;
mod server_update;
mod sun;
mod terrain;
mod update;

use common::communicate::spark_socket_receiver;
use common::stopwatch::TimerSet;
use gaia_thread::gaia_thread;
use nanomsg::{Socket, Protocol};
use server::Server;
use std::sync::mpsc::channel;
use std::thread::Thread;

#[allow(missing_docs)]
pub fn main() {
  env_logger::init().unwrap();

  debug!("starting");

  let mut args: Vec<String> = std::env::args().collect();
  if args.len() != 2 {
    panic!("Need only server listen URL as a parameter");
  }

  let listen_url = args.pop().unwrap();

  let mut incoming = Socket::new(Protocol::Rep).unwrap();

  let mut endpoints = Vec::new();
  endpoints.push(incoming.bind(listen_url.as_slice()).unwrap());

  let incoming = spark_socket_receiver(incoming);

  let timers = TimerSet::new();
  let world = Server::new(&timers);

  let (ups_to_gaia_send, ups_to_gaia_recv) = channel();
  let (ups_from_gaia_send, ups_from_gaia_recv) = channel();

  let _gaia_thread = {
    let terrain = world.terrain_game_loader.terrain.clone();
    let id_allocator = world.id_allocator.clone();
    Thread::spawn(move || {
      gaia_thread(
        id_allocator,
        ups_to_gaia_recv,
        ups_from_gaia_send,
        terrain,
      );
    })
  };

  server_thread::server_thread(
    &timers,
    world,
    &mut endpoints,
    incoming,
    ups_from_gaia_recv,
    ups_to_gaia_send,
  );

  for mut endpoint in endpoints.into_iter() {
    endpoint.shutdown().unwrap();
  }

  timers.print();

  debug!("finished");
}
