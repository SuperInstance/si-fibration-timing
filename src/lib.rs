//! Fiber bundle model for timing channels in concurrent agent systems.
//!
//! A fiber bundle (E, B, π, F) where:
//! - E (total space) = all possible (task, timing) pairs
//! - B (base space) = task state space
//! - F (fiber) = timing manifold
//! - π: E → B is the projection
//!
//! The connection (parallel transport) IS the conservation law:
//! moving between tasks preserves the timing structure.

/// A point in the base space (task state).
#[derive(Debug, Clone)]
pub struct TaskPoint {
    pub task_id: usize,
    pub state: Vec<f64>,
}

/// A point in the fiber (timing manifold).
#[derive(Debug, Clone)]
pub struct TimingFiber {
    pub latency: f64,
    pub jitter: f64,
    pub throughput: f64,
}

/// A point in the total space (task + timing).
#[derive(Debug, Clone)]
pub struct TotalSpacePoint {
    pub base: TaskPoint,
    pub fiber: TimingFiber,
}

/// A connection (parallel transport rule).
#[derive(Debug, Clone)]
pub struct Connection {
    /// How much timing is preserved during transport (0=none, 1=perfect).
    pub conservation: f64,
    /// Base drift rate.
    pub drift_rate: f64,
}

impl Connection {
    pub fn new(conservation: f64, drift_rate: f64) -> Self {
        Self { conservation, drift_rate }
    }

    /// Parallel transport a fiber from one base point to another.
    pub fn transport(&self, fiber: &TimingFiber, from: &TaskPoint, to: &TaskPoint) -> TimingFiber {
        let distance = euclidean(&from.state, &to.state);
        let decay = (-self.drift_rate * distance).exp();
        TimingFiber {
            latency: fiber.latency * (self.conservation * decay + (1.0 - self.conservation)),
            jitter: fiber.jitter * (self.conservation * decay + (1.0 - self.conservation)),
            throughput: fiber.throughput * (self.conservation * decay + (1.0 - self.conservation)),
        }
    }

    /// Conservation error: how much timing structure is lost during transport.
    pub fn transport_error(&self, fiber: &TimingFiber, transported: &TimingFiber) -> f64 {
        let lat_err = (fiber.latency - transported.latency).powi(2);
        let jit_err = (fiber.jitter - transported.jitter).powi(2);
        let thr_err = (fiber.throughput - transported.throughput).powi(2);
        (lat_err + jit_err + thr_err).sqrt()
    }
}

/// A section: assigns a timing fiber to each task point.
#[derive(Debug, Clone)]
pub struct Section {
    pub points: Vec<TotalSpacePoint>,
}

impl Section {
    pub fn new() -> Self {
        Self { points: Vec::new() }
    }

    pub fn add(&mut self, task: TaskPoint, timing: TimingFiber) {
        self.points.push(TotalSpacePoint { base: task, fiber: timing });
    }

    /// Check if the section is flat (constant timing across tasks).
    pub fn is_flat(&self) -> bool {
        if self.points.len() < 2 {
            return true;
        }
        let ref_timing = &self.points[0].fiber;
        self.points.iter().all(|p| {
            (p.fiber.latency - ref_timing.latency).abs() < 1e-10
            && (p.fiber.jitter - ref_timing.jitter).abs() < 1e-10
            && (p.fiber.throughput - ref_timing.throughput).abs() < 1e-10
        })
    }

    /// Compute curvature: how much the section deviates from parallel transport.
    pub fn curvature(&self, connection: &Connection) -> f64 {
        if self.points.len() < 3 {
            return 0.0;
        }
        let mut total = 0.0;
        let mut count = 0;
        for i in 0..self.points.len() - 2 {
            let p1 = &self.points[i];
            let p2 = &self.points[i + 1];
            let p3 = &self.points[i + 2];

            // Transport p1's fiber to p3 via p2
            let transported_12 = connection.transport(&p1.fiber, &p1.base, &p2.base);
            let transported_123 = connection.transport(&transported_12, &p2.base, &p3.base);

            // Transport p1's fiber directly to p3
            let transported_13 = connection.transport(&p1.fiber, &p1.base, &p3.base);

            total += connection.transport_error(&transported_123, &transported_13);
            count += 1;
        }
        if count == 0 { 0.0 } else { total / count as f64 }
    }

