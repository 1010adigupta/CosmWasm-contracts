use cosmwasm_std::{
    attr, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdError, StdResult, Storage,
};
use cw721::{Cw721QueryMsg, Cw721ReceiveMsg, Cw721ReceiveMsgValue};
use cosmwasm_storage::Map;

#[derive(Default)]
pub struct State {
    contracts: Map<String, Vec<String>>,
    all_collections: Vec<String>,
    owner: String, // Set owner address
}

pub enum HandleMsg {
    CreateCollection {
        deployment_config: ProptixDeploymentConfig,
        runtime_config: ProptixRuntimeConfig,
    },
    SetBaseURI {
        collection: String,
        uri: String,
        status: bool,
    },
    SetWhitelist {
        collection: String,
        user: String,
        status: bool,
    },
}

impl State {
    fn store_collection(&mut self, storage: &mut dyn Storage, owner: &str, collection: &str) -> StdResult<()> {
        let owner_collections: Vec<String> = self.contracts.may_load(owner)?.unwrap_or_default();
        let new_collections = [&owner_collections[..], &[collection.to_string()]].concat();
        self.contracts.save(owner, &new_collections)?;
        self.all_collections.push(collection.to_string());
        Ok(())
    }

    fn get_last_deployed(&self, storage: &dyn Storage, owner: &str) -> StdResult<Option<String>> {
        self.contracts.may_load(owner)
            .map(|collections| collections.last().cloned())
    }

    fn get_all_contracts(&self, _storage: &dyn Storage) -> StdResult<Vec<String>> {
        Ok(self.all_collections.clone())
    }

    fn get_deployed(&self, storage: &dyn Storage, owner: &str) -> StdResult<Vec<String>> {
        self.contracts.may_load(owner)
            .map(|collections| collections.clone())
            .unwrap_or_default()
    }

    impl State {
        fn set_base_uri(&mut self, collection: &str, uri: String, status: bool) -> StdResult<()> {
            // Get the contract's address from the collection name
            let contract_address = self.get_contract_address(collection)?;
    
            // Update the base URI and status for the specified collection
            self.collection_base_uris.save(contract_address.as_bytes(), &uri)?;
            self.collection_uri_statuses.save(contract_address.as_bytes(), &status)?;
    
            Ok(())
        }
    
        fn set_whitelist(&mut self, collection: &str, user: &str, status: bool) -> StdResult<()> {
            // Get the contract's address from the collection name
            let contract_address = self.get_contract_address(collection)?;
    
            // Update the whitelist status for the specified user and collection
            self.whitelisted_users
                .update(&contract_address, |whitelist| match whitelist {
                    Some(mut list) => {
                        if status {
                            list.push(user.to_string());
                        } else {
                            list.retain(|u| u != user);
                        }
                        Some(list)
                    }
                    None => Some(vec![user.to_string()]),
                });
    
            Ok(())
        }
    
        fn get_contract_address(&self, collection: &str) -> StdResult<String> {
            // Try to load the contract address from the mapping
            if let Some(contract_address) = self.collection_addresses.may_load(collection.as_bytes())? {
                Ok(contract_address)
            } else {
                Err(StdError::generic_err("Collection not found"))
            }
        }
    }
    
}


pub fn instantiate(_deps: DepsMut, _env: Env, _info: MessageInfo, _msg: InstantiateMsg) -> Result<Response, StdError> {
    // Initialize state if needed
    Ok(Response::default())
}

pub fn handle(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: HandleMsg,
) -> Result<Response, StdError> {
    match msg {
        HandleMsg::CreateCollection { deployment_config, runtime_config } => {
            let collection_addr = create_collection(deployment_config, runtime_config);
            store_collection(deps.storage, &info.sender, &collection_addr)?;
            Ok(Response::new()
                .add_attribute("action", "create_collection")
                .add_attribute("collection", collection_addr))
        }
        HandleMsg::SetBaseURI { collection, uri, status } => {
            set_base_uri(collection, uri, status)?;
            Ok(Response::new()
                .add_attribute("action", "set_base_uri")
                .add_attribute("collection", collection))
        }
        HandleMsg::SetWhitelist { collection, user, status } => {
            set_whitelist(collection, user, status)?;
            Ok(Response::new()
                .add_attribute("action", "set_whitelist")
                .add_attribute("collection", collection))
        }
    }
}

pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> Result<Binary, StdError> {
    match msg {
        QueryMsg::LastDeployed { owner } => {
            let collection = get_last_deployed(deps.storage, &owner)?;
            let response = QueryResponse { collection };
            let json_response = serde_json::to_string(&response)
                .map_err(|_| StdError::generic_err("Failed to serialize query response"))?;
            Ok(Binary::from(json_response))
        }
        QueryMsg::AllContracts {} => {
            let collections = get_all_contracts(deps.storage)?;
            let response = QueryResponse { collections };
            let json_response = serde_json::to_string(&response)
                .map_err(|_| StdError::generic_err("Failed to serialize query response"))?;
            Ok(Binary::from(json_response))
        }
        QueryMsg::Deployed { owner } => {
            let collections = get_deployed(deps.storage, &owner)?;
            let response = QueryResponse { collections };
            let json_response = serde_json::to_string(&response)
                .map_err(|_| StdError::generic_err("Failed to serialize query response"))?;
            Ok(Binary::from(json_response))
        }
    }
}

// Implement your state methods here

// Implement utility functions (e.g., create_collection, store_collection, set_base_uri, set_whitelist)

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema)]
pub struct ProptixDeploymentConfig {
    pub name: String,
    pub symbol: String,
    pub max_supply: u64,
    pub treasury_address: String,
    // Add other fields as needed for deployment configuration
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema)]
pub struct ProptixRuntimeConfig {
    pub base_token_uri: String,
    pub base_token_uri_extension: String,
    pub prereveal_token_uri: String,
    pub mint_price: u128,
    pub sale_start_time: u64,
    pub sale_end_time: u64,
    pub protocol_fee: u8,
    // Add other fields as needed for runtime configuration
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub deployment_config: ProptixDeploymentConfig,
    pub runtime_config: ProptixRuntimeConfig,
    // Add other fields as needed for instantiation
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema)]
pub struct QueryResponse {
    // Define fields for query responses here
    pub collection_name: String,
    pub total_tokens: u64,
    pub token_ids: Vec<u64>,
    // Add other fields as needed for query responses
}


