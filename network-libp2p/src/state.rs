extern crate rand;

use std::io::{ Read, Write };
use std::error::Error;
use std::fs::File;
use std::path::{ Path, PathBuf };
use std::sync::Arc;
use std::time::Duration;

use super::libp2p_core::Multiaddr;
use super::libp2p_peerstore::{ Peerstore, PeerAccess, PeerId };
use super::libp2p_peerstore::json_peerstore::JsonPeerstore;
use super::pem;
use super::secp256k1::Secp256k1;
use super::secp256k1::key::{ SecretKey, PublicKey };
use super::slog::Logger;

/// Location of the libp2p peerstore inside the Network base dir.
const PEERS_FILE: &str = "peerstore.json";
/// Location of the libp2p local peer secret key inside the Network base dir.
const LOCAL_PEM_FILE: &str = "local_peer_id.pem";

/// Represents the present state of a libp2p network.
pub struct NetworkState {
    pub base_dir: PathBuf,
    pub pubkey: PublicKey,
    pub seckey: SecretKey,
    pub peer_id: PeerId,
    pub listen_multiaddr: Multiaddr,
    pub peer_store: Arc<JsonPeerstore>,
}

impl NetworkState {
    /// Create a new libp2p network state. Used to initialize 
    /// network service.
    pub fn new(
        // config: LighthouseConfig, 
        base_dir: &Path,
        listen_port: &u16,
        log: &Logger) 
        -> Result <Self, Box<Error>>
    {
        let curve = Secp256k1::new();
        let seckey = match 
            NetworkState::load_secret_key_from_pem_file(base_dir, &curve)
        {
            Ok(k) => k,
            _ => NetworkState::generate_new_secret_key(base_dir, &curve)?
        };
        let pubkey = PublicKey::from_secret_key(&curve, &seckey)?;
        let peer_id = PeerId::from_public_key(
            &pubkey.serialize_vec(&curve, false));
        info!(log, "Loaded keys"; "peer_id" => &peer_id.to_base58());
        let peer_store =  {
            let path = base_dir.join(PEERS_FILE);
            let base = JsonPeerstore::new(path)?;
            Arc::new(base)
        };
        info!(log, "Loaded peerstore"; "peer_count" => &peer_store.peers().count());
        let listen_multiaddr =
            NetworkState::multiaddr_on_port(&listen_port.to_string());
        Ok(Self {
            base_dir: PathBuf::from(base_dir),
            seckey,
            pubkey,
            peer_id,
            listen_multiaddr,
            peer_store,
        })
    }

    /// Return a TCP multiaddress on 0.0.0.0 for a given port.
    pub fn multiaddr_on_port(port: &str) -> Multiaddr {
        return format!("/ip4/0.0.0.0/tcp/{}", port)
            .parse::<Multiaddr>().unwrap()
    }

    pub fn add_peer(&mut self,
                    peer_id: PeerId,
                    multiaddr: Multiaddr,
                    duration_secs: u64) {
        self.peer_store.peer_or_create(&peer_id)
            .add_addr(multiaddr, Duration::from_secs(duration_secs));
    }

    /// Instantiate a SecretKey from a .pem file on disk. 
    pub fn load_secret_key_from_pem_file(
        base_dir: &Path,
        curve: &Secp256k1)
        -> Result<SecretKey, Box<Error>> 
    {
        let path = base_dir.join(LOCAL_PEM_FILE);
        let mut contents = String::new();
        let mut file = File::open(path)?;
        file.read_to_string(&mut contents)?;
        let pem_key = pem::parse(contents)?;
        let key = SecretKey::from_slice(curve, &pem_key.contents)?;
        Ok(key)
    }
    
    /// Generate a new SecretKey and store it on disk as a .pem file. 
    pub fn generate_new_secret_key(
        base_dir: &Path,
        curve: &Secp256k1)
        -> Result<SecretKey, Box<Error>> 
    {
        let mut rng = rand::thread_rng();
        let sk = SecretKey::new(&curve, &mut rng);
        let pem_key = pem::Pem {
            tag: String::from("EC PRIVATE KEY"),
            contents: sk[..].to_vec()
        };
        let s_string = pem::encode(&pem_key);
        let path = base_dir.join(LOCAL_PEM_FILE);
        let mut s_file = File::create(path)?;
        s_file.write(s_string.as_bytes())?;
        Ok(sk)
    }
}
