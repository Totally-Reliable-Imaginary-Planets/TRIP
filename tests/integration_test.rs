use common_game::components::asteroid::Asteroid;
use common_game::components::sunray::Sunray;
use common_game::protocols::messages::OrchestratorToPlanet;
use common_game::protocols::messages::PlanetToOrchestrator;
use std::sync::mpsc;
use trip::Trip;

mod common;

#[test]
fn test_planet_run() {
    let harness = common::TestHarness::setup();
    harness.start();

    let result = harness.stop_and_join();
    assert!(result.is_ok());
}

#[test]
fn test_planet_sunray_ack() {
    let harness = common::TestHarness::setup();
    harness.start();

    harness
        .orch_tx
        .send(OrchestratorToPlanet::Sunray(Sunray::default()))
        .expect("Failed to send sunray message");

    match harness.recv_with_timeout() {
        PlanetToOrchestrator::SunrayAck { planet_id: 0 } => {}
        _other => panic!("Wrong response received"),
    }

    let result = harness.stop_and_join();
    assert!(result.is_ok());
}

#[test]
fn test_planet_asteroid_ack() {
    let harness = common::TestHarness::setup();
    harness.start();

    harness
        .orch_tx
        .send(OrchestratorToPlanet::Asteroid(Asteroid::default()))
        .expect("Failed to send asteroid message");

    match harness.recv_with_timeout() {
        PlanetToOrchestrator::AsteroidAck {
            rocket: None,
            planet_id: 0,
        } => {}
        _other => panic!("Wrong response received"),
    }

    let result = harness.stop_and_join();
    assert!(result.is_ok());
}

#[test]
fn test_planet_survive_asteroid() {
    let harness = common::TestHarness::setup();
    harness.start();

    harness
        .orch_tx
        .send(OrchestratorToPlanet::Sunray(Sunray::default()))
        .expect("Failed to send sunray message");

    match harness.recv_with_timeout() {
        PlanetToOrchestrator::SunrayAck { planet_id: 0 } => {}
        _other => panic!("Wrong response received"),
    }

    harness
        .orch_tx
        .send(OrchestratorToPlanet::Asteroid(Asteroid::default()))
        .expect("Failed to send asteroid message");

    match harness.recv_with_timeout() {
        PlanetToOrchestrator::AsteroidAck {
            rocket: Some(_),
            planet_id: 0,
        } => {}
        _other => panic!("Wrong response received"),
    }

    let result = harness.stop_and_join();
    assert!(result.is_ok());
}

#[test]
fn test_concurrent_message_sending() {
    let (orch_tx, orch_rx) = mpsc::channel();
    let (planet_tx, _planet_rx) = mpsc::channel();
    let (_expl_tx, expl_rx) = mpsc::channel();
    let (planet_tx2, _planet_rx2) = mpsc::channel();

    let mut trip = Trip::new(1, orch_rx, planet_tx, expl_rx, planet_tx2).unwrap();

    let handle = std::thread::spawn(move || {
        for _ in 0..100 {
            trip.run().ok();
        }
    });

    // Send many messages from multiple threads
    let tx1 = orch_tx.clone();
    let t1 = std::thread::spawn(move || {
        for _ in 0..50 {
            tx1.send(OrchestratorToPlanet::StartPlanetAI).unwrap();
        }
    });

    // Send many messages from multiple threads
    let tx2 = orch_tx.clone();
    let t2 = std::thread::spawn(move || {
        for _ in 0..50 {
            tx2.send(OrchestratorToPlanet::StopPlanetAI).unwrap();
        }
    });

    t1.join().unwrap();
    t2.join().unwrap();
    drop(orch_tx);

    let result = handle.join();
    assert!(result.is_ok());
}
