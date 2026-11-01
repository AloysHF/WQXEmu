// Hardware machine implementations.
//
// Each supported model lives in its own module and implements the
// `Machine` trait. `create_machine` instantiates a model from a
// `MachineModel`. The model is chosen explicitly by each frontend; the
// fixed firmware directory layout below the system directory names it.

use anyhow::Result;

use crate::machine::{Machine, MachineModel, RomFiles};

pub mod cc800;
pub mod nc1020;
pub mod nc2000;
pub mod nc3000;
pub mod pc1000;

/// Create a machine for the given model, loading its ROM files.
pub fn create_machine(model: MachineModel, files: &RomFiles) -> Result<Box<dyn Machine>> {
    match model {
        MachineModel::Cc800 => Ok(Box::new(cc800::Cc800Machine::new(files)?)),
        MachineModel::Nc1020 => Ok(Box::new(nc1020::Nc1020Machine::new(files)?)),
        MachineModel::Pc1000 => Ok(Box::new(pc1000::Pc1000Machine::new(files)?)),
        MachineModel::Nc2000 => Ok(Box::new(nc2000::Nc2000Machine::new(files)?)),
        MachineModel::Nc3000 => Ok(Box::new(nc3000::Nc3000Machine::new(files)?)),
    }
}
