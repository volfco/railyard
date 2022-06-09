mod eval;
pub mod steps;

use crate::eval::{Step, StepState};
use std::collections::HashMap;
//
// use boxcar_rpc;
// use tokio::time::Instant;
//
// use serde::de::DeserializeOwned;
// use serde::ser::Serialize;
//
// use etcd_client::Client;
// use tokio::sync::RwLock;
//
// struct KeyMap<T: Serialize + DeserializeOwned> {
//     etcd_client: Client,
//     etcd_path: String,
//
//     inner: Arc<RwLock<HashMap<String, T>>>,
// }
//
// /// Stored at `/testing/railyard/tasks/{uuid}`
// struct RpcInvocation {
//     /// ID of this invocation
//     invocation_uuid: Uuid,
//     /// The boxcar_service ID the Rpc was sent to
//     boxcar_service_id: Uuid,
//     /// Boxcar RPC slot ID
//     boxcar_rpc_slot: u16,
//
//     boxcar_request: boxcar_rpc::RpcRequest,
// }
//
// struct BoxcarCluster {
//     /// Map of boxcar_service IDs to open boxcar_rpc::Client(s). Instant is last access time.
//     /// Used to cache clients and prevent constant re-connecting
//     client_map: HashMap<Uuid, (Instant, boxcar_rpc::Client)>,
//
//     /// Task UUID -> Rpc Invocation
//     task_map: HashMap<Uuid, RpcInvocation>,
// }

pub struct Job {
    steps: Box<dyn Step>,
}

pub fn eval_job(job: Job) {
    let mut step_state = StepState {
        step_id: Default::default(),
        data: Default::default(),
        children: vec![],
    };

    println!("state: {:?}", &step_state);
    println!("{:?}", job.steps.eval(&mut step_state));
    println!("state: {:?}", &step_state);
    println!("{:?}", job.steps.eval(&mut step_state));
    println!("state: {:?}", &step_state);
}

#[cfg(test)]
mod tests {
    use crate::steps::{Branch, FlipBool};
    use crate::{eval_job, Job};

    #[test]
    fn test_job() {
        tracing_subscriber::fmt::init();

        let job = Job {
            steps: Box::new(Branch {
                base: Box::new(FlipBool {
                    initial_state: true,
                }),
                left: None,
                right: None,
            }),
        };

        eval_job(job);
    }

    // #[test]
    // fn it_works() {
    //     let job = JobDef {
    //         name: Some("my-job".to_string()),
    //         tree: JobType::Branch(
    //             Box::new(JobType::Single(TaskDef {
    //                 method: "hello-world".to_string(),
    //                 arguments: serde_json::Value::Null,
    //                 resources: HashMap::new(),
    //                 policy: None,
    //                 raw: None
    //             })),
    //             Some(Box::new(JobType::Single(TaskDef {
    //                 method: "hello-world".to_string(),
    //                 arguments: serde_json::Value::Null,
    //                 resources: HashMap::new(),
    //                 policy: None,
    //                 raw: None
    //             }))),
    //             Some(Box::new(JobType::Single(TaskDef {
    //                 method: "hello-world".to_string(),
    //                 arguments: serde_json::Value::Null,
    //                 resources: HashMap::new(),
    //                 policy: None,
    //                 raw: None
    //             }))),
    //         ),
    //         policy: None,
    //         constraints: None,
    //     };
    //
    //     let output = serde_json::to_string(&job).unwrap();
    //     println!("{}", output);
    //
    // }
}
