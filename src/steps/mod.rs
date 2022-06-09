pub mod boxcar;

use crate::eval::*;
use tracing::trace;

#[derive(Debug)]
pub struct FlipBool {
    pub(crate) initial_state: bool,
}
impl Step for FlipBool {
    fn eval(&self, state: &mut StepState) -> EvalResult {
        if serde_json::Value::Null == state.data {
            state.data = serde_json::Value::Bool(self.initial_state);
        }

        if let serde_json::Value::Bool(val) = state.data {
            state.data = serde_json::Value::Bool(!val);
        }

        if state.data.as_bool().unwrap() {
            EvalResult::Success
        } else {
            EvalResult::Running
        }
    }
}

#[derive(Debug)]
pub struct Group {
    values: Vec<Box<dyn Step>>,
    /// requite all elements to return successful in order for step to return successful
    require: bool,
    /// Run steps in parallel.
    parallel: bool,
    /// When parallel=false, should step execution stop on error
    stop: bool,
}

#[derive(Debug)]
pub struct Branch {
    pub(crate) base: Box<dyn Step>,
    pub(crate) left: Option<Box<dyn Step>>,
    pub(crate) right: Option<Box<dyn Step>>,
}
impl Step for Branch {
    fn eval(&self, state: &mut StepState) -> EvalResult {
        // self.children[0] is base step state
        // self.children[1] is left step state
        // self.children[2] is right step state
        if state.children.is_empty() {
            trace!("step state children not populated");
            state.children.push(StepState::new());
            state.children.push(StepState::new());
            state.children.push(StepState::new());
        }

        let result = match self.base.eval(&mut state.children[0]) {
            // if the base step is in Pending or Running, return Running
            EvalResult::Pending | EvalResult::Running => EvalResult::Running,
            EvalResult::Success => {
                trace!("base step returned EvalResult::Success");
                match &self.left {
                    None => {
                        trace!("left step is not defined. returning EvalResult::Success");
                        EvalResult::Success
                    }
                    Some(left_step) => {
                        trace!("left step defined. evaluating");
                        let step = left_step.eval(&mut state.children[1]);
                        trace!("left step returned: {:?}. ", &step);

                        step
                    }
                }
            }
            EvalResult::Failure => {
                trace!("base step returned EvalResult::Failure");
                match &self.right {
                    None => {
                        trace!("right step is not defined. returning EvalResult::Success");
                        EvalResult::Success
                    }
                    Some(right_step) => {
                        trace!("right step defined. evaluating");
                        let step = right_step.eval(&mut state.children[1]);
                        trace!("right step returned: {:?}. ", &step);

                        step
                    }
                }
            }
            EvalResult::Lost | EvalResult::Error => EvalResult::Failure,
        };

        trace!("returning {:?}", &result);

        result
    }
}
