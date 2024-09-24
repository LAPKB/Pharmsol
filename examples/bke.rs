fn main() {
    use pharmsol::*;

    let subject = Subject::builder("1")
        .infusion(0., 500.0, 0, 0.5)
        .observation(0.5, 3.3371689999999998, 0)
        .observation(1., 3.4750559999999999, 0)
        .observation(2., 2.621165, 0)
        .observation(3., 1.9103840000000001, 0)
        .observation(4., 1.4256610000000001, 0)
        .observation(6., 0.80077540000000003, 0)
        .observation(8., 0.35291400000000001, 0)
        .observation(12., 0.1020168, 0)
        .observation(18., 0.015118929999999999, 0)
        .observation(24., 0.0025999220000000002, 0)
        .build();

    let an = equation::Analytical::new(
        one_compartment,
        |_p, _t, _cov| {},
        |_p| lag! {},
        |_p| fa! {},
        |_p, _t, _cov, _x| {},
        |x, p, _t, _cov, y| {
            fetch_params!(p, _ke, v);
            y[0] = x[0] / v;
        },
        (1, 1),
    );

    let ode = equation::ODE::new(
        |x, p, _t, dx, rateiv, _cov| {
            // fetch_cov!(cov, t, wt);
            fetch_params!(p, ke, _v);
            dx[0] = -ke * x[0] + rateiv[0];
        },
        |_p| lag! {},
        |_p| fa! {},
        |_p, _t, _cov, _x| {},
        |x, p, _t, _cov, y| {
            fetch_params!(p, _ke, v);
            y[0] = x[0] / v;
        },
        (1, 1),
    );

    let net = equation::ODENet::new(
        vec![dmatrix![-1.0], dmatrix![0.0]],
        vec![OutEq::new(0, 0, Op::Div(1))],
        (1, 1),
    );

    let op = an.estimate_predictions(&subject, &vec![0.31261484375, 116.3623046875]);
    println!("Analytical: \n{:#?}", op.flat_predictions());

    let op = ode.estimate_predictions(&subject, &vec![0.31261484375, 116.3623046875]);
    println!("ODE: \n{:#?}", op.flat_predictions());

    let op = net.estimate_predictions(&subject, &vec![0.31261484375, 116.3623046875]);
    println!("Net: \n{:#?}", op.flat_predictions());
}
