# Guía de inicio rápido

Pon en marcha un contrato TrustLink y verifica claims en pocos minutos.

*[Read this in English](../../quickstart.md)*

## 1. Requisitos previos

| Herramienta | Instalación |
|---|---|
| Rust (stable) | https://rustup.rs |
| Target wasm32 | `rustup target add wasm32-unknown-unknown` |
| Soroban CLI | `cargo install --locked soroban-cli` |

## 2. Usa el despliegue en testnet

Ya existe una instancia de TrustLink desplegada en Stellar Testnet, así que puedes empezar a integrar sin desplegar la tuya propia:

```
Contract ID: CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCN8
Network Passphrase: Test SDF Network ; September 2015
RPC URL: https://soroban-testnet.stellar.org
```

## 3. O ejecuta tu propia instancia local

```bash
# Clona el repositorio
git clone https://github.com/Haroldwonder/TrustLink.git
cd TrustLink

# Inicia un nodo local de Stellar Quickstart
docker compose up -d

# Compila, despliega e inicializa localmente
make local-deploy
```

El ID del contrato desplegado se escribe en `.local.contract-id`.

## 4. Verifica un claim (llamada cross-contract en Rust)

```rust
let trustlink = trustlink::Client::new(&env, &trustlink_id);
let claim = String::from_str(&env, "KYC_PASSED");

if !trustlink.has_valid_claim(&subject, &claim) {
    return Err(Error::KYCRequired);
}
```

## 5. Siguientes pasos

- Patrones completos de integración en Rust y TypeScript: [docs/integration-guide.md](../../integration-guide.md)
- Términos clave usados en toda la documentación: [docs/glossary.md](glossary.md)
- Modelo de seguridad y jerarquía de confianza: [docs/security.md](../../security.md)
- ¿No conoces conceptos de Soroban como TTL o `require_auth`? Consulta [docs/stellar-concepts.md](../../stellar-concepts.md)