    /// Holonomy: transport around a loop and measure the gap.
    pub fn holonomy(&self, connection: &Connection) -> f64 {
        if self.points.len() < 2 {
            return 0.0;
        }
        let first = &self.points[0];
        let mut current = first.fiber.clone();
        for i in 0..self.points.len() - 1 {
            current = connection.transport(&current, &self.points[i].base, &self.points[i + 1].base);
        }
        // Close the loop
        let last = &self.points[self.points.len() - 1];
        let back = connection.transport(&current, &last.base, &first.base);
        connection.transport_error(&first.fiber, &back)
    }
}

/// A fiber bundle.
pub struct FiberBundle {
    pub connection: Connection,
    pub section: Section,
}

impl FiberBundle {
    pub fn new(connection: Connection) -> Self {
        Self { connection, section: Section::new() }
    }

    /// Compute the holonomy group order (number of distinct holonomy values).
    pub fn holonomy_group_order(&self) -> usize {
        if self.section.points.len() < 3 {
            return 1;
        }
        // Compute holonomy for all cyclic sub-paths
        let mut values = Vec::new();
        for i in 0..self.section.points.len() {
            for j in (i + 1)..self.section.points.len() {
                let sub: Vec<_> = self.section.points[i..=j].to_vec();
                let mut sub_section = Section::new();
                for p in sub {
                    sub_section.add(p.base.clone(), p.fiber.clone());
                }
                let h = sub_section.holonomy(&self.connection);
                if h > 1e-10 {
                    values.push(h);
                }
            }
        }
        // Count distinct values (within tolerance)
        values.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let mut distinct = 0;
        let mut prev = -1.0;
        for v in &values {
            if (v - prev).abs() > 1e-6 {
                distinct += 1;
                prev = *v;
            }
        }
        distinct.max(1)
    }

    /// Conservation law: total timing budget is preserved.
    pub fn conservation_invariant(&self) -> f64 {
        if self.section.points.is_empty() {
            return 0.0;
        }
        let total: f64 = self.section.points.iter()
            .map(|p| p.fiber.latency + p.fiber.jitter + p.fiber.throughput)
            .sum();
        let expected = self.section.points.len() as f64 * 3.0; // If all were unit
        total
    }
}

