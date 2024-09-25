use std::collections::HashMap;
use std::error::Error;
use std::sync::Arc;

use tonic::{Request, Response, Status};
use tokio::sync::Mutex;
use tokio::sync::mpsc::{self, Sender};
use tokio_stream::wrappers::ReceiverStream;

use crate::clean;
use crate::event::ServerEventSender;
use crate::types::{
    ClientResponse, EventRegister, HostInfo, JoinInfo, SessionData, SessionID,
    SessionType, UserID,
};

pub fn make_server(server: impl Clean)
        -> clean::clean_server::CleanServer<CleanServer> {
    let s = CleanServer::new(server);
    clean::clean_server::CleanServer::new(s)
}

pub struct CleanServer {
    server: Box<dyn Clean>,
    channels: Arc<Mutex<HashMap<EventRegister, Sender<ClientResponse>>>>,
}

impl CleanServer {
    pub fn new(server: impl Clean) -> Self {
        Self {
            server: Box::new(server),
            channels: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

#[tonic::async_trait]
pub trait Clean: Send + Sync + 'static {
    // client initiated API
    async fn host_session(&self, typ: SessionType) -> Result<SessionData, Box<dyn Error>>;
    async fn list_sessions(&self) -> Result<Vec<SessionData>, Box<dyn Error>>;
    async fn join_session(&self, sid: SessionID, uid: UserID, user_name: &str)
        -> Result<(), Box<dyn Error>>;
    // server callbacks
    async fn register_server_event_sender(&self, sid: SessionID, uid: UserID,
                          s: ServerEventSender) -> Result<(), Box<dyn Error>>;
}

#[tonic::async_trait]
impl clean::clean_server::Clean for CleanServer {
    // client initiated API
    async fn host_session(&self, request: Request<clean::HostInfo>)
            -> Result<Response<clean::SessionData>, Status> {
        let hi: HostInfo = request.into_inner().try_into()
            .map_err(|e| Status::internal(&format!("{}", e)))?;
        let c = self.server.host_session(hi.session_type()).await
            .map_err(|e| Status::internal(&format!("{}", e)))?;
        let reply = c.into();
        Ok(Response::new(reply))
    }
    async fn list_sessions(&self, _: Request<clean::Empty>)
            -> Result<Response<clean::Sessions>, Status> {
        let c = self.server.list_sessions().await
            .map_err(|e| Status::internal(&format!("{}", e)))?;
        let v: Vec<clean::SessionData> = c.iter().map(|sd| sd.clone().into()).collect();
        Ok(Response::new(
            clean::Sessions {
                data: v,
            }
        ))
    }
    async fn join_session(&self, request: Request<clean::JoinInfo>)
            -> Result<Response<clean::Empty>, Status> {
        let ji: JoinInfo = request.into_inner().into();
        self.server.join_session(ji.session_id(), ji.user_id(), ji.user_name()).await
            .map_err(|e| Status::internal(&format!("{}", e)))?;
        Ok(Response::new(clean::Empty{}))
    }
    // server callbacks
    type ServerEventsStream = ReceiverStream<Result<clean::ServerRequest, Status>>;
    async fn server_events(&self, request: Request<clean::EventRegister>)
            -> Result<Response<Self::ServerEventsStream>, Status> {
        // outer channel to return message to the client
        let (tx, rx) = mpsc::channel(100);

        // inner channel to pass values from the server implementation
        let (ctx, mut crx) = mpsc::channel(100);

        let er: EventRegister = request.into_inner().into();

        // a responder channel to respond to a server event
        let (rtx, rrx) = mpsc::channel(100);

        // store the transmitter to send messages back to the client
        self.channels.lock().await.insert(er.clone(), rtx);

        // give the server an event sender so it can send message to the client
        self.server.register_server_event_sender(er.session_id(), er.user_id(),
            ServerEventSender::new(ctx, rrx)).await
            .map_err(|e| Status::internal(&format!("{}", e)))?;

        // listen for messages from the server
        // and send them to the client
        tokio::spawn(async move {
            loop {
                if let Some(se) = crx.recv().await {
                    let s: clean::ServerRequest = se.into();
                    if let Err(e) = tx.send(Ok(s)).await {
                        error!("Could not send server event to client: {:?}", e);
                    }
                } else {
                    info!("Server shutting down");
                    break;
                }
            }
        });
        Ok(Response::new(ReceiverStream::new(rx)))
    }

    async fn respond_to_server_event(&self, request: Request<clean::ClientEventResponse>)
            -> Result<Response<clean::Empty>, Status> {
        let inner = request.into_inner();
        let er = inner.er
            .ok_or_else(|| Status::internal("Invalid response"))?.into();
        let i: clean::ClientResponse = inner.client_response
            .ok_or_else(|| Status::internal("Invalid response"))?.into();
        let cr: ClientResponse = i.try_into()
            .map_err(|e| Status::internal(&format!("{}", e)))?;
        // send this response to the waiting server event sender
        self.channels.lock().await.get(&er)
            .ok_or_else(|| Status::internal("Invalid response"))?
            .send(cr).await
            .map_err(|e| Status::internal(&format!("{}", e)))?;
        Ok(Response::new(clean::Empty{}))
    }
}
