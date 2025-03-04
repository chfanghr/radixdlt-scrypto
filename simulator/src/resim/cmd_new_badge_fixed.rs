use clap::Parser;
use radix_engine::types::*;
use radix_engine_interface::api::node_modules::metadata::{MetadataValue, Url};
use radix_engine_interface::api::node_modules::ModuleConfig;

use crate::resim::*;

/// Create a fungible badge with fixed supply
#[derive(Parser, Debug)]
pub struct NewBadgeFixed {
    /// The total supply
    pub total_supply: Decimal,

    /// The symbol
    #[clap(long)]
    pub symbol: Option<String>,

    /// The name
    #[clap(long)]
    pub name: Option<String>,

    /// The description
    #[clap(long)]
    pub description: Option<String>,

    /// The website URL
    #[clap(long)]
    pub info_url: Option<String>,

    /// The ICON url
    #[clap(long)]
    pub icon_url: Option<String>,

    /// The network to use when outputting manifest, [simulator | adapanet | nebunet | mainnet]
    #[clap(short, long)]
    pub network: Option<String>,

    /// Output a transaction manifest without execution
    #[clap(short, long)]
    pub manifest: Option<PathBuf>,

    /// The private keys used for signing, separated by comma
    #[clap(short, long)]
    pub signing_keys: Option<String>,

    /// Turn on tracing
    #[clap(short, long)]
    pub trace: bool,
}

impl NewBadgeFixed {
    pub fn run<O: std::io::Write>(&self, out: &mut O) -> Result<(), Error> {
        let default_account = get_default_account()?;
        let mut metadata = BTreeMap::new();
        if let Some(symbol) = self.symbol.clone() {
            metadata.insert("symbol".to_string(), MetadataValue::String(symbol));
        }
        if let Some(name) = self.name.clone() {
            metadata.insert("name".to_string(), MetadataValue::String(name));
        }
        if let Some(description) = self.description.clone() {
            metadata.insert(
                "description".to_string(),
                MetadataValue::String(description),
            );
        }
        if let Some(info_url) = self.info_url.clone() {
            metadata.insert("info_url".to_string(), MetadataValue::Url(Url(info_url)));
        }
        if let Some(icon_url) = self.icon_url.clone() {
            metadata.insert("icon_url".to_string(), MetadataValue::Url(Url(icon_url)));
        };

        let metadata = ModuleConfig {
            init: metadata.into(),
            roles: RolesInit::default(),
        };

        let manifest = ManifestBuilder::new()
            .lock_fee_from_faucet()
            .new_badge_fixed(OwnerRole::None, metadata, self.total_supply)
            .try_deposit_batch_or_refund(default_account)
            .build();
        handle_manifest(
            manifest,
            &self.signing_keys,
            &self.network,
            &self.manifest,
            self.trace,
            true,
            out,
        )
        .map(|_| ())
    }
}