fn euclidean(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b.iter()).map(|(x, y)| (x - y).powi(2)).sum::<f64>().sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_task(id: usize, state: Vec<f64>) -> TaskPoint {
        TaskPoint { task_id: id, state }
    }

    fn make_timing(latency: f64, jitter: f64, throughput: f64) -> TimingFiber {
        TimingFiber { latency, jitter, throughput }
    }

    #[test]
    fn test_connection_transport() {
        let conn = Connection::new(0.9, 0.1);
        let fiber = make_timing(1.0, 0.1, 10.0);
        let from = make_task(0, vec![0.0, 0.0]);
        let to = make_task(1, vec![1.0, 0.0]);
        let transported = conn.transport(&fiber, &from, &to);
        assert!(transported.latency < fiber.latency);
        assert!(transported.latency > 0.0);
    }

    #[test]
    fn test_perfect_conservation() {
        let conn = Connection::new(1.0, 0.0);
        let fiber = make_timing(5.0, 0.5, 100.0);
        let from = make_task(0, vec![0.0]);
        let to = make_task(1, vec![10.0]);
        let transported = conn.transport(&fiber, &from, &to);
        assert!((transported.latency - fiber.latency).abs() < 1e-10);
    }

    #[test]
    fn test_zero_conservation_identity() {
        // conservation=0 means identity transport — timing unchanged
        let conn = Connection::new(0.0, 0.5);
        let fiber = make_timing(5.0, 0.5, 100.0);
        let from = make_task(0, vec![0.0]);
        let to = make_task(1, vec![5.0]);
        let transported = conn.transport(&fiber, &from, &to);
        assert!((transported.latency - fiber.latency).abs() < 1e-10);
    }

    #[test]
    fn test_transport_error() {
        let conn = Connection::new(0.9, 0.1);
        let a = make_timing(1.0, 0.1, 10.0);
        let b = make_timing(1.0, 0.1, 10.0);
        assert!(conn.transport_error(&a, &b) < 1e-10);
    }

    #[test]
    fn test_section_flat() {
        let mut s = Section::new();
        s.add(make_task(0, vec![0.0]), make_timing(1.0, 0.1, 10.0));
        s.add(make_task(1, vec![1.0]), make_timing(1.0, 0.1, 10.0));
        assert!(s.is_flat());
    }

    #[test]
    fn test_section_not_flat() {
        let mut s = Section::new();
        s.add(make_task(0, vec![0.0]), make_timing(1.0, 0.1, 10.0));
        s.add(make_task(1, vec![1.0]), make_timing(2.0, 0.1, 10.0));
        assert!(!s.is_flat());
    }

    #[test]
    fn test_curvature_triangle() {
        let conn = Connection::new(0.8, 0.2);
        let mut s = Section::new();
        s.add(make_task(0, vec![0.0, 0.0]), make_timing(1.0, 0.1, 10.0));
        s.add(make_task(1, vec![1.0, 0.0]), make_timing(1.2, 0.15, 9.0));
        s.add(make_task(2, vec![0.5, 0.87]), make_timing(1.1, 0.12, 9.5));
        let c = s.curvature(&conn);
        assert!(c >= 0.0);
    }

    #[test]
    fn test_holonomy_loop() {
        let conn = Connection::new(0.7, 0.3);
        let mut s = Section::new();
        s.add(make_task(0, vec![0.0, 0.0]), make_timing(1.0, 0.1, 10.0));
        s.add(make_task(1, vec![1.0, 0.0]), make_timing(1.5, 0.2, 8.0));
        s.add(make_task(2, vec![0.5, 0.87]), make_timing(1.3, 0.15, 9.0));
        let h = s.holonomy(&conn);
        assert!(h >= 0.0);
    }

    #[test]
    fn test_holonomy_perfect_is_zero() {
        let conn = Connection::new(1.0, 0.0);
        let mut s = Section::new();
        s.add(make_task(0, vec![0.0]), make_timing(5.0, 0.5, 100.0));
        s.add(make_task(1, vec![1.0]), make_timing(5.0, 0.5, 100.0));
        let h = s.holonomy(&conn);
        assert!(h < 1e-10);
    }

    #[test]
    fn test_fiber_bundle_creation() {
        let conn = Connection::new(0.9, 0.1);
        let fb = FiberBundle::new(conn);
        assert!(fb.section.points.is_empty());
    }

    #[test]
    fn test_euclidean() {
        let d = euclidean(&[0.0, 0.0], &[3.0, 4.0]);
        assert!((d - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_conservation_invariant() {
        let conn = Connection::new(0.9, 0.1);
        let mut fb = FiberBundle::new(conn);
        fb.section.add(make_task(0, vec![0.0]), make_timing(1.0, 0.5, 10.0));
        fb.section.add(make_task(1, vec![1.0]), make_timing(2.0, 0.3, 8.0));
        let inv = fb.conservation_invariant();
        assert!(inv > 0.0);
    }

    #[test]
    fn test_holonomy_group_order() {
        let conn = Connection::new(0.8, 0.2);
        let mut fb = FiberBundle::new(conn);
        fb.section.add(make_task(0, vec![0.0, 0.0]), make_timing(1.0, 0.1, 10.0));
        fb.section.add(make_task(1, vec![1.0, 0.0]), make_timing(1.5, 0.2, 8.0));
        fb.section.add(make_task(2, vec![0.5, 0.87]), make_timing(1.2, 0.15, 9.0));
        fb.section.add(make_task(3, vec![0.0, 0.0]), make_timing(0.9, 0.08, 11.0));
        let order = fb.holonomy_group_order();
        assert!(order >= 1);
    }

    #[test]
    fn test_chain_transport_composition() {
        // Transport A→B then B→C should differ from A→C (non-flat connection)
        let conn = Connection::new(0.7, 0.5);
        let fiber = make_timing(10.0, 1.0, 100.0);
        let a = make_task(0, vec![0.0]);
        let b = make_task(1, vec![2.0]);
        let c = make_task(2, vec![4.0]);

        let ab = conn.transport(&fiber, &a, &b);
        let abc = conn.transport(&ab, &b, &c);
        let ac = conn.transport(&fiber, &a, &c);

        // They should differ (curvature)
        assert!(conn.transport_error(&abc, &ac) > 1e-10);
    }

    #[test]
    fn test_five_agent_timing() {
        let conn = Connection::new(0.85, 0.15);
        let mut fb = FiberBundle::new(conn.clone());
        for i in 0..5 {
            let angle = i as f64 * std::f64::consts::TAU / 5.0;
            fb.section.add(
                make_task(i, vec![angle.cos(), angle.sin()]),
                make_timing(1.0 + i as f64 * 0.2, 0.1 + i as f64 * 0.02, 10.0 - i as f64),
            );
        }
        assert!(!fb.section.is_flat());
        let h = fb.section.holonomy(&conn);
        assert!(h > 0.0);
    }
}
