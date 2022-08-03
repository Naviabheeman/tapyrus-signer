use crate::cli::setup::index_of;
use crate::cli::setup::traits::Response;
use crate::cli::setup::vss_to_shared_secret_map;
use crate::crypto::vss::Vss;
use crate::errors::Error;
use crate::rpc::Rpc;
use crate::sign::Sign;
use crate::signer_node::NodeParameters;
use clap::{App, Arg, ArgMatches, SubCommand};
use curv::arithmetic::traits::Converter;
use curv::cryptographic_primitives::secret_sharing::feldman_vss::ShamirSecretSharing;
use curv::elliptic::curves::traits::ECPoint;
use curv::elliptic::curves::traits::ECScalar;
use curv::FE;
use std::fmt;
use std::str::FromStr;
use tapyrus::{PrivateKey, PublicKey};

pub struct AggregateResponse {
    aggregated_public_key: PublicKey,
    node_shared_secret: FE,
}

impl AggregateResponse {
    fn new(aggregated_public_key: PublicKey, node_shared_secret: FE) -> Self {
        AggregateResponse {
            aggregated_public_key: aggregated_public_key,
            node_shared_secret: node_shared_secret,
        }
    }
}

impl Response for AggregateResponse {}

impl fmt::Display for AggregateResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let secret = format!("{:0>64}", self.node_shared_secret.to_big_int().to_hex());
        write!(f, "{} {}", self.aggregated_public_key, secret,)
    }
}

pub struct AggregateCommand {}

