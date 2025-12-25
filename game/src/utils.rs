use fyrox::plugin::error::{GameError, GameResult};
use fyrox::{
    core::pool::Handle,
    graph::SceneGraph,
    scene::{collider::Collider, graph::Graph, node::Node, sound::Sound},
};

pub fn has_ground_contact(collider: Handle<Node>, graph: &Graph) -> Result<bool, GameError> {
    let collider = graph.try_get_of_type::<Collider>(collider)?;
    for contact in collider.contacts(&graph.physics) {
        for manifold in contact.manifolds.iter() {
            if manifold.local_n1.y.abs() > 0.7 || manifold.local_n2.y.abs() > 0.7 {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

pub fn try_play_sound(sound: Handle<Node>, graph: &mut Graph) -> GameResult {
    let sound = graph.try_get_mut_of_type::<Sound>(sound)?;
    sound.try_play();
    Ok(())
}
