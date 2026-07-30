"""Shared SCVal builder utilities for TrustLink Python bindings."""

from typing import Optional, List
from stellar_sdk import Keypair, xdr


def sc_str(s: str) -> xdr.SCVal:
    return xdr.SCVal(type=xdr.SCValType.SC_VAL_TYPE_SYMBOL, sym=s.encode())


def sc_addr(a: str) -> xdr.SCVal:
    return xdr.SCVal(
        type=xdr.SCValType.SC_VAL_TYPE_ADDRESS,
        address=xdr.SCAddress(
            type=xdr.SCAddressType.SC_ADDRESS_TYPE_ACCOUNT,
            account_id=xdr.AccountID(
                type=xdr.PublicKeyType.PUBLIC_KEY_TYPE_ED25519,
                ed25519=xdr.Uint256(Keypair.from_public_key(a).raw_public_key()),
            ),
        ),
    )


def sc_u32(n: int) -> xdr.SCVal:
    return xdr.SCVal(type=xdr.SCValType.SC_VAL_TYPE_U32, u32=xdr.Uint32(n))


def sc_u64(n: int) -> xdr.SCVal:
    return xdr.SCVal(type=xdr.SCValType.SC_VAL_TYPE_U64, u64=xdr.Uint64(n))


def sc_opt_str(s: Optional[str]) -> xdr.SCVal:
    if s is None:
        return xdr.SCVal(type=xdr.SCValType.SC_VAL_TYPE_VEC, vec=[])
    return xdr.SCVal(
        type=xdr.SCValType.SC_VAL_TYPE_VEC,
        vec=[xdr.SCVal(type=xdr.SCValType.SC_VAL_TYPE_SYMBOL, sym=s.encode())],
    )


def sc_opt_u64(n: Optional[int]) -> xdr.SCVal:
    if n is None:
        return xdr.SCVal(type=xdr.SCValType.SC_VAL_TYPE_VEC, vec=[])
    return xdr.SCVal(
        type=xdr.SCValType.SC_VAL_TYPE_VEC,
        vec=[xdr.SCVal(type=xdr.SCValType.SC_VAL_TYPE_U64, u64=xdr.Uint64(n))],
    )


def sc_vec_str(strs: List[str]) -> xdr.SCVal:
    return xdr.SCVal(
        type=xdr.SCValType.SC_VAL_TYPE_VEC,
        vec=[xdr.SCVal(type=xdr.SCValType.SC_VAL_TYPE_SYMBOL, sym=s.encode()) for s in strs],
    )


def sc_vec_addr(addrs: List[str]) -> xdr.SCVal:
    return xdr.SCVal(
        type=xdr.SCValType.SC_VAL_TYPE_VEC,
        vec=[sc_addr(a) for a in addrs],
    )


def sc_null() -> xdr.SCVal:
    return xdr.SCVal(type=xdr.SCValType.SC_VAL_TYPE_VEC, vec=[])
