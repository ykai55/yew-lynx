use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use axum::Router;
use axum::body::{Body, Bytes};
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Request, State};
use axum::http::header::{CACHE_CONTROL, CONTENT_TYPE};
use axum::http::{HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use notify::event::ModifyKind;
use notify::{Event, EventKind, RecursiveMode, Watcher};
use serde::Serialize;
use sha2::{Digest, Sha256};
use tokio::process::Command;
use tokio::sync::{RwLock, mpsc, watch};

const RELOAD_PATH: &str = "/.well-known/yew-lynx/reload";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Backend {
    Yew,
    Dioxus,
    All,
}

impl Backend {
    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "yew" => Ok(Self::Yew),
            "dioxus" => Ok(Self::Dioxus),
            "all" => Ok(Self::All),
            _ => Err(format!("unsupported backend: {value}")),
        }
    }
}

#[derive(Debug)]
struct Arguments {
    backend: Backend,
    bind: IpAddr,
    port: u16,
}

impl Arguments {
    fn parse() -> Result<Option<Self>, String> {
        let mut backend = Backend::All;
        let mut bind = IpAddr::V4(Ipv4Addr::UNSPECIFIED);
        let mut port = 8000;
        let mut arguments = env::args().skip(1);
        while let Some(argument) = arguments.next() {
            match argument.as_str() {
                "--backend" => {
                    backend = Backend::parse(
                        &arguments
                            .next()
                            .ok_or_else(|| "--backend requires a value".to_owned())?,
                    )?;
                }
                "--bind" => {
                    bind = arguments
                        .next()
                        .ok_or_else(|| "--bind requires a value".to_owned())?
                        .parse()
                        .map_err(|error| format!("invalid bind address: {error}"))?;
                }
                "--port" => {
                    port = arguments
                        .next()
                        .ok_or_else(|| "--port requires a value".to_owned())?
                        .parse()
                        .map_err(|error| format!("invalid port: {error}"))?;
                }
                "-h" | "--help" => return Ok(None),
                _ => return Err(format!("unknown argument: {argument}")),
            }
        }
        if port == 0 {
            return Err("port must be between 1 and 65535".to_owned());
        }
        Ok(Some(Self {
            backend,
            bind,
            port,
        }))
    }
}

#[derive(Clone, Debug)]
struct BuildPlan {
    packages: Vec<&'static str>,
    artifacts: Vec<(&'static str, &'static str)>,
    watch_paths: Vec<PathBuf>,
}

