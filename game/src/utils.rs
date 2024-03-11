use fyrox::{
    core::pool::Handle,
    graph::{BaseSceneGraph, SceneGraph},
    scene::{collider::Collider, graph::Graph, node::Node, sound::Sound},
};

pub fn has_ground_contact(collider: Handle<Node>, graph: &Graph) -> bool {
    if let Some(collider) = graph.try_get(collider).and_then(|n| n.cast::<Collider>()) {
        for contact in collider.contacts(&graph.physics) {
            for manifold in contact.manifolds.iter() {
                if manifold.local_n1.y.abs() > 0.7 || manifold.local_n2.y.abs() > 0.7 {
                    return true;
                }
            }
        }
    }
    false
}

pub fn try_play_sound(sound: Handle<Node>, graph: &mut Graph) {
    if let Some(sound) = graph.try_get_mut_of_type::<Sound>(sound) {
        sound.try_play();
    }
}
