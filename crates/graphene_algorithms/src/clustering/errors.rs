use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum ClusteringError {
    InvalidDamping(f64),
    EmptyData,
    InvalidK(usize),
    ConvergenceFailed(usize),
}

impl fmt::Display for ClusteringError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ClusteringError::InvalidDamping(d) => write!(f, "Damping must be in [0.5, 1.0), got {}", d),
            ClusteringError::EmptyData => write!(f, "Empty dataset provided"),
            ClusteringError::InvalidK(k) => write!(f, "Invalid cluster count k: {}", k),
            ClusteringError::ConvergenceFailed(iter) => write!(f, "Clustering failed to converge after {} iterations", iter),
        }
    }
}

impl std::error::Error for ClusteringError {}
