#![cfg(test)]

use cosmwasm_std::testing::{MockApi, MockStorage};
use cosmwasm_std::{OwnedDeps, Querier};
use osmo_bindings::OsmosisQuery;
use osmo_bindings_test::OsmosisApp;
use std::marker::PhantomData;

fn mock_dependencies_with_custom_quierier<Q: Querier>(
    querier: Q,
) -> OwnedDeps<MockStorage, MockApi, Q, OsmosisQuery> {
    OwnedDeps {
        storage: MockStorage::default(),
        api: MockApi::default(),
        querier,
        custom_query_type: PhantomData,
    }
}

pub fn mock_dependencies() -> OwnedDeps<MockStorage, MockApi, OsmosisApp, OsmosisQuery> {
    let custom_querier = OsmosisApp::new();
    mock_dependencies_with_custom_quierier(custom_querier)
}
