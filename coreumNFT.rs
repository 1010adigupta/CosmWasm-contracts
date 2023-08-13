use cosmwasm_std::{
    attr,
    to_binary,
    Addr,
    BankMsg,
    Binary,
    Coin,
    Deps,
    DepsMut,
    Env,
    MessageInfo,
    Response,
    StdError,
    StdResult,
    Storage,
};
use cw721::{
    ContractError,
    ExecuteMsg,
    instantiate as cw721_instantiate,
    query as cw721_query,
    Cw721Contract,
    TokenInfoResponse,
    TokensResponse,
};
use cw721_base::msg::{ MintMsg, TransferMsg };
use cw_storage_plus::{ Item, Map };

#[derive(Clone, PartialEq, Debug, Default)]
pub struct State {
    pub base_token_uri: String,
    pub base_token_uri_extension: String,
    pub prereveal_token_uri: String,
    pub treasury_address: String,
    pub protocol_address: String,
    pub mint_price: u128,
    pub sale_start_time: u64,
    pub sale_end_time: u64,
    pub protocol_fee: u8,
    pub max_total_mint: u64,
    pub current_token_id: u64,
    pub uri_status: bool,
}

#[derive(Clone, PartialEq, Debug, Default)]
pub struct Params {
    pub mint_fee: Coin,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum HandleMsg {
    IssueClass {
        name: String,
        symbol: String,
        description: Option<String>,
        uri: Option<String>,
        uri_hash: Option<String>,
        data: Option<Binary>,
        features: Option<Vec<u32>>,
        royalty_rate: Option<String>,
    },
    Mint {
        class_id: String,
        id: String,
        uri: Option<String>,
        uri_hash: Option<String>,
        data: Option<Binary>,
    },
    Burn {
        class_id: String,
        id: String,
    },
    Freeze {
        class_id: String,
        id: String,
    },
    Unfreeze {
        class_id: String,
        id: String,
    },
    AddToWhitelist {
        class_id: String,
        id: String,
        account: String,
    },
    RemoveFromWhitelist {
        class_id: String,
        id: String,
        account: String,
    },
}

impl State {
    pub fn new(deployment_config: &DeploymentConfig, runtime_config: &RuntimeConfig) -> Self {
        State {
            base_token_uri: runtime_config.base_token_uri.clone(),
            base_token_uri_extension: runtime_config.base_token_uri_extension.clone(),
            prereveal_token_uri: runtime_config.prereveal_token_uri.clone(),
            treasury_address: deployment_config.treasury_address.clone(),
            protocol_address: String::new(), // Needs to be set
            mint_price: runtime_config.mint_price,
            sale_start_time: runtime_config.sale_start_time,
            sale_end_time: runtime_config.sale_end_time,
            protocol_fee: runtime_config.protocol_fee,
            max_total_mint: deployment_config.max_supply,
            current_token_id: 0,
            uri_status: false,
        }
    }

    // Add other required state methods as needed

    pub fn whitelist(&mut self, account: Addr, status: bool) {
        // Update the isWhitelisted mapping
        self.is_whitelisted.update(&account.to_string(), |_| Some(status));

        // You can emit an event if needed using `attr`
        let event_type = if status { "whitelist_add" } else { "whitelist_remove" };
        attr("action", event_type);
        attr("account", account);
    }

    pub fn purchase(&mut self, count: u64, sender: Addr) -> Result<(), ContractError> {
        // Ensure that the sender is whitelisted
        if !self.is_whitelisted(&sender) {
            return Err(ContractError::Unauthorized {});
        }

        // Ensure that the sale is active
        let now = env.block.time;
        if now < self.sale_start_time || now > self.sale_end_time {
            return Err(ContractError::SaleNotActive {});
        }

        // Calculate the total cost
        let total_cost = self.mint_price * (count as u128);

        // Ensure that the sender has enough funds
        if total_cost > self.get_balance(&sender)? {
            return Err(ContractError::InsufficientFunds {});
        }

        // Distribute sales income
        let protocol_fee_amount = (total_cost * (self.protocol_fee as u128)) / 100;
        let treasury_amount = total_cost - protocol_fee_amount;

        // Update balances and state
        self.update_balance(&sender, -total_cost)?;
        self.update_balance(&self.protocol_address, protocol_fee_amount as u64)?;
        self.update_balance(&self.treasury_address, treasury_amount as u64)?;
        self.current_token_id += count;

        // Mint the purchased tokens
        for _ in 0..count {
            self.mint(deps.as_mut(), env.clone(), sender.clone())?;
        }

        // Return a successful response
        Ok(())
    }
    pub fn get_balance(&self, addr: &Addr) -> Result<u64, ContractError> {
        let balance = self.balance.may_load(addr.as_bytes())?.unwrap_or_default();
        Ok(balance)
    }

    pub fn update_balance(&mut self, addr: &Addr, amount: i128) -> Result<(), ContractError> {
        let current_balance = self.get_balance(addr)? as i128;
        if current_balance + amount < 0 {
            return Err(ContractError::InsufficientFunds {});
        }

        self.balance.save(addr.as_bytes(), &(current_balance + amount) as u64);
        Ok(())
    }

