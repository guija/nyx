extern crate nyx_space as nyx;

use nyx::dynamics::guidance::{Mnvr, Thruster};
use nyx::linalg::Vector3;
use nyx::md::optimizer::*;
use nyx::md::ui::*;

#[test]
fn fb_tgt_sma_ecc() {
    if pretty_env_logger::try_init().is_err() {
        println!("could not init env_logger");
    }

    let cosm = Cosm::de438();
    let eme2k = cosm.frame("EME2000");

    let orig_dt = Epoch::from_gregorian_utc_at_midnight(2020, 1, 1);

    let xi_orig = Orbit::keplerian(8_000.0, 0.2, 30.0, 60.0, 60.0, 0.0, orig_dt, eme2k);

    let target_delta_t: Duration = xi_orig.period() / 2.0;

    let spacecraft = Spacecraft {
        orbit: xi_orig,
        dry_mass_kg: 10.0,
        fuel_mass_kg: 90.0,
        thruster: Some(Thruster {
            thrust: 500.0,
            isp: 300.0,
        }),
        mode: GuidanceMode::Thrust,
        ..Default::default()
    };

    let dynamics = SpacecraftDynamics::new(OrbitalDynamics::two_body());
    let setup = Propagator::default(dynamics);

    // Define the objective
    let objectives = [
        Objective::within_tolerance(StateParameter::Eccentricity, 0.4, 1e-5),
        Objective::within_tolerance(StateParameter::SMA, 8100.0, 0.1),
    ];

    // The variables in this targeter
    let variables = [
        Variable::from(Vary::MnvrAlpha).with_initial_guess(-0.3021017411736592_f64.to_radians()),
        // Variable::from(Vary::MnvrAlphaDot).with_initial_guess(45.0),
        Variable::from(Vary::MnvrAlphaDDot)
            .with_initial_guess(-2.1098425649685995_f64.to_radians()),
        Variable::from(Vary::MnvrBeta).with_initial_guess(0.3530352682197084_f64.to_radians()),
        // Variable::from(Vary::MnvrBetaDot).with_initial_guess(45.0),
        Variable::from(Vary::MnvrBetaDDot)
            .with_initial_guess(4.152947118658474e-7_f64.to_radians()),
        // Variable::from(Vary::Duration).with_initial_guess(5.0),
    ];

    let tgt = Optimizer::new(&setup, variables, objectives);

    println!("{}", tgt);

    let achievement_epoch = orig_dt + target_delta_t;

    let solution_fd = tgt
        .try_achieve_from(spacecraft, orig_dt, achievement_epoch)
        .unwrap();

    println!("Finite differencing solution: {}", solution_fd);

    let gmat_sol = 3.1160765514523914;
    println!(
        "GMAT validation - tgt_sma_from_peri: Δv = {:.3} m/s\terr = {:.6} m/s",
        solution_fd.correction.norm() * 1e3,
        (solution_fd.correction.norm() - gmat_sol).abs() * 1e3
    );
    // GMAT validation
    assert!(
        (solution_fd.correction.norm() - gmat_sol).abs() < 1e-6
            || solution_fd.correction.norm() < gmat_sol,
        "Finite differencing result different from GMAT and greater!"
    );
}

#[test]
fn val_tgt_finite_burn() {
    // In this test, we take a known finite burn solution and use the optimizer to solve for it.
    // It should converge after 0 iterations.

    if pretty_env_logger::try_init().is_err() {
        println!("could not init env_logger");
    }

    let cosm = Cosm::de438_gmat();
    let eme2k = cosm.frame("EME2000");

    // Build the initial spacecraft state
    let start_time = Epoch::from_gregorian_tai_at_midnight(2002, 1, 1);
    let orbit = Orbit::cartesian(
        -2436.45, -2436.45, 6891.037, 5.088_611, -5.088_611, 0.0, start_time, eme2k,
    );

    // Define the thruster
    let monoprop = Thruster {
        thrust: 10.0,
        isp: 300.0,
    };
    let dry_mass = 1e3;
    let fuel_mass = 756.0;
    let sc_state = Spacecraft::from_thruster(
        orbit,
        dry_mass,
        fuel_mass,
        monoprop,
        GuidanceMode::Custom(0),
    );

    let prop_time = 50.0 * TimeUnit::Minute;

    let end_time = start_time + prop_time;

    // Define the dynamics
    let bodies = vec![Bodies::Luna, Bodies::Sun, Bodies::JupiterBarycenter];
    let orbital_dyn = OrbitalDynamics::point_masses(&bodies, cosm);

    // With 100% thrust: RSS errors:     pos = 3.14651e1 km      vel = 3.75245e-2 km/s

    // Define the maneuver and its schedule
    let mnvr0 = Mnvr::from_time_invariant(
        Epoch::from_gregorian_tai_at_midnight(2002, 1, 1),
        end_time,
        1.0, // Full thrust
        Vector3::new(1.0, 0.0, 0.0),
        Frame::Inertial,
    );

    // And create the spacecraft with that controller
    let sc = SpacecraftDynamics::from_ctrl(orbital_dyn.clone(), Arc::new(mnvr0));
    // Setup a propagator, and propagate for that duration
    // NOTE: We specify the use an RK89 to match the GMAT setup.
    let prop = Propagator::rk89(sc, PropOpts::with_fixed_step(10.0 * TimeUnit::Second));
    let sc_xf_desired = prop.with(sc_state).for_duration(prop_time).unwrap();

    // Build an impulsive targeter for this known solution
    let sc_no_thrust = SpacecraftDynamics::new(orbital_dyn);
    let prop_no_thrust = Propagator::rk89(
        sc_no_thrust,
        PropOpts::with_fixed_step(10.0 * TimeUnit::Second),
    );
    let impulsive_tgt = Optimizer::delta_v(
        &prop_no_thrust,
        [
            Objective::within_tolerance(StateParameter::X, sc_xf_desired.orbit.x, 1e-5),
            Objective::within_tolerance(StateParameter::Y, sc_xf_desired.orbit.y, 1e-5),
            Objective::within_tolerance(StateParameter::Z, sc_xf_desired.orbit.z, 1e-5),
        ],
    )
    .try_achieve_from(sc_state, sc_state.epoch(), sc_xf_desired.epoch())
    .unwrap();

    println!("{}", impulsive_tgt);
    println!("\n\nKNOWN SOLUTION\n{}", mnvr0);

    // Solve for this known solution
    Optimizer::convert_impulsive_mnvr(sc_state, impulsive_tgt.correction, &prop).unwrap();
}
