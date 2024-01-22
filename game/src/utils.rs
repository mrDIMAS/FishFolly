use fyrox::{
    core::pool::Handle,
    scene::{collider::Collider, graph::Graph, node::Node},
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