impl<'a> AggregateCommand {
    pub fn execute(matches: &ArgMatches) -> Result<Box<dyn Response>, Error> {
        let private_key: PrivateKey = matches
            .value_of("private-key")
            .and_then(|key| PrivateKey::from_wif(key).ok())
            .ok_or(Error::InvalidArgs("private-key".to_string()))?;

        let vss_vec: Vec<Vss> = matches
            .values_of("vss")
            .ok_or(Error::InvalidArgs("vss is invalid".to_string()))?
            .map(|s| Vss::from_str(s).map_err(|_| Error::InvalidSS))
            .collect::<Result<Vec<Vss>, _>>()?;

        let mut public_keys = vss_vec
            .iter()
            .map(|vss| vss.sender_public_key)
            .collect::<Vec<PublicKey>>();
        NodeParameters::<Rpc>::sort_publickey(&mut public_keys);

        // threshold is not used in 'aggregate' command
        let params = ShamirSecretSharing {
            threshold: 1,
            share_count: vss_vec.len(),
        };
        let vss_map = vss_to_shared_secret_map(&vss_vec, &params);

        let index = index_of(&private_key, &public_keys);
        let shared_keys = Sign::verify_vss_and_construct_key(&vss_map, &index)?;

        let slice = shared_keys.y.pk_to_key_slice();

        let uncompressed = PublicKey::from_slice(&slice).map_err(|_| Error::InvalidKey)?;
        // Convert compressed public key
        let public_key =
            PublicKey::from_slice(&uncompressed.key.serialize()).map_err(|_| Error::InvalidKey)?;

        Ok(Box::new(AggregateResponse::new(
            public_key,
            shared_keys.x_i,
        )))
    }

    pub fn args<'b>() -> App<'a, 'b> {
        SubCommand::with_name("aggregate").args(&[
            Arg::with_name("vss")
                .long("vss")
                .required(true)
                .multiple(true)
                .takes_value(true)
                .help("secret values (Vss) of the all signers. These values is generated by `tapyrus-setup createnodevss`"),
            Arg::with_name("private-key")
                .long("private-key")
                .required(true)
                .takes_value(true)
                .help("private key of this signer with a WIF format"),
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use curv::elliptic::curves::traits::ECScalar;
    use curv::BigInt;
    use std::str::FromStr;
    use tapyrus::PublicKey;

    #[test]
    fn test_aggregate_response() {
        let public_key = PublicKey::from_str(
            "03842d51608d08bee79587fb3b54ea68f5279e13fac7d72515a7205e6672858ca2",
        )
        .unwrap();
        let response = AggregateResponse::new(public_key, ECScalar::from(&BigInt::from(0xff)));

        assert_eq!(format!("{}", response), "03842d51608d08bee79587fb3b54ea68f5279e13fac7d72515a7205e6672858ca2 00000000000000000000000000000000000000000000000000000000000000ff");
    }

    #[test]
    fn test_execute() {
        let matches = AggregateCommand::args().get_matches_from(vec![
            "aggregate",
            "--private-key",
            "L2hmApEYQBQo81RLJc5MMwo6ZZywnfVzuQj6uCfxFLaV2Yo2pVyq",
            "--vss",
            "03b8ad9e3271a20d5eb2b622e455fcffa5c9c90e38b192772b2e1b58f6b442e78d0313f2a73541e6d55a75a80a6da819885c6ed6e56ecff19f5e928c4ea202ca7c900002b8ad9e3271a20d5eb2b622e455fcffa5c9c90e38b192772b2e1b58f6b442e78d1bb2811fe36fa9e15b7afc0ecdb4c51cad86c2c9135607f38e4ae58198311273bf7eb32ebd24be2854eeb231efb2c2515375a5d67a9aebbca6fb2a3a89653230b8fc1a0f9198b1db842ad620f01d6fa97bf9cbe7e36d4685d68c49817e9a3478c2efa633314d55aa6e1f6d5ce9f345f1aa8dd6dc3a972c14de923269ed4f7d67b8ad9e3271a20d5eb2b622e455fcffa5c9c90e38b192772b2e1b58f6b442e78d1bb2811fe36fa9e15b7afc0ecdb4c51cad86c2c9135607f38e4ae58198311273bf7eb32ebd24be2854eeb231efb2c2515375a5d67a9aebbca6fb2a3a89653230b8fc1a0f9198b1db842ad620f01d6fa97bf9cbe7e36d4685d68c49817e9a3478c2efa633314d55aa6e1f6d5ce9f345f1aa8dd6dc3a972c14de923269ed4f7d67",
            "--vss",
            "0313f2a73541e6d55a75a80a6da819885c6ed6e56ecff19f5e928c4ea202ca7c900313f2a73541e6d55a75a80a6da819885c6ed6e56ecff19f5e928c4ea202ca7c90000213f2a73541e6d55a75a80a6da819885c6ed6e56ecff19f5e928c4ea202ca7c90adbd69de8655fcc6ead8e771f9f31ead7a431e543bf8ac8d921c80ab301bc8d1c8e12c1e4cce10fc64680e9d69b942e3291f62e8cb84e8e32934f3b92ab01fe5e345110a1f558da2f71a654248fbec93e04a757d2cf7277dbd0c2510d6e915aa3ca6d71918c41a84df9c234d47a887c0697a4f2a5b02c99162fbdb1a85d37f1c13f2a73541e6d55a75a80a6da819885c6ed6e56ecff19f5e928c4ea202ca7c90adbd69de8655fcc6ead8e771f9f31ead7a431e543bf8ac8d921c80ab301bc8d1c8e12c1e4cce10fc64680e9d69b942e3291f62e8cb84e8e32934f3b92ab01fe5e345110a1f558da2f71a654248fbec93e04a757d2cf7277dbd0c2510d6e915aa3ca6d71918c41a84df9c234d47a887c0697a4f2a5b02c99162fbdb1a85d37f1c",
            "--vss",
            "023cb7d6326e33332d04d026be1a04cdaf084703d8dc75322182d8fb314a03a8770313f2a73541e6d55a75a80a6da819885c6ed6e56ecff19f5e928c4ea202ca7c9000023cb7d6326e33332d04d026be1a04cdaf084703d8dc75322182d8fb314a03a877be6e3e5cdfc8877c9f9b1a0bbee781019c55098025b03fcede5e4947d16f6140b9e82400e4ba7c8fce269ded9b65df2fdf7d75b3f2a38584a861792019de52d19c5ef89431259b68b4cfd6374c826f4fb33f9f92f701e39644bcddf15cfadf368563c6d7708aee02f688255e3695d187cfb7a9555b09eb19236c3918a7be7f2e3cb7d6326e33332d04d026be1a04cdaf084703d8dc75322182d8fb314a03a877be6e3e5cdfc8877c9f9b1a0bbee781019c55098025b03fcede5e4947d16f6140b9e82400e4ba7c8fce269ded9b65df2fdf7d75b3f2a38584a861792019de52d19c5ef89431259b68b4cfd6374c826f4fb33f9f92f701e39644bcddf15cfadf368563c6d7708aee02f688255e3695d187cfb7a9555b09eb19236c3918a7be7f2e",
        ]);
        let response = AggregateCommand::execute(&matches);
        assert!(response.is_ok());
        let pubkey = response.unwrap();
        // Result must be compressed public key.
        assert_eq!(format!("{}", pubkey), "03addb2555f37abf8f28f11f498bec7bd1460e7243c1813847c49a7ae326a97d1c 84fa4423ba9c5e324443b60868319f3b2910f275415b4083a527e8104aab3a70");
    }

    #[test]
    fn test_execute_invalid_private_key() {
        let matches = AggregateCommand::args().get_matches_from(vec![
            "aggregate",
            "--private-key",
            "x",
            "--vss",
            "03b8ad9e3271a20d5eb2b622e455fcffa5c9c90e38b192772b2e1b58f6b442e78d0313f2a73541e6d55a75a80a6da819885c6ed6e56ecff19f5e928c4ea202ca7c900002b8ad9e3271a20d5eb2b622e455fcffa5c9c90e38b192772b2e1b58f6b442e78d1bb2811fe36fa9e15b7afc0ecdb4c51cad86c2c9135607f38e4ae58198311273bf7eb32ebd24be2854eeb231efb2c2515375a5d67a9aebbca6fb2a3a89653230b8fc1a0f9198b1db842ad620f01d6fa97bf9cbe7e36d4685d68c49817e9a3478c2efa633314d55aa6e1f6d5ce9f345f1aa8dd6dc3a972c14de923269ed4f7d67b8ad9e3271a20d5eb2b622e455fcffa5c9c90e38b192772b2e1b58f6b442e78d1bb2811fe36fa9e15b7afc0ecdb4c51cad86c2c9135607f38e4ae58198311273bf7eb32ebd24be2854eeb231efb2c2515375a5d67a9aebbca6fb2a3a89653230b8fc1a0f9198b1db842ad620f01d6fa97bf9cbe7e36d4685d68c49817e9a3478c2efa633314d55aa6e1f6d5ce9f345f1aa8dd6dc3a972c14de923269ed4f7d67",
            "--vss",
            "0313f2a73541e6d55a75a80a6da819885c6ed6e56ecff19f5e928c4ea202ca7c900313f2a73541e6d55a75a80a6da819885c6ed6e56ecff19f5e928c4ea202ca7c90000213f2a73541e6d55a75a80a6da819885c6ed6e56ecff19f5e928c4ea202ca7c90adbd69de8655fcc6ead8e771f9f31ead7a431e543bf8ac8d921c80ab301bc8d1c8e12c1e4cce10fc64680e9d69b942e3291f62e8cb84e8e32934f3b92ab01fe5e345110a1f558da2f71a654248fbec93e04a757d2cf7277dbd0c2510d6e915aa3ca6d71918c41a84df9c234d47a887c0697a4f2a5b02c99162fbdb1a85d37f1c13f2a73541e6d55a75a80a6da819885c6ed6e56ecff19f5e928c4ea202ca7c90adbd69de8655fcc6ead8e771f9f31ead7a431e543bf8ac8d921c80ab301bc8d1c8e12c1e4cce10fc64680e9d69b942e3291f62e8cb84e8e32934f3b92ab01fe5e345110a1f558da2f71a654248fbec93e04a757d2cf7277dbd0c2510d6e915aa3ca6d71918c41a84df9c234d47a887c0697a4f2a5b02c99162fbdb1a85d37f1c",
            "--vss",
            "023cb7d6326e33332d04d026be1a04cdaf084703d8dc75322182d8fb314a03a8770313f2a73541e6d55a75a80a6da819885c6ed6e56ecff19f5e928c4ea202ca7c9000023cb7d6326e33332d04d026be1a04cdaf084703d8dc75322182d8fb314a03a877be6e3e5cdfc8877c9f9b1a0bbee781019c55098025b03fcede5e4947d16f6140b9e82400e4ba7c8fce269ded9b65df2fdf7d75b3f2a38584a861792019de52d19c5ef89431259b68b4cfd6374c826f4fb33f9f92f701e39644bcddf15cfadf368563c6d7708aee02f688255e3695d187cfb7a9555b09eb19236c3918a7be7f2e3cb7d6326e33332d04d026be1a04cdaf084703d8dc75322182d8fb314a03a877be6e3e5cdfc8877c9f9b1a0bbee781019c55098025b03fcede5e4947d16f6140b9e82400e4ba7c8fce269ded9b65df2fdf7d75b3f2a38584a861792019de52d19c5ef89431259b68b4cfd6374c826f4fb33f9f92f701e39644bcddf15cfadf368563c6d7708aee02f688255e3695d187cfb7a9555b09eb19236c3918a7be7f2e",
        ]);
        let response = AggregateCommand::execute(&matches);
        assert_eq!(
            format!("{}", response.err().unwrap()),
            "InvalidArgs(\"private-key\")"
        );
    }

    #[test]
    fn test_execute_invalid_vss() {
        let matches = AggregateCommand::args().get_matches_from(vec![
            "aggregate",
            "--private-key",
            "L2hmApEYQBQo81RLJc5MMwo6ZZywnfVzuQj6uCfxFLaV2Yo2pVyq",
            "--vss",
            "x",
            "--vss",
            "0313f2a73541e6d55a75a80a6da819885c6ed6e56ecff19f5e928c4ea202ca7c900313f2a73541e6d55a75a80a6da819885c6ed6e56ecff19f5e928c4ea202ca7c90000213f2a73541e6d55a75a80a6da819885c6ed6e56ecff19f5e928c4ea202ca7c90adbd69de8655fcc6ead8e771f9f31ead7a431e543bf8ac8d921c80ab301bc8d1c8e12c1e4cce10fc64680e9d69b942e3291f62e8cb84e8e32934f3b92ab01fe5e345110a1f558da2f71a654248fbec93e04a757d2cf7277dbd0c2510d6e915aa3ca6d71918c41a84df9c234d47a887c0697a4f2a5b02c99162fbdb1a85d37f1c13f2a73541e6d55a75a80a6da819885c6ed6e56ecff19f5e928c4ea202ca7c90adbd69de8655fcc6ead8e771f9f31ead7a431e543bf8ac8d921c80ab301bc8d1c8e12c1e4cce10fc64680e9d69b942e3291f62e8cb84e8e32934f3b92ab01fe5e345110a1f558da2f71a654248fbec93e04a757d2cf7277dbd0c2510d6e915aa3ca6d71918c41a84df9c234d47a887c0697a4f2a5b02c99162fbdb1a85d37f1c",
            "--vss",
            "023cb7d6326e33332d04d026be1a04cdaf084703d8dc75322182d8fb314a03a8770313f2a73541e6d55a75a80a6da819885c6ed6e56ecff19f5e928c4ea202ca7c9000023cb7d6326e33332d04d026be1a04cdaf084703d8dc75322182d8fb314a03a877be6e3e5cdfc8877c9f9b1a0bbee781019c55098025b03fcede5e4947d16f6140b9e82400e4ba7c8fce269ded9b65df2fdf7d75b3f2a38584a861792019de52d19c5ef89431259b68b4cfd6374c826f4fb33f9f92f701e39644bcddf15cfadf368563c6d7708aee02f688255e3695d187cfb7a9555b09eb19236c3918a7be7f2e3cb7d6326e33332d04d026be1a04cdaf084703d8dc75322182d8fb314a03a877be6e3e5cdfc8877c9f9b1a0bbee781019c55098025b03fcede5e4947d16f6140b9e82400e4ba7c8fce269ded9b65df2fdf7d75b3f2a38584a861792019de52d19c5ef89431259b68b4cfd6374c826f4fb33f9f92f701e39644bcddf15cfadf368563c6d7708aee02f688255e3695d187cfb7a9555b09eb19236c3918a7be7f2e",
        ]);
        let response = AggregateCommand::execute(&matches);
        assert_eq!(format!("{}", response.err().unwrap()), "InvalidSS");
    }
}
