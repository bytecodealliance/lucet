export as namespace WasmCrypto;

interface IValue {
    value: number
}

declare const SIGN_BYTES: IValue;
declare const SIGN_PUBLICKEYBYTES: IValue;
declare const SIGN_SECRETKEYBYTES: IValue;
declare const SIGN_KEYPAIRBYTES: IValue;
declare const SIGN_SEEDBYTES: IValue;
declare const SIGN_RANDBYTES: IValue;
declare const HASH_BYTES: IValue;
declare const HMAC_BYTES: IValue;
declare const FA_SCALARBYTES: IValue;
declare const FA_POINTBYTES: IValue;