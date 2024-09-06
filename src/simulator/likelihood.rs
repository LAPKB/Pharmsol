use crate::{
    data::{error_model::ErrorModel, Observation},
    Data, Subject,
};

use cached::proc_macro::cached;
use cached::UnboundCache;
use indicatif::{ProgressBar, ProgressStyle};
use ndarray::{Array1, Array2, Axis, ShapeBuilder};
use rayon::prelude::*;

use super::Equation;

const FRAC_1_SQRT_2PI: f64 =
    std::f64::consts::FRAC_2_SQRT_PI * std::f64::consts::FRAC_1_SQRT_2 / 2.0;

#[derive(Debug, Clone, Default)]
pub struct SubjectPredictions {
    predictions: Vec<Prediction>,
    flat_predictions: Array1<f64>,
    flat_observations: Array1<f64>,
    flat_time: Array1<f64>,
}
impl SubjectPredictions {
    pub fn get_predictions(&self) -> &Vec<Prediction> {
        &self.predictions
    }

    pub fn flat_observations(&self) -> Vec<f64> {
        self.flat_observations.to_vec()
    }

    pub fn flat_predictions(&self) -> Vec<f64> {
        self.flat_predictions.to_vec()
    }

    pub fn flat_time(&self) -> Vec<f64> {
        self.flat_time.to_vec()
    }

    pub(crate) fn likelihood(&self, error_model: &ErrorModel) -> f64 {
        self.predictions
            .iter()
            .map(|p| p.likelihood(error_model))
            .product()
    }

    pub(crate) fn squared_error(&self) -> f64 {
        self.predictions
            .iter()
            .map(|p| (p.observation - p.prediction).powi(2))
            .sum()
    }
}

/// Probability density function
fn normpdf(obs: f64, pred: f64, sigma: f64) -> f64 {
    (FRAC_1_SQRT_2PI / sigma) * (-((obs - pred) * (obs - pred)) / (2.0 * sigma * sigma)).exp()
}

impl From<Vec<Prediction>> for SubjectPredictions {
    fn from(predictions: Vec<Prediction>) -> Self {
        Self {
            flat_predictions: predictions.iter().map(|p| p.prediction).collect(),
            flat_observations: predictions.iter().map(|p| p.observation).collect(),
            flat_time: predictions.iter().map(|p| p.time).collect(),
            predictions,
        }
    }
}

pub struct PopulationPredictions {
    pub subject_predictions: Array2<SubjectPredictions>,
}

impl Default for PopulationPredictions {
    fn default() -> Self {
        Self {
            subject_predictions: Array2::default((0, 0)),
        }
    }
}

impl From<Array2<SubjectPredictions>> for PopulationPredictions {
    fn from(subject_predictions: Array2<SubjectPredictions>) -> Self {
        Self {
            subject_predictions,
        }
    }
}

pub fn pf_psi(
    equation: &Equation,
    subjects: &Data,
    support_points: &Array2<f64>,
    error_model: &ErrorModel,
    nparticles: usize,
    progress: bool,
) -> Array2<f64> {
    let mut psi: Array2<f64> = Array2::default((subjects.len(), support_points.nrows()).f());
    let subjects = subjects.get_subjects();
    let pb = match progress {
        true => {
            let pb = ProgressBar::new(psi.ncols() as u64 * psi.nrows() as u64);
            pb.set_style(
                ProgressStyle::with_template(
                    "Cycle #1:\n[{elapsed_precise}] {bar:40.green} {percent}% ETA:{eta}",
                )
                .unwrap()
                .progress_chars("##-"),
            );
            Some(pb)
        }
        false => None,
    };
    psi.axis_iter_mut(Axis(0))
        .into_par_iter()
        .enumerate()
        .for_each(|(i, mut row)| {
            row.axis_iter_mut(Axis(0))
                .into_par_iter()
                .enumerate()
                .for_each(|(j, mut element)| {
                    let subject = subjects.get(i).unwrap();
                    let ll = equation.particle_filter(
                        subject,
                        support_points.row(j).to_vec().as_ref(),
                        nparticles,
                        error_model,
                    );

                    element.fill(ll);
                    if let Some(pb_ref) = pb.as_ref() {
                        pb_ref.inc(1);
                    }
                });
        });
    if let Some(pb_ref) = pb.as_ref() {
        pb_ref.finish();
    }
    psi
}