impl BuildPlan {
    fn new(root: &Path, backend: Backend) -> Self {
        let mut watch_paths = vec![
            root.join("Cargo.toml"),
            root.join("Cargo.lock"),
            root.join("crates/element-bridge-core"),
            root.join("crates/element-bridge-wasm-guest"),
            root.join("crates/lynx"),
        ];
        let (packages, artifacts) = match backend {
            Backend::Yew => {
                watch_paths.extend([
                    root.join("examples/counter"),
                    root.join("adapters/yew"),
                    root.join("runtimes/yew"),
                    root.join(".deps/yew/packages/yew"),
                ]);
                (
                    vec!["yew-lynx-counter"],
                    vec![("/yew_lynx_counter.wasm", "yew_lynx_counter.wasm")],
                )
            }
            Backend::Dioxus => {
                watch_paths.extend([
                    root.join("examples/dioxus-counter"),
                    root.join("adapters/dioxus"),
                    root.join("runtimes/dioxus"),
                ]);
                (
                    vec!["lynx-element-bridge-dioxus-counter"],
                    vec![(
                        "/lynx_element_bridge_dioxus_counter.wasm",
                        "lynx_element_bridge_dioxus_counter.wasm",
                    )],
                )
            }
            Backend::All => {
                watch_paths.extend([
                    root.join("examples/counter"),
                    root.join("examples/dioxus-counter"),
                    root.join("adapters/yew"),
                    root.join("adapters/dioxus"),
                    root.join("runtimes/yew"),
                    root.join("runtimes/dioxus"),
                    root.join(".deps/yew/packages/yew"),
                ]);
                (
                    vec!["yew-lynx-counter", "lynx-element-bridge-dioxus-counter"],
                    vec![
                        ("/yew_lynx_counter.wasm", "yew_lynx_counter.wasm"),
                        (
                            "/lynx_element_bridge_dioxus_counter.wasm",
                            "lynx_element_bridge_dioxus_counter.wasm",
                        ),
                    ],
                )
            }
        };
        Self {
            packages,
            artifacts,
            watch_paths,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ReloadState {
    v: u8,
    generation: u64,
    artifacts: Vec<ArtifactState>,
}

#[derive(Clone, Debug, Serialize)]
struct ArtifactState {
    path: &'static str,
    sha256: String,
    size: u64,
}

#[derive(Clone)]
struct ServerState {
    files: Arc<RwLock<HashMap<&'static str, Bytes>>>,
    reload: watch::Sender<ReloadState>,
}

struct PublishedBuild {
    files: HashMap<&'static str, Bytes>,
    reload: ReloadState,
}

#[cfg(unix)]
struct ProcessGroup(Option<i32>);

#[cfg(unix)]
impl ProcessGroup {
    fn disarm(&mut self) {
        self.0 = None;
    }
}

#[cfg(unix)]
impl Drop for ProcessGroup {
    fn drop(&mut self) {
        if let Some(process_group) = self.0 {
            // SAFETY: Cargo was spawned as this process-group leader, so a negative PID targets
            // that group rather than an unrelated process.
            unsafe {
                libc::kill(-process_group, libc::SIGKILL);
            }
        }
    }
}

async fn websocket_handler(
    State(state): State<ServerState>,
    websocket: WebSocketUpgrade,
) -> Response {
    websocket.on_upgrade(move |socket| send_reload_states(socket, state.reload.subscribe()))
}

async fn send_reload_states(mut socket: WebSocket, mut reload: watch::Receiver<ReloadState>) {
    if send_reload_state(&mut socket, &mut reload).await.is_err() {
        return;
    }
    loop {
        tokio::select! {
            changed = reload.changed() => {
                if changed.is_err() || send_reload_state(&mut socket, &mut reload).await.is_err() {
                    return;
                }
            }
            incoming = socket.recv() => {
                match incoming {
                    Some(Ok(Message::Ping(payload))) => {
                        if socket.send(Message::Pong(payload)).await.is_err() {
                            return;
                        }
                    }
                    Some(Ok(Message::Close(_))) | Some(Err(_)) | None => return,
                    Some(Ok(_)) => {}
                }
            }
        }
    }
}

async fn send_reload_state(
    socket: &mut WebSocket,
    reload: &mut watch::Receiver<ReloadState>,
) -> Result<(), ()> {
    let message = serde_json::to_string(&*reload.borrow_and_update()).map_err(|error| {
        eprintln!("failed to encode reload state: {error}");
    })?;
    socket
        .send(Message::Text(message.into()))
        .await
        .map_err(|_| ())
}

async fn artifact_handler(State(state): State<ServerState>, request: Request) -> Response {
    let file = state.files.read().await.get(request.uri().path()).cloned();
    let Some(bytes) = file else {
        return StatusCode::NOT_FOUND.into_response();
    };
    Response::builder()
        .status(StatusCode::OK)
        .header(CONTENT_TYPE, HeaderValue::from_static("application/wasm"))
        .header(CACHE_CONTROL, HeaderValue::from_static("no-store"))
        .body(Body::from(bytes))
        .expect("static response headers must be valid")
}

async fn build(root: &Path, plan: &BuildPlan) -> Result<(), Box<dyn Error>> {
    println!("Building WASM guest(s)");
    let mut command = Command::new("cargo");
    command
        .current_dir(root)
        .args([
            "build",
            "--manifest-path",
            root.join("Cargo.toml")
                .to_str()
                .ok_or("non-UTF-8 root path")?,
            "--locked",
            "--release",
            "--target",
            "wasm32-wasip1",
            "--target-dir",
            root.join("target").to_str().ok_or("non-UTF-8 root path")?,
        ])
        .kill_on_drop(true);
    for package in &plan.packages {
        command.args(["--package", package]);
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;

        command.as_std_mut().process_group(0);
    }
    let mut child = command.spawn()?;
    #[cfg(unix)]
    let mut process_group =
        ProcessGroup(Some(child.id().ok_or("Cargo process has no PID")? as i32));
    let status = child.wait().await?;
    #[cfg(unix)]
    process_group.disarm();
    if !status.success() {
        return Err(format!("Cargo exited with {status}").into());
    }
    Ok(())
}

async fn inspect_build(
    output: &Path,
    plan: &BuildPlan,
    generation: u64,
) -> Result<PublishedBuild, Box<dyn Error>> {
    let mut artifacts = Vec::with_capacity(plan.artifacts.len());
    let mut files = HashMap::with_capacity(plan.artifacts.len());
    for (path, file_name) in &plan.artifacts {
        let bytes = tokio::fs::read(output.join(file_name)).await?;
        artifacts.push(ArtifactState {
            path,
            sha256: format!("{:x}", Sha256::digest(&bytes)),
            size: bytes.len() as u64,
        });
        files.insert(*path, Bytes::from(bytes));
    }
    Ok(PublishedBuild {
        files,
        reload: ReloadState {
            v: 1,
            generation,
            artifacts,
        },
    })
}

fn usage() {
    println!(
        "Usage: cargo run --locked -p yew-lynx-dev-server -- \
         [--backend yew|dioxus|all] [--bind IP] [--port PORT]"
    );
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("dev server crate must be under tools/dev-wasm")
        .to_owned()
}

fn is_source_change(kind: &EventKind) -> bool {
    matches!(
        kind,
        EventKind::Any
            | EventKind::Create(_)
            | EventKind::Modify(
                ModifyKind::Any | ModifyKind::Data(_) | ModifyKind::Name(_) | ModifyKind::Other
            )
            | EventKind::Remove(_)
    )
}

fn start_watcher(
    plan: &BuildPlan,
) -> Result<(notify::RecommendedWatcher, mpsc::Receiver<Event>), notify::Error> {
    let (events, event_receiver) = mpsc::channel::<Event>(1);
    let mut watcher =
        notify::recommended_watcher(move |result: notify::Result<Event>| match result {
            Ok(event) if is_source_change(&event.kind) => {
                let _ = events.try_send(event);
            }
            Ok(_) => {}
            Err(error) => eprintln!("watch error: {error}"),
        })?;
    println!("Watching:");
    for path in &plan.watch_paths {
        println!("  {}", path.display());
        watcher.watch(
            path,
            if path.is_dir() {
                RecursiveMode::Recursive
            } else {
                RecursiveMode::NonRecursive
            },
        )?;
    }
    Ok((watcher, event_receiver))
}

async fn publish(state: &ServerState, next: PublishedBuild) {
    *state.files.write().await = next.files;
    state.reload.send_replace(next.reload);
}

async fn shutdown_signal() {
    #[cfg(unix)]
    {
        let mut terminate =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                .expect("SIGTERM handler must be installable");
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {}
            _ = terminate.recv() => {}
        }
    }
    #[cfg(not(unix))]
    tokio::signal::ctrl_c()
        .await
        .expect("Ctrl-C handler must be installable");
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let arguments = match Arguments::parse() {
        Ok(Some(arguments)) => arguments,
        Ok(None) => {
            usage();
            return Ok(());
        }
        Err(error) => {
            eprintln!("{error}");
            usage();
            return Err(error.into());
        }
    };
    let root = workspace_root();
    let output = root.join("target/wasm32-wasip1/release");
    let plan = BuildPlan::new(&root, arguments.backend);
    for path in &plan.watch_paths {
        if !path.exists() {
            return Err(format!("watch path does not exist: {}", path.display()).into());
        }
    }

    let (_watcher, mut event_receiver) = start_watcher(&plan)?;
    loop {
        tokio::select! {
            result = build(&root, &plan) => result?,
            _ = shutdown_signal() => return Ok(()),
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
        if event_receiver.try_recv().is_err() {
            break;
        }
        while event_receiver.try_recv().is_ok() {}
        println!("Source changed during the initial build; rebuilding");
    }
    let initial = inspect_build(&output, &plan, 1).await?;
    let (reload, _) = watch::channel(initial.reload);
    let state = ServerState {
        files: Arc::new(RwLock::new(initial.files)),
        reload,
    };
    let app = Router::new()
        .route(RELOAD_PATH, get(websocket_handler))
        .fallback(artifact_handler)
        .with_state(state.clone());
    let address = SocketAddr::new(arguments.bind, arguments.port);
    let listener = tokio::net::TcpListener::bind(address).await?;
    println!("Serving on http://{address}");
    for (path, _) in &plan.artifacts {
        println!("  http://127.0.0.1:{}{path}", arguments.port);
    }
    println!(
        "WebSocket reload endpoint: ws://127.0.0.1:{}{RELOAD_PATH}",
        arguments.port
    );
    println!(
        "For a USB-connected Android device, run: adb reverse tcp:{} tcp:{}",
        arguments.port, arguments.port
    );

    let mut server = tokio::spawn(async move { axum::serve(listener, app).await });
    let mut generation = 1;
    'running: loop {
        tokio::select! {
            _ = shutdown_signal() => break,
            result = &mut server => {
                return Err(format!("HTTP server stopped unexpectedly: {result:?}").into());
            }
            event = event_receiver.recv() => {
                let Some(event) = event else { break };
                println!("Paths updated: {:?}", event.paths);
                loop {
                    tokio::time::sleep(Duration::from_millis(200)).await;
                    while event_receiver.try_recv().is_ok() {}
                    let result = tokio::select! {
                        result = build(&root, &plan) => result,
                        _ = shutdown_signal() => break 'running,
                        server_result = &mut server => {
                            return Err(format!("HTTP server stopped unexpectedly: {server_result:?}").into());
                        }
                    };
                    match result {
                        Ok(()) => {
                            tokio::time::sleep(Duration::from_millis(50)).await;
                            if event_receiver.try_recv().is_ok() {
                                while event_receiver.try_recv().is_ok() {}
                                println!("Source changed during the build; discarding it and rebuilding");
                                continue;
                            }
                            generation += 1;
                            match inspect_build(&output, &plan, generation).await {
                                Ok(next) => publish(&state, next).await,
                                Err(error) => eprintln!("failed to inspect build output: {error}"),
                            }
                        }
                        Err(error) => eprintln!("build failed; keeping the previous guest: {error}"),
                    }
                    break;
                }
            }
        }
    }
    server.abort();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;
    use notify::event::{AccessKind, DataChange};

    #[test]
    fn yew_plan_only_serves_and_watches_yew() {
        let plan = BuildPlan::new(Path::new("/repo"), Backend::Yew);
        assert_eq!(plan.packages, ["yew-lynx-counter"]);
        assert_eq!(plan.artifacts[0].0, "/yew_lynx_counter.wasm");
        assert!(
            plan.watch_paths
                .contains(&PathBuf::from("/repo/examples/counter"))
        );
        assert!(
            plan.watch_paths
                .contains(&PathBuf::from("/repo/runtimes/yew"))
        );
        assert!(
            !plan
                .watch_paths
                .contains(&PathBuf::from("/repo/examples/dioxus-counter"))
        );
    }

    #[test]
    fn dioxus_plan_only_serves_and_watches_dioxus() {
        let plan = BuildPlan::new(Path::new("/repo"), Backend::Dioxus);
        assert_eq!(plan.packages, ["lynx-element-bridge-dioxus-counter"]);
        assert_eq!(
            plan.artifacts[0].0,
            "/lynx_element_bridge_dioxus_counter.wasm"
        );
        assert!(
            plan.watch_paths
                .contains(&PathBuf::from("/repo/examples/dioxus-counter"))
        );
        assert!(
            plan.watch_paths
                .contains(&PathBuf::from("/repo/runtimes/dioxus"))
        );
        assert!(
            !plan
                .watch_paths
                .contains(&PathBuf::from("/repo/examples/counter"))
        );
    }

    #[test]
    fn reload_protocol_is_stable() {
        let json = serde_json::to_value(ReloadState {
            v: 1,
            generation: 7,
            artifacts: vec![ArtifactState {
                path: "/page.wasm",
                sha256: "abc".to_owned(),
                size: 42,
            }],
        })
        .unwrap();
        assert_eq!(json["v"], 1);
        assert_eq!(json["generation"], 7);
        assert_eq!(json["artifacts"][0]["path"], "/page.wasm");
        assert_eq!(json["artifacts"][0]["sha256"], "abc");
        assert_eq!(json["artifacts"][0]["size"], 42);
    }

    #[test]
    fn watcher_ignores_reads_but_accepts_content_changes() {
        assert!(!is_source_change(&EventKind::Access(AccessKind::Any)));
        assert!(is_source_change(&EventKind::Modify(ModifyKind::Data(
            DataChange::Any
        ))));
    }

    #[tokio::test]
    async fn artifact_handler_only_serves_whitelisted_wasm_without_caching() {
        let (reload, _) = watch::channel(ReloadState {
            v: 1,
            generation: 1,
            artifacts: Vec::new(),
        });
        let state = ServerState {
            files: Arc::new(RwLock::new(HashMap::from([(
                "/page.wasm",
                Bytes::from_static(b"wasm"),
            )]))),
            reload,
        };
        let response = artifact_handler(
            State(state.clone()),
            Request::builder()
                .uri("/page.wasm")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()[CONTENT_TYPE], "application/wasm");
        assert_eq!(response.headers()[CACHE_CONTROL], "no-store");
        assert_eq!(
            to_bytes(response.into_body(), 16).await.unwrap(),
            b"wasm"[..]
        );

        let missing = artifact_handler(
            State(state),
            Request::builder()
                .uri("/other.wasm")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(missing.status(), StatusCode::NOT_FOUND);
    }
}
