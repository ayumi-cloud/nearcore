use clap::{App, Arg};
use genesis_populate::GenesisBuilder;
use near::{get_default_home, get_store_path, load_config};
use near_store::create_store;
use std::path::Path;

fn main() {
    let default_home = get_default_home();
    let matches = App::new("Genesis populator")
        .arg(
            Arg::with_name("home")
                .long("home")
                .default_value(&default_home)
                .help("Directory for config and data (default \"~/.near\")")
                .takes_value(true),
        )
        .arg(Arg::with_name("additional-accounts-num").long("additional-accounts-num").required(true).takes_value(true).help("Number of additional accounts per shard to add directly to the trie (TESTING ONLY)"))
        .get_matches();

    let home_dir = matches.value_of("home").map(|dir| Path::new(dir)).unwrap();
    let additional_accounts_num = matches
        .value_of("additional-accounts-num")
        .map(|x| x.parse::<u64>().expect("Failed to parse number of additional accounts."))
        .unwrap();
    let near_config = load_config(home_dir);

    let store = create_store(&get_store_path(home_dir));
    GenesisBuilder::from_config_and_store(home_dir, near_config.genesis_config.clone(), store)
        .add_additional_accounts(additional_accounts_num)
        .add_additional_accounts_contract(
            include_bytes!(
                "../../../runtime/runtime/tests/tiny-contract-rs/res/tiny_contract_rs.wasm"
            )
            .to_vec(),
        )
        .print_progress()
        .build()
        .unwrap()
        .dump_state()
        .unwrap();
}