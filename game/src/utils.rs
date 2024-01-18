use fyrox::{
    core::pool::Handle,
    scene::{collider::Collider, graph::Graph, node::Node, rigidbody::RigidBody},
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

pub fn push_up(body: Handle<Node>, collider: Handle<Node>, graph: &mut Graph, amount: f32) {
    if has_ground_contact(collider, graph) {
        if let Some(rigid_body) = graph[body].cast_mut::<RigidBody>() {
            let mut velocity = rigid_body.lin_vel();
            velocity.y += amount;
            rigid_body.set_lin_vel(velocity);
        }
    }
}
