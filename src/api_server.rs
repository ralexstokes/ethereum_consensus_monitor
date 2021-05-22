use crate::chain::Coordinate;
use crate::config::Spec;
use crate::fork_choice::ForkChoice;
use crate::node::Node;
use futures::future;
use serde::Serialize;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use warp::Filter;

#[derive(Serialize)]
struct NodeResponse {
    id: Option<u64>,
    head: Option<Coordinate>,
}

pub struct ApiServer {
    web_dir: PathBuf,
    nodes: Vec<Node>,
    fork_choice_handle: ForkChoice,
    spec: Spec,
}

impl ApiServer {
    pub fn new<P>(web_dir: P, nodes: Vec<Node>, fork_choice_handle: ForkChoice, spec: Spec) -> Self
    where
        P: AsRef<Path>,
    {
        Self {
            web_dir: web_dir.as_ref().to_path_buf(),
            nodes,
            fork_choice_handle,
            spec,
        }
    }

    pub async fn run(&self, addr: impl Into<SocketAddr>) {
        let app_html = self.web_dir.join("index.html");
        let app = warp::get()
            .and(warp::path::end())
            .and(warp::fs::file(app_html));

        let spec = self.spec.clone();
        let spec = warp::get()
            .and(warp::path("spec"))
            .and(warp::any().map(move || spec.clone()))
            .and_then(serve_spec);

        let nodes_handle = self.nodes.clone();
        let nodes = warp::get()
            .and(warp::path("nodes"))
            .and(warp::path::end())
            .and(warp::any().map(move || nodes_handle.clone()))
            .and_then(get_nodes);

        let fork_choice_handle = self.fork_choice_handle.clone();
        let fork_choice = warp::get()
            .and(warp::path("fork-choice"))
            .and(warp::path::end())
            .and(warp::any().map(move || fork_choice_handle.clone()))
            .and_then(get_fork_choice);

        let routes = app.or(nodes).or(spec).or(fork_choice);
        warp::serve(routes).run(addr).await
    }
}

async fn get_nodes(nodes: Vec<Node>) -> Result<impl warp::Reply, warp::Rejection> {
    let reads = nodes.iter().map(|node| async move {
        let node = node.read().await;
        NodeResponse {
            id: node.id,
            head: node.head,
        }
    });
    let nodes = future::join_all(reads).await;
    Ok(warp::reply::json(&nodes))
}

async fn serve_spec(spec: Spec) -> Result<impl warp::Reply, warp::Rejection> {
    Ok(warp::reply::json(&spec))
}

async fn get_fork_choice(fork_choice: ForkChoice) -> Result<impl warp::Reply, warp::Rejection> {
    let tree = fork_choice.tree.read().expect("has data");
    Ok(warp::reply::json(&*tree))
}
