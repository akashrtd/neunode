//! Versioned encrypted identity keys, unlocked by an independent random local secret.
//! Backups must include the keystore secret, or use NEUNODE_KEYSTORE_KEY supplied externally.
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use aes_gcm::aead::rand_core::{OsRng, RngCore};
use anyhow::{Context, Result};
use neunode_identity::keyring::Keyring;
use zeroize::{ZeroizeOnDrop, Zeroizing};

#[derive(serde::Serialize, serde::Deserialize, ZeroizeOnDrop)]
struct PrivateKeys {
    ed25519_private: String,
    secp256k1_private: String,
}

const MAGIC: &[u8] = b"NNKEY002";

pub fn identity_dir(did: &str) -> Result<PathBuf> {
    anyhow::ensure!(
        did.starts_with("did:") && !did.contains(['/', '\\', '\0']),
        "invalid identity path"
    );
    Ok(dirs::home_dir()
        .context("home directory unavailable")?
        .join(".neunode/identities")
        .join(did.replace(':', "_")))
}

fn master_key(create: bool) -> Result<Zeroizing<[u8; 32]>> {
    if let Ok(value) = std::env::var("NEUNODE_KEYSTORE_KEY") {
        return hex::decode(Zeroizing::new(value).as_str())?
            .try_into()
            .map(Zeroizing::new)
            .map_err(|_| anyhow::anyhow!("NEUNODE_KEYSTORE_KEY must encode 32 bytes"));
    }
    let dir = dirs::home_dir().context("home directory unavailable")?.join(".neunode/keystore");
    protected_dir(&dir)?;
    let path = dir.join("master.key");
    if !path.exists() {
        anyhow::ensure!(
            create,
            "keystore master key missing; restore it from backup or supply NEUNODE_KEYSTORE_KEY"
        );
        let mut key = Zeroizing::new([0; 32]);
        OsRng.fill_bytes(key.as_mut());
        let temporary = dir.join(format!("master-{}", random_id()));
        private_write(&temporary, key.as_ref())?;
        // Link publishes a complete file and cannot replace a concurrently created secret.
        let result = fs::hard_link(&temporary, &path);
        fs::remove_file(&temporary)?;
        if let Err(error) = result {
            if error.kind() != std::io::ErrorKind::AlreadyExists {
                return Err(error.into());
            }
        }
    }
    ensure_private(&path)?;
    Zeroizing::new(fs::read(path)?)
        .as_slice()
        .try_into()
        .map(Zeroizing::new)
        .map_err(|_| anyhow::anyhow!("invalid keystore master key"))
}

pub fn random_id() -> String {
    let mut bytes = [0; 32];
    OsRng.fill_bytes(&mut bytes);
    hex::encode(bytes)
}

pub fn protected_dir(path: &Path) -> Result<()> {
    fs::create_dir_all(path)?;
    anyhow::ensure!(fs::symlink_metadata(path)?.is_dir(), "secret directory must not be a symlink");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
    }
    Ok(())
}

pub fn ensure_private(path: &Path) -> Result<()> {
    let metadata = fs::symlink_metadata(path)?;
    anyhow::ensure!(metadata.is_file(), "secret must be a regular file");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        anyhow::ensure!(
            metadata.permissions().mode() & 0o077 == 0,
            "secret file permissions must be 0600"
        );
    }
    Ok(())
}

pub fn private_write(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}

pub fn save_at(dir: &Path, keyring: &Keyring) -> Result<()> {
    save_with_key(dir, keyring, &*master_key(true)?)
}

fn save_with_key(dir: &Path, keyring: &Keyring, key: &[u8; 32]) -> Result<()> {
    protected_dir(dir)?;
    let (ed, secp) = keyring.to_bytes();
    let (ed, secp) = (Zeroizing::new(ed), Zeroizing::new(secp));
    let private = PrivateKeys {
        ed25519_private: hex::encode(ed.as_slice()),
        secp256k1_private: hex::encode(secp.as_slice()),
    };
    let data = Zeroizing::new(serde_json::to_vec(&private)?);
    let mut bytes = MAGIC.to_vec();
    bytes.extend(neunode_crypto::aead::encrypt(key, &data)?);
    let temporary = dir.join(format!("keys-{}.tmp", random_id()));
    private_write(&temporary, &bytes)?;
    fs::rename(&temporary, dir.join("keys.json.enc"))?;
    fs::File::open(dir)?.sync_all()?;
    Ok(())
}

