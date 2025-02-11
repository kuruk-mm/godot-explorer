use std::{
    cell::RefCell,
    rc::Rc,
    sync::{Arc, Mutex},
};

use deno_core::{op, Op, OpDecl, OpState};

use crate::dcl::{
    crdt::{
        message::{append_gos_component, process_many_messages, put_or_delete_lww_component},
        SceneCrdtState,
    },
    js::{SceneDying, SceneMainCrdtFileContent},
    serialization::{reader::DclReader, writer::DclWriter},
    RendererResponse, SceneId, SceneResponse, SharedSceneCrdtState,
};

use super::{SceneElapsedTime, SceneLogs};

// list of op declarations
pub fn ops() -> Vec<OpDecl> {
    vec![
        op_crdt_send_to_renderer::DECL,
        op_crdt_recv_from_renderer::DECL,
    ]
}

// receive and process a buffer of crdt messages
#[op(v8)]
fn op_crdt_send_to_renderer(op_state: Rc<RefCell<OpState>>, messages: &[u8]) {
    let dying = op_state.borrow().borrow::<SceneDying>().0;
    if dying {
        return;
    }

    let mut op_state = op_state.borrow_mut();
    let elapsed_time = op_state.borrow::<SceneElapsedTime>().0;
    let scene_id = op_state.take::<SceneId>();

    let logs = op_state.take::<SceneLogs>();
    op_state.put(SceneLogs(Vec::new()));

    let mutex_scene_crdt_state = op_state.take::<SharedSceneCrdtState>();
    let cloned_scene_crdt = mutex_scene_crdt_state.clone();
    let mut scene_crdt_state = cloned_scene_crdt.lock().unwrap();

    let mut stream = DclReader::new(messages);
    process_many_messages(&mut stream, &mut scene_crdt_state);

    let dirty = scene_crdt_state.take_dirty();
    drop(scene_crdt_state);
    drop(cloned_scene_crdt);

    op_state.put(mutex_scene_crdt_state);
    op_state.put(scene_id);

    let sender = op_state.borrow_mut::<std::sync::mpsc::SyncSender<SceneResponse>>();
    sender
        .send(SceneResponse::Ok(scene_id, dirty, logs.0, elapsed_time))
        .expect("error sending scene response!!")
}

#[op(v8)]
async fn op_crdt_recv_from_renderer(op_state: Rc<RefCell<OpState>>) -> Vec<Vec<u8>> {
    let dying = op_state.borrow().borrow::<SceneDying>().0;
    if dying {
        return vec![];
    }

    let mut receiver = op_state
        .borrow_mut()
        .take::<tokio::sync::mpsc::Receiver<RendererResponse>>();
    let response = receiver.recv().await;

    let mut op_state = op_state.borrow_mut();
    op_state.put(receiver);

    let mutex_scene_crdt_state = op_state.take::<Arc<Mutex<SceneCrdtState>>>();
    let cloned_scene_crdt = mutex_scene_crdt_state.clone();
    let scene_crdt_state = cloned_scene_crdt.lock().unwrap();

    let data = match response {
        Some(RendererResponse::Ok(data)) => {
            let (_dirty_entities, dirty_lww_components, dirty_gos_components) = data;

            let mut data_buf = Vec::new();
            let mut data_writter = DclWriter::new(&mut data_buf);

            for (component_id, entities) in dirty_lww_components {
                for entity_id in entities {
                    if let Err(err) = put_or_delete_lww_component(
                        &scene_crdt_state,
                        &entity_id,
                        &component_id,
                        &mut data_writter,
                    ) {
                        tracing::info!("error writing crdt message: {err}");
                    }
                }
            }

            for (component_id, entities) in dirty_gos_components {
                for (entity_id, element_count) in entities {
                    if let Err(err) = append_gos_component(
                        &scene_crdt_state,
                        &entity_id,
                        &component_id,
                        element_count,
                        &mut data_writter,
                    ) {
                        tracing::info!("error writing crdt message: {err}");
                    }
                }
            }

            data_buf
        }
        _ => {
            // channel has been closed, shutdown gracefully
            tracing::info!("{}: shutting down", std::thread::current().name().unwrap());

            // TODO: handle recv from renderer
            op_state.put(SceneDying(true));

            Default::default()
        }
    };

    op_state.put(mutex_scene_crdt_state);
    let mut ret = Vec::<Vec<u8>>::with_capacity(1);
    if let Some(main_crdt) = op_state.try_take::<SceneMainCrdtFileContent>() {
        ret.push(main_crdt.0);
    }
    ret.push(data);
    ret
}