    pub fn mint(&mut self, deps: DepsMut, env: Env, recipient: Addr) -> Result<(), ContractError> {
        let token_id = self.current_token_id;

        // Implement the logic to mint the token using cw721 mint function
        let mint_msg = MintMsg {
            owner: recipient.to_string(),
            token_id: token_id.to_string(),
            uri: Some(self.token_uri(token_id)),
            data: None, // Set data if needed
        };

        let mint_response = self.cw721.mint(deps, env.clone(), mint_msg)?;

        // Update state and attributes
        self.current_token_id += 1;
        self.nfts.push(token_id);

        // Return a successful response
        Ok(());
        unimplemented!()
    }
}

impl Contract for State {
    fn instantiate(
        &mut self,
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        msg: InstantiateMsg
    ) -> Result<Response, ContractError> {
        // Initialize the CW721 contract and create initial tokens
        let cw721_msg = Cw721InstantiateMsg {
            name: "My NFT".to_string(),
            symbol: "MYNFT".to_string(),
            minter: Some(info.sender.to_string()),
            metadata: None, // Set metadata if needed
        };

        // Call the CW721 instantiate function
        let cw721_response: ContractResult<Response> = cw721_instantiate(
            deps.clone(),
            env.clone(),
            info.clone(),
            cw721_msg.clone()
        );

        // Handle the CW721 response (checking for errors)
        let cw721_response = cw721_response.map_err(ContractError::from)?;

        // Initialize your custom state based on msg

        // You can return a combined response or just the CW721 response
        Ok(cw721_response)
    }
}

fn execute(
    &mut self,
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Whitelist { address, status } => {
            // Ensure that the sender is the contract owner
            if self.owner != deps.api.addr_canonicalize(info.sender.as_str())? {
                return Err(ContractError::Unauthorized {});
            }

            // Call the whitelist function to update state
            self.whitelist(address.clone(), status);

            // Return a successful response
            Ok(
                Response::new().add_attributes(
                    vec![attr("action", "whitelist"), attr("account", address)]
                )
            )
        }
        ExecuteMsg::Purchase { count } => {
            self.purchase(count, deps.api.addr_canonicalize(info.sender.as_str())?)?;

            // Return a successful response
            Ok(
                Response::new().add_attributes(
                    vec![
                        attr("action", "purchase"),
                        attr("buyer", info.sender),
                        attr("count", count.to_string())
                    ]
                )
            )
        }
        // Implement other ExecuteMsg cases as needed
    }
}

fn query(&self, deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    // Use the cw721_query to handle queries
    // (Optional)Implement additional queries specific to your contract
}

// Implement the required CW721 handlers using Cw721Contract trait

impl Cw721Contract for State {
    fn transfer(
        &mut self,
        deps: DepsMut,
        env: Env,
        sender: Addr,
        recipient: Addr,
        token_id: String
    ) -> Result<(), ContractError> {
        
        // Check if the sender owns the token
        if !self.tokens.may_load(&token_id)?.map_or(false, |owner| owner == sender) {
            return Err(ContractError::Unauthorized {});
        }

        // Create the CW721 transfer message
        let transfer_msg = Cw721TransferMsg {
            recipient: recipient.to_string(),
            token_id: token_id.clone(),
        };

        // Call the CW721 contract's transfer function
        let cw721_response: Response = cw721_base::transfer(
            deps.clone(),
            env.clone(),
            sender.clone(),
            transfer_msg
        )?;

        // Update the state with the new token owner
        self.tokens.save(&token_id, &recipient);

        // Return a successful response
        Ok(())
    }

    fn token_info(
        &self,
        deps: Deps,
        env: Env,
        token_id: String
    ) -> Result<TokenInfoResponse, ContractError> {
        // Implement token_info query using cw721_query
        let query_msg = Cw721QueryMsg::TokenInfo { token_id };
        let response: ContractResult<Binary> = cw721_query(deps, env, query_msg);
        let response = response.map_err(ContractError::from)?;

        // Decode and return the response
        let token_info_response: TokenInfoResponse = from_binary(&response)?;
        Ok(token_info_response)
    }

    fn tokens(
        &self,
        deps: Deps,
        env: Env,
        address: String,
        page: Option<PageRequest>
    ) -> Result<TokensResponse, ContractError> {
        // Implement tokens query using cw721_query
        let query_msg = Cw721QueryMsg::Tokens {
            owner: address,
            start_after: None, // Implement start_after if needed
            limit: page.map(|p| p.limit.unwrap_or(10)),
        };
        let response: ContractResult<Binary> = cw721_query(deps, env, query_msg);
        let response = response.map_err(ContractError::from)?;

        // Decode and return the response
        let tokens_response: TokensResponse = from_binary(&response)?;
        Ok(tokens_response)
    }

    // Implement other CW721 handlers as needed
}