pub fn load(did: &str) -> Result<Keyring> {
    let dir = identity_dir(did)?;
    let encrypted = dir.join("keys.json.enc");
    let versioned = if encrypted.exists() {
        ensure_private(&encrypted)?;
        fs::read(&encrypted)?.starts_with(MAGIC)
    } else {
        false
    };
    load_at(&dir, did, &*master_key(!versioned)?)
}

fn load_at(dir: &Path, did: &str, key: &[u8; 32]) -> Result<Keyring> {
    let encrypted = dir.join("keys.json.enc");
    let (data, migrate) = if encrypted.exists() {
        ensure_private(&encrypted)?;
        let bytes = fs::read(&encrypted)?;
        if let Some(ciphertext) = bytes.strip_prefix(MAGIC) {
            (neunode_crypto::aead::decrypt(key, ciphertext)?, false)
        } else {
            #[allow(deprecated)]
            let old_key = neunode_crypto::aead::derive_machine_key();
            (neunode_crypto::aead::decrypt(&old_key, &bytes)?, true)
        }
    } else {
        let legacy = dir.join("keys.json");
        ensure_private(&legacy)?;
        (fs::read(legacy)?, true)
    };
    let data = Zeroizing::new(data);
    let private: PrivateKeys = serde_json::from_slice(&data)?;
    let ed = Zeroizing::new(hex::decode(&private.ed25519_private)?);
    let secp = Zeroizing::new(hex::decode(&private.secp256k1_private)?);
    let ed = Zeroizing::new(
        <[u8; 32]>::try_from(ed.as_slice())
            .map_err(|_| anyhow::anyhow!("invalid Ed25519 key length"))?,
    );
    let secp = Zeroizing::new(
        <[u8; 32]>::try_from(secp.as_slice())
            .map_err(|_| anyhow::anyhow!("invalid secp256k1 key length"))?,
    );
    let keyring = Keyring::from_bytes(&ed, &secp)?;
    anyhow::ensure!(keyring.to_did().0 == did, "stored keys do not control the requested DID");
    if migrate {
        save_with_key(dir, &keyring, key)?;
        // Delete plaintext only after the new encrypted file is durably published.
        if dir.join("keys.json").exists() {
            fs::remove_file(dir.join("keys.json"))?;
        }
    }
    Ok(keyring)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn encrypted_keys_require_master_secret_and_matching_identity() {
        let temp = tempfile::tempdir().unwrap();
        let keyring = Keyring::generate();
        let key = [1; 32];
        let did = keyring.to_did().0;
        save_with_key(temp.path(), &keyring, &key).unwrap();
        assert_eq!(load_at(temp.path(), &did, &key).unwrap().to_did().0, did);
        assert!(load_at(temp.path(), &did, &[2; 32]).is_err());
        assert!(load_at(temp.path(), &Keyring::generate().to_did().0, &key).is_err());
        let bytes = fs::read(temp.path().join("keys.json.enc")).unwrap();
        assert!(bytes.starts_with(MAGIC));
        assert!(!String::from_utf8_lossy(&bytes).contains("ed25519_private"));
    }
    #[test]
    fn migrates_plaintext_only_after_durable_encryption() {
        let temp = tempfile::tempdir().unwrap();
        let keyring = Keyring::generate();
        let (ed, secp) = keyring.to_bytes();
        let data = serde_json::to_vec(&PrivateKeys {
            ed25519_private: hex::encode(ed),
            secp256k1_private: hex::encode(secp),
        })
        .unwrap();
        private_write(&temp.path().join("keys.json"), &data).unwrap();
        let restored = load_at(temp.path(), &keyring.to_did().0, &[3; 32]).unwrap();
        assert_eq!(restored.to_did(), keyring.to_did());
        assert!(!temp.path().join("keys.json").exists());
        assert!(temp.path().join("keys.json.enc").exists());
    }
    #[cfg(unix)]
    #[test]
    fn rejects_public_secret_files_and_symlink_directories() {
        use std::os::unix::fs::{symlink, PermissionsExt};
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("secret");
        private_write(&path, &[0; 32]).unwrap();
        assert_eq!(fs::metadata(&path).unwrap().permissions().mode() & 0o777, 0o600);
        fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
        assert!(ensure_private(&path).is_err());
        let alias = temp.path().join("alias");
        symlink(temp.path(), &alias).unwrap();
        assert!(protected_dir(&alias).is_err());
    }
}
