use crate::compare::{compare, CompareResult};
use crate::model::Direction;
use crate::schema::{connect, fetch_schema};
use anyhow::{Context, Result};
use std::sync::mpsc::Sender;
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};

pub enum Which {
    Source,
    Target,
}

pub enum Cmd {
    Test { which: Which, cs: String },
    Compare { src_cs: String, tgt_cs: String },
}

pub enum Reply {
    Connected { which_is_source: bool, ok: bool, msg: String },
    Result(Box<CompareResult>),
    Failed(String),
    Idle,
}

/// Spawn tokio thread + command loop; returns cmd sender.
pub fn start(reply_tx: Sender<Reply>) -> UnboundedSender<Cmd> {
    let (cmd_tx, mut cmd_rx) = unbounded_channel::<Cmd>();
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().expect("tokio");
        rt.block_on(async move {
            while let Some(cmd) = cmd_rx.recv().await {
                match cmd {
                    Cmd::Test { which, cs } => {
                        let r = connect(&cs).await.map(|_| ());
                        let _ = reply_tx.send(Reply::Connected {
                            which_is_source: matches!(which, Which::Source),
                            ok: r.is_ok(),
                            msg: r.err().map(|e| format!("{e:#}")).unwrap_or_default(),
                        });
                    }
                    Cmd::Compare { src_cs, tgt_cs } => {
                        let r = run_compare(&src_cs, &tgt_cs).await;
                        let _ = reply_tx.send(match r {
                            Ok(res) => Reply::Result(Box::new(res)),
                            Err(e) => Reply::Failed(format!("{e:#}")),
                        });
                    }
                }
            }
        });
    });
    cmd_tx
}

async fn run_compare(src_cs: &str, tgt_cs: &str) -> Result<CompareResult> {
    let mut a = connect(src_cs).await.context("source connect")?;
    let mut b = connect(tgt_cs).await.context("target connect")?;
    let s = fetch_schema(&mut a).await.context("source schema fetch")?;
    let t = fetch_schema(&mut b).await.context("target schema fetch")?;
    Ok(compare(&s, &t, Direction::SourceToTarget))
}
