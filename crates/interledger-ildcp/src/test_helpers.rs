use super::*;
use futures::future::err;
use interledger_service::{incoming_service_fn, Account, IncomingService, Username};

use interledger_packet::{Address, ErrorCode, RejectBuilder};
use interledger_ccp::{CcpRoutingAccount, RoutingRelation};

use lazy_static::lazy_static;
use std::str::FromStr;

lazy_static! {
    pub static ref USERNAME_ACC: TestAccount = TestAccount::new(0, "ausername", None, RoutingRelation::Child);
    pub static ref ILPADDR_ACC: TestAccount =
        TestAccount::new(0, "anotherusername", Some("example.account"), RoutingRelation::Peer);
    pub static ref SERVICE_ADDRESS: Address = Address::from_str("example.connector").unwrap();
}

#[derive(Debug, Clone)]
pub struct TestAccount {
    pub id: u64,
    pub username: Username,
    pub ilp_address: Address,
    pub routing_relation: RoutingRelation,
}

impl Account for TestAccount {
    type AccountId = u64;

    fn id(&self) -> u64 {
        self.id
    }

    fn username(&self) -> &Username {
        &self.username
    }

    fn asset_code(&self) -> &str {
        "XYZ"
    }

    fn asset_scale(&self) -> u8 {
        9
    }

    fn client_address(&self) -> &Address {
        &self.ilp_address
    }
}

impl CcpRoutingAccount for TestAccount {
    fn routing_relation(&self) -> RoutingRelation {
        self.routing_relation
    }
}

// Test Service

impl TestAccount {
    pub fn new(id: u64, username: &str, ilp_address: Option<&str>, routing_relation: RoutingRelation) -> Self {
        // During account creation, a user should not be obliged to provide an
        // ILP Address. The Store should be able to generate an ILP Address
        // based on the provided username and the node's iLP Address.
        let ilp_address = if routing_relation == RoutingRelation::Child {
            SERVICE_ADDRESS.clone().with_suffix(username.as_ref()).unwrap()
        } else {
            Address::from_str(ilp_address.unwrap()).unwrap()
        };
        Self {
            id,
            username: Username::from_str(username).unwrap(),
            ilp_address,
            routing_relation,
        }
    }
}

pub fn test_service() -> IldcpService<impl IncomingService<TestAccount> + Clone, TestAccount> {
    IldcpService::new(
        SERVICE_ADDRESS.clone(),
        incoming_service_fn(|_request| {
            Box::new(err(RejectBuilder {
                code: ErrorCode::F02_UNREACHABLE,
                message: b"No other incoming handler!",
                data: &[],
                triggered_by: Some(&SERVICE_ADDRESS),
            }
            .build()))
        }),
    )
}