// impl PopulationPredictions {
//     pub fn get_psi(&self, ep: &ErrorModel) -> Array2<f64> {
//         let mut psi = Array2::zeros((
//             self.subject_predictions.nrows(),
//             self.subject_predictions.ncols(),
//         ));
//         psi.axis_iter_mut(Axis(0))
//             .into_par_iter()
//             .enumerate()
//             .for_each(|(i, mut row)| {
//                 row.axis_iter_mut(Axis(0))
//                     .into_par_iter()
//                     .enumerate()
//                     .for_each(|(j, mut element)| {
//                         element.fill(self.subject_predictions.get((i, j)).unwrap().likelihood(ep));
//                     })
//             });
//         psi
//     }
// }
//TODO: Export this function as a method of Data
pub fn psi(
    equation: &Equation,
    subjects: &Data,
    support_points: &Array2<f64>,
    error_model: &ErrorModel,
    progress: bool,
    cache: bool,
) -> Array2<f64> {
    let mut pred: Array2<f64> = Array2::default((subjects.len(), support_points.nrows()).f());
    let subjects = subjects.get_subjects();
    let pb = match progress {
        true => {
            let pb = ProgressBar::new(pred.ncols() as u64 * pred.nrows() as u64);
            pb.set_style(
                ProgressStyle::with_template(
                    "Cycle #1:\n[{elapsed_precise}] {bar:40.green} {percent}% ETA:{eta}",
                )
                .unwrap()
                .progress_chars("##-"),
            );
            Some(pb)
        }
        false => None,
    };

    pred.axis_iter_mut(Axis(0))
        .into_par_iter()
        .enumerate()
        .for_each(|(i, mut row)| {
            row.axis_iter_mut(Axis(0))
                .into_par_iter()
                .enumerate()
                .for_each(|(j, mut element)| {
                    let subject = subjects.get(i).unwrap();
                    let likelihood = if cache {
                        subject_likelihood(
                            subject,
                            equation,
                            support_points.row(j).to_vec().as_ref(),
                            error_model,
                        )
                    } else {
                        subject_likelihood_no_cache(
                            subject,
                            equation,
                            support_points.row(j).to_vec().as_ref(),
                            error_model,
                        )
                    };
                    element.fill(likelihood);
                    if let Some(pb_ref) = pb.as_ref() {
                        pb_ref.inc(1);
                    }
                });
        });
    if let Some(pb_ref) = pb.as_ref() {
        pb_ref.finish();
    }

    pred.into()
}

// Commented code for disk cache
// #[io_cached(
//    disk = true,
//    map_error = r##"|e| anyhow!(e)"##,
//    convert = r#"{ format!("{}{}", subject.id(), spphash(support_point)) }"#,
//    ty = "DiskCache<String, f64>"
//)]

#[inline(always)]
#[cached(
    ty = "UnboundCache<String, f64>",
    create = "{ UnboundCache::with_capacity(100_000) }",
    convert = r#"{ format!("{}{}", subject.id(), spphash(support_point)) }"#
)]
fn subject_likelihood(
    subject: &Subject,
    equation: &Equation,
    support_point: &Vec<f64>,
    error_model: &ErrorModel,
) -> f64 {
    let ypred = equation.simulate_subject(subject, support_point);
    let likelihood = ypred.likelihood(error_model);
    likelihood
}

fn spphash(spp: &[f64]) -> u64 {
    spp.iter().fold(0, |acc, x| acc + x.to_bits())
}

/// Prediction holds an observation and its prediction
#[derive(Debug, Clone, Copy)]
pub struct Prediction {
    time: f64,
    observation: f64,
    prediction: f64,
    outeq: usize,
    errorpoly: Option<(f64, f64, f64, f64)>,
}

impl Prediction {
    pub fn time(&self) -> f64 {
        self.time
    }
    pub fn observation(&self) -> f64 {
        self.observation
    }
    pub fn prediction(&self) -> f64 {
        self.prediction
    }
    pub fn outeq(&self) -> usize {
        self.outeq
    }
    pub fn errorpoly(&self) -> Option<(f64, f64, f64, f64)> {
        self.errorpoly
    }
    pub fn prediction_error(&self) -> f64 {
        self.prediction - self.observation
    }
    pub fn percentage_error(&self) -> f64 {
        ((self.prediction - self.observation) / self.observation) * 100.0
    }
    pub fn absolute_error(&self) -> f64 {
        (self.prediction - self.observation).abs()
    }
    pub fn squared_error(&self) -> f64 {
        (self.prediction - self.observation).powi(2)
    }
    pub fn likelihood(&self, error_model: &ErrorModel) -> f64 {
        let sigma = error_model.estimate_sigma(self);
        normpdf(self.observation, self.prediction, sigma)
    }
}

pub(crate) trait ToPrediction {
    fn to_obs_pred(&self, pred: f64) -> Prediction;
}

impl ToPrediction for Observation {
    fn to_obs_pred(&self, pred: f64) -> Prediction {
        Prediction {
            time: self.time(),
            observation: self.value(),
            prediction: pred,
            outeq: self.outeq(),
            errorpoly: self.errorpoly(),
        }
    }
}

impl Default for Prediction {
    fn default() -> Self {
        Self {
            time: 0.0,
            observation: 0.0,
            prediction: 0.0,
            outeq: 0,
            errorpoly: None,
        }
    }
}

// Implement display for Prediction
impl std::fmt::Display for Prediction {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "Time: {:.2}\tObs: {:.4}\tPred: {:.15}\tOuteq: {:.2}",
            self.time, self.observation, self.prediction, self.outeq
        )
    }
}
