use super::packet::*;
use futures::future::ok;
use interledger_ccp::{CcpRoutingAccount, RoutingRelation};
use interledger_packet::*;
use interledger_service::*;
use log::debug;
use std::marker::PhantomData;

/// A simple service that intercepts incoming ILDCP requests
/// and responds using the information in the Account struct.
#[derive(Clone)]
pub struct IldcpService<I, A> {
    ilp_address: Address,
    next: I,
    account_type: PhantomData<A>,
}

impl<I, A> IldcpService<I, A>
where
    I: IncomingService<A>,
    A: CcpRoutingAccount + Account,
{
    pub fn new(ilp_address: Address, next: I) -> Self {
        IldcpService {
            ilp_address,
            next,
            account_type: PhantomData,
        }
    }
}

impl<I, A> IncomingService<A> for IldcpService<I, A>
where
    I: IncomingService<A>,
    A: CcpRoutingAccount + Account,
{
    type Future = BoxedIlpFuture;

    fn handle_request(&mut self, request: IncomingRequest<A>) -> Self::Future {
        if is_ildcp_request(&request.prepare) {
            let from = if request.from.routing_relation() == RoutingRelation::Child {
                self.ilp_address
                    .with_suffix(request.from.username().as_bytes())
                    .unwrap()
            } else {
                request.from.client_address().clone()
            };
            let builder = IldcpResponseBuilder {
                client_address: &from,
                asset_code: request.from.asset_code(),
                asset_scale: request.from.asset_scale(),
            };
            debug!("Responding to query for ildcp info by account: {:?}", from);
            let response = builder.build();
            let fulfill = Fulfill::from(response);
            Box::new(ok(fulfill))
        } else {
            Box::new(self.next.handle_request(request))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::*;
    use bytes::Bytes;
    use futures::future::Future;
    use std::convert::TryFrom;
    use std::str::FromStr;
    use std::time::SystemTime;
    use crate::packet::{ILDCP_DESTINATION, PEER_PROTOCOL_CONDITION};

    #[test]
    fn appends_username() {
        let mut service = test_service();
        let result = service
            .handle_request(IncomingRequest {
                from: USERNAME_ACC.clone(), // account without an ILP Address configured, implicitly considered a Child
                prepare: PrepareBuilder {
                    destination: ILDCP_DESTINATION.clone(),
                    amount: 100,
                    execution_condition: &PEER_PROTOCOL_CONDITION,
                    expires_at: SystemTime::UNIX_EPOCH,
                    data: &[],
                }
                .build(),
            })
            .wait();

        let fulfill: Fulfill = result.unwrap();
        let response = IldcpResponse::try_from(Bytes::from(fulfill.data())).unwrap();
        assert_eq!(
            Address::from_str("example.connector.ausername").unwrap(),
            response.client_address()
        );
    }

    #[test]
    fn overrides_with_ilp_address() {
        let mut service = test_service();
        let result = service
            .handle_request(IncomingRequest {
                from: ILPADDR_ACC.clone(), // Peer account specifies their address
                prepare: PrepareBuilder {
                    destination: ILDCP_DESTINATION.clone(),
                    amount: 100,
                    execution_condition: &PEER_PROTOCOL_CONDITION,
                    expires_at: SystemTime::UNIX_EPOCH,
                    data: &[],
                }
                .build(),
            })
            .wait();

        let fulfill: Fulfill = result.unwrap();
        let response = IldcpResponse::try_from(Bytes::from(fulfill.data())).unwrap();
        assert_eq!(
            Address::from_str("example.account").unwrap(),
            response.client_address()
        );
    }

}