pub trait Step: std::fmt::Debug {
    fn eval(&self, _: &mut StepState) -> EvalResult;
}

#[derive(Debug)]
pub enum EvalResult {
    /// Task has not started, or is waiting for a condition to be met
    Pending,
    /// Task has started
    Running,

    /// Step has completed successfully
    Success,
    /// Step has completed unsuccessfully
    Failure,

    /// Step has been lost
    Lost,
    /// Step has errored out
    Error,
}

#[derive(Debug)]
pub struct StepState {
    /// Step State UUID
    pub(crate) step_id: uuid::Uuid,
    /// Current Step's data
    pub(crate) data: serde_json::Value,
    /// Children Step data
    pub(crate) children: Vec<StepState>,
}
impl StepState {
    pub(crate) fn new() -> Self {
        Self {
            step_id: Default::default(),
            data: serde_json::Value::Null,
            children: vec![],
        }
    }
}
