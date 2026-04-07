// airgenome — API compatible with nexus

pub mod resource_guard;

pub mod rules {
    use crate::Vitals;

    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    pub enum Severity {
        Ok,
        Warn,
        Critical,
    }

    pub fn firing(_v: &Vitals) -> Vec<usize> {
        vec![]
    }

    pub fn severity(_rule_idx: usize, _v: &Vitals) -> Severity {
        Severity::Ok
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Axis {
    Cpu,
    Ram,
    Gpu,
    Npu,
    Power,
    Io,
}

pub struct Rule {
    pub name: &'static str,
}

pub static RULES: &[Rule] = &[];

#[derive(Debug, Clone)]
pub struct Vitals {
    pub cpu: f64,
    pub ram: f64,
    pub gpu: f64,
    pub npu: f64,
    pub power: f64,
    pub io: f64,
}

impl Vitals {
    pub fn get(&self, axis: Axis) -> f64 {
        match axis {
            Axis::Cpu => self.cpu,
            Axis::Ram => self.ram,
            Axis::Gpu => self.gpu,
            Axis::Npu => self.npu,
            Axis::Power => self.power,
            Axis::Io => self.io,
        }
    }
}

pub fn sample() -> Vitals {
    Vitals {
        cpu: 0.0,
        ram: 0.0,
        gpu: 0.0,
        npu: 0.0,
        power: 0.0,
        io: 0.0,
    }
}
