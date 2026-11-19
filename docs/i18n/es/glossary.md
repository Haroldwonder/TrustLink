# Glosario

Términos clave usados en la documentación y el código de TrustLink.

*[Read this in English](../../glossary.md)*

| Término | Definición |
|---|---|
| **Attestation (Atestación)** | Un registro on-chain creado por un issuer que certifica que un subject posee un claim específico (p. ej. "KYC_PASSED"). |
| **Claim** | Una afirmación con nombre sobre un subject, como `KYC_PASSED` o `ACCREDITED_INVESTOR`. Los tipos de claim se registran antes de usarse. |
| **Issuer (Emisor)** | Una dirección autorizada por el admin para crear atestaciones. El admin puede añadir o eliminar issuers. |
| **Subject (Sujeto)** | La dirección sobre la que se emite una atestación (p. ej. el usuario que se está verificando con KYC). |
| **Admin (Administrador)** | La dirección con autoridad para configurar el contrato: registrar issuers, establecer comisiones, gestionar contratos puente y transferir derechos de administrador. |
| **Bridge Contract (Contrato puente)** | Un contrato externo registrado para importar atestaciones originadas en otra cadena. |
| **Revocation (Revocación)** | Marcar una atestación existente como no válida. TrustLink mantiene un historial inmutable — las atestaciones revocadas no se eliminan, solo se marcan. |
| **Expiration (Expiración)** | Una marca de tiempo `valid_to` opcional después de la cual una atestación deja de considerarse válida. |
| **TTL (Time To Live)** | El mecanismo del ledger de Soroban que controla cuánto tiempo permanecen activas las entradas de almacenamiento persistente antes de requerir una extensión. Ver [docs/stellar-concepts.md](../../stellar-concepts.md). |
| **`require_auth`** | La llamada del SDK de Soroban que verifica que la dirección que llama ha autorizado la invocación actual. |
| **Multi-Sig Attestation (Atestación multifirma)** | Una atestación que requiere firmas/aprobación de varios issuers antes de ser válida. |
| **Indexer (Indexador)** | El servicio off-chain (`indexer/`) que lee los eventos del contrato y los almacena en una base de datos consultable. |
| **Soroban** | La plataforma de contratos inteligentes de Stellar sobre la que está construido el contrato de TrustLink. |
| **WASM** | WebAssembly — el formato binario compilado en el que se ejecutan los contratos de Soroban. |
