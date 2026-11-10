// TigerBeetle side of the consumer. A transfer id that already exists is not an error: it is how
// a replayed event shows up, and it moves no money.
use cn_tigerbeetle as tb;
use std::collections::HashMap;
use std::fmt;
use std::time::Duration;
use tokio::time::timeout;

const SEND_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug)]
pub enum LedgerError {
    Transport(String),
    Rejected {
        index: usize,
        result: tb::CreateTransferResult,
    },
}

impl fmt::Display for LedgerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LedgerError::Transport(m) => write!(f, "tigerbeetle is unreachable: {m}"),
            LedgerError::Rejected { index, result } => {
                write!(f, "tigerbeetle rejected transfer {index}: {result:?}")
            }
        }
    }
}

impl std::error::Error for LedgerError {}

/// Sends all transfers in one request.
///
/// TigerBeetle only reports the transfers that didn't come out `Ok`, and it treats a chain as a
/// unit: when one transfer of a chain fails (an `Exists` counts as a failure here), the others in
/// that chain are reported as `LinkedEventFailed`.
///
/// So the result is judged per chain, by its root cause:
/// - `Exists` means the whole chain was applied before (our ids come from the trade, so a
///   replayed chain is identical), and counts as a success.
/// - Any other cause is an error. The error names the transfer that caused it, not the other
///   transfers in its chain.
pub async fn send(client: &tb::Client, transfers: &[tb::Transfer]) -> Result<(), LedgerError> {
    if transfers.is_empty() {
        return Ok(());
    }
    let results = timeout(SEND_TIMEOUT, client.create_transfers(transfers))
        .await
        .map_err(|_| LedgerError::Transport(format!("no answer within {SEND_TIMEOUT:?}")))?
        .map_err(|e| LedgerError::Transport(e.to_string()))?;
    let reported: HashMap<usize, tb::CreateTransferResult> =
        results.iter().map(|r| (r.index, r.result)).collect();

    let mut start = 0;
    while start < transfers.len() {
        let mut end = start;
        while end + 1 < transfers.len() && transfers[end].flags.contains(tb::TransferFlags::Linked)
        {
            end += 1;
        }
        let chain: Vec<(usize, tb::CreateTransferResult)> = (start..=end)
            .filter_map(|i| reported.get(&i).map(|r| (i, *r)))
            .collect();
        let root = chain
            .iter()
            .find(|(_, r)| *r != tb::CreateTransferResult::LinkedEventFailed)
            .or(chain.first());
        match root {
            None | Some((_, tb::CreateTransferResult::Exists)) => {}
            Some((index, result)) => {
                return Err(LedgerError::Rejected {
                    index: *index,
                    result: *result,
                });
            }
        }
        start = end + 1;
    }
    Ok(())
}
