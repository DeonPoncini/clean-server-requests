use std::sync::Arc;

use futures_util::TryFutureExt;
use tonic::Request;
use tonic::transport::{Channel, Uri};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use crate::clean;
use crate::error::Error;
use crate::event::ServerEvent;
use crate::types::Result;
use crate::types::{
    CoinGuess, DiceGuess, EventRegister, FlipCoin, JoinInfo, HostInfo, Ping, Pong,
    RollDice, Sessions, SessionData, SessionID, SessionType, StartInfo, UserID, Winner,
};

pub struct CleanClient {
    client: clean::clean_client::CleanClient<Channel>,
}

impl CleanClient {
    pub async fn new(address: &str) -> Result<Self> {
        let uri = address.parse::<Uri>()?;
        let client = clean::clean_client::CleanClient::connect(uri).await?;
        Ok(Self {
            client: client,
        })
    }

    // client drive API
    pub async fn host_session(&mut self, typ: SessionType, player_count: u8)
            -> Result<SessionData> {
        let hi = HostInfo::new(typ, player_count);
        let request = Request::new(hi.into());
        let response = self.client.host_session(request).await?;
        Ok(response.into_inner().try_into()?)
    }

    pub async fn list_sessions(&mut self) -> Result<Vec<SessionData>> {
        let request = Request::new(clean::Empty{});
        let response = self.client.list_sessions(request).await?;
        let s: Sessions = response.into_inner().try_into()?;
        Ok(s.sessions().to_vec())
    }

    pub async fn join_session(&mut self, sid: SessionID, uid: UserID,
                              user_name: &str) -> Result<()> {
        let ji = JoinInfo::new(sid, uid, user_name);
        let request = Request::new(ji.into());
        let _ = self.client.join_session(request).await?;
        Ok(())
    }

    pub async fn start_session(&mut self, sid: SessionID) -> Result<()> {
        let si = StartInfo::new(sid);
        let request = Request::new(si.into());
        let _ = self.client.start_session(request).await?;
        Ok(())
    }

    // listen for server events
    pub async fn server_events_listen(&mut self, sid: SessionID, uid: UserID,
            listener: Arc<dyn ServerEvent>) -> Result<JoinHandle<Result<()>>> {
        let (tx, mut rx) = mpsc::channel::<(clean::server_request::Msg, EventRegister)>(100);
        let mut client_clone = self.client.clone();

        let handle: JoinHandle<Result<()>> = tokio::spawn(async move {
            let mut error = None;
            while let Some((event, er)) = rx.recv().await {
                let server_el = Arc::clone(&listener);
                let cr = match server_listener_handler(server_el, event).await {
                    Ok(i) => i,
                    Err(e) => {
                       let err = format!("{:?}", e);
                       error = Some(err.clone());
                       Some(clean::client_response::Msg::Error(err))
                    }
                };

                if let Some(c) = cr {
                    info!("Responding with {:?} for user {:?}", c, uid);
                    let cm = clean::ClientResponse {
                        msg: Some(c),
                    };
                    let cer = clean::ClientEventResponse {
                        er: Some(er.clone().into()),
                        client_response: Some(cm),
                    };
                    let r = Request::new(cer);
                    if let Err(e) = client_clone.respond_to_server_event(r).await {
                        error!("Failed to respond to server event: {:?}", e);
                        error = Some(format!("{:?}", e));
                        break;
                    }
                }
            }
            match error {
                Some(e) => { return Err(Box::new(Error::ClientError(e))); }
                None => { return Ok(()); }
            }
        }.map_err(|e| Box::new(e) as
                  Box<dyn std::error::Error + Send + Sync + 'static>));

        let er = EventRegister::new(sid, uid);
        let request = Request::new(er.clone().into());
        let mut stream = self.client.server_events(request).await?.into_inner();

        tokio::spawn(async move {
            while let Ok(Some(event)) = stream.message().await {
                if let Some(sr) = event.msg {
                    if let Err(e) = tx.send((sr, er.clone())).await {
                        error!("Failed to send server event: {:?}", e);
                        break;
                    }
                }
            }
        });

        Ok(handle)
    }
}

async fn server_listener_handler(server_el: Arc<dyn ServerEvent>,
                 msg: clean::server_request::Msg)
    -> Result<Option<clean::client_response::Msg>>{
    match msg {
        clean::server_request::Msg::UserJoined(ji) => {
            let ji: JoinInfo = ji.into();
            server_el.join_info(ji.session_id().into(), ji.user_id().into(),
                                ji.user_name()).await?;
            return Ok(None);
        }
        clean::server_request::Msg::Ping(p) => {
            let ping: Ping = p.into();
            let r = server_el.ping(ping.text()).await?;
            let pong = Pong::new(&r);
            return Ok(Some(clean::client_response::Msg::Pong(pong.into())));
        }
        clean::server_request::Msg::Dice(rd) => {
            let rd: RollDice = rd.into();
            let r = server_el.roll_dice(rd.sides(), rd.count()).await?;
            let dg = DiceGuess::new(&r);
            return Ok(Some(clean::client_response::Msg::DiceGuess(dg.into())));
        }
        clean::server_request::Msg::Coin(fc) => {
            let fc: FlipCoin = fc.into();
            let r = server_el.flip_coin(fc.count()).await?;
            let cg = CoinGuess::new(&r);
            return Ok(Some(clean::client_response::Msg::CoinGuess(cg.into())));
        }
        clean::server_request::Msg::Winner(w) => {
            let w: Winner = w.into();
            server_el.winner(w.user_id(), w.user_name()).await?;
            return Ok(None);
        }
        clean::server_request::Msg::TryAgain(_) => {
            let r = server_el.try_again().await?;
            return Ok(Some(clean::client_response::Msg::Again(r)));
        }
        clean::server_request::Msg::Error(e) => {
            server_el.error(&e).await?;
            return Ok(None);
        }
    }
}
