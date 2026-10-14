use crate::instance::Instance;
use crate::mode::Mode;
use crate::workflow::{Workflow, WorkflowDescriptor};

pub trait Sealed {}

impl<W: Workflow> Sealed for W {}

impl<W, M: Mode> Sealed for WorkflowDescriptor<W, M> {}

impl<W, M: Mode> Sealed for Instance<W, M> {}
