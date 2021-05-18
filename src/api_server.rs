use crate::monitor::Nodes;
use crate::node::Head;
use futures::future;
use serde::Serialize;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use warp::Filter;

#[derive(Serialize)]
struct NodeResponse {
    id: Option<u64>,
    head: Option<Head>,
}

pub struct ApiServer {
    web_dir: PathBuf,
    nodes: Nodes,
}

impl ApiServer {
    pub fn new<P>(web_dir: P, nodes: Nodes) -> Self
    where
        P: AsRef<Path>,
    {
        Self {
            web_dir: web_dir.as_ref().to_path_buf(),
            nodes,
        }
    }

    pub async fn run(&self, addr: impl Into<SocketAddr>) {
        let app_html = self.web_dir.join("index.html");
        let app = warp::get()
            .and(warp::path::end())
            .and(warp::fs::file(app_html));

        let nodes_handle = self.nodes.clone();
        let nodes = warp::get()
            .and(warp::path("nodes"))
            .and(warp::path::end())
            .and(warp::any().map(move || nodes_handle.clone()))
            .and_then(get_nodes);

        let routes = app.or(nodes);
        warp::serve(routes).run(addr).await
    }
}

async fn get_nodes(nodes: Nodes) -> Result<impl warp::Reply, warp::Rejection> {
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
