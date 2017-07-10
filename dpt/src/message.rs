use util::{keccak256, pk2id, id2pk};
use hash::SECP256K1;
use std::net::SocketAddr;
use secp256k1::key::SecretKey;
use secp256k1::{RecoverableSignature, Message, RecoveryId};
use bigint::{H256, H512};
use tokio_core::net::UdpCodec;
use std::io;

macro_rules! try_none {
    ( $ex:expr ) => {
        match $ex {
            Ok(val) => val,
            Err(_) => return Ok(None),
        }
    }
}

pub struct DPTCodec {
    secret_key: SecretKey,
}

pub struct DPTMessage {
    pub addr: SocketAddr,
    pub typ: u8,
    pub data: Vec<u8>
}

impl UdpCodec for DPTCodec {
    type In = Option<(DPTMessage, H512)>;
    type Out = DPTMessage;

    fn decode(&mut self, src: &SocketAddr, buf: &[u8]) -> Result<Self::In, io::Error> {
        if buf.len() < 98 {
            return Ok(None);
        }

        let hash = keccak256(&buf[32..]);
        let check_hash = H256::from(&buf[0..32]);
        if check_hash != hash {
            return Ok(None);
        }

        let sighash = keccak256(&buf[97..]);
        let rec_id = try_none!(RecoveryId::from_i32(buf[96] as i32));
        let rec_sig = try_none!(RecoverableSignature::from_compact(
            &SECP256K1, &buf[32..96], rec_id
        ));
        let message = try_none!(Message::from_slice(&sighash));
        let public_key = try_none!(SECP256K1.recover(&message, &rec_sig));
        let remote_id = pk2id(&public_key);

        let typ = buf[97];
        let mut data = Vec::new();
        for i in 98..buf.len() {
            data.push(buf[i]);
        }

        Ok(Some((DPTMessage { addr: src.clone(), typ, data }, remote_id)))
    }

    fn encode(&mut self, mut msg: DPTMessage, buf: &mut Vec<u8>) -> SocketAddr {
        let mut typdata = Vec::new();
        typdata.push(msg.typ);
        typdata.append(&mut msg.data);

        let sighash = keccak256(&typdata);
        let message = Message::from_slice(&sighash).unwrap();
        let rec_sig = &SECP256K1.sign_recoverable(&message, &self.secret_key).unwrap();
        let (rec, sig) = rec_sig.serialize_compact(&SECP256K1);

        let mut hashdata = Vec::new();
        for d in sig.as_ref() {
            hashdata.push(*d);
        }
        hashdata.push(rec.to_i32() as u8);
        hashdata.append(&mut typdata);

        let hash = keccak256(&hashdata);

        for i in 0..32 {
            buf.push(hash[i]);
        }
        buf.append(&mut hashdata);

        msg.addr
    }
}