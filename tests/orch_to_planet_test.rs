use common_game::components::asteroid::Asteroid;
use common_game::components::sunray::Sunray;
use common_game::protocols::messages::OrchestratorToPlanet;
use common_game::protocols::messages::PlanetToOrchestrator;

mod common;

#[test]
fn test_planet_sunray_ack() {
    let harness = common::TestHarness::setup();
    harness.start();

    harness
        .orch_tx
        .send(OrchestratorToPlanet::Sunray(Sunray::default()))
        .expect("Failed to send sunray message");

    match harness.recv_pto_with_timeout() {
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

    match harness.recv_pto_with_timeout() {
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

    match harness.recv_pto_with_timeout() {
        PlanetToOrchestrator::SunrayAck { planet_id: 0 } => {}
        _other => panic!("Wrong response received"),
    }

    harness
        .orch_tx
        .send(OrchestratorToPlanet::Asteroid(Asteroid::default()))
        .expect("Failed to send asteroid message");

    match harness.recv_pto_with_timeout() {
        PlanetToOrchestrator::AsteroidAck {
            rocket: Some(_),
            planet_id: 0,
        } => {}
        _other => panic!("Wrong response received"),
    }

    let result = harness.stop_and_join();
    assert!(result.is_ok());
}
