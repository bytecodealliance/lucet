import {
    SIGN_RANDBYTES, SIGN_SEEDBYTES, SIGN_KEYPAIRBYTES, SIGN_BYTES, SIGN_PUBLICKEYBYTES,
    signKeypairFromSeed, signPublicKey, sign, signVerify, bin2hex, hex2bin
} from "./wasm-crypto/crypto";
import { Console, Random, CommandLine, Process, FileSystem, Environ }
    from "../../../modules/wasa/assembly/wasa";

/**
 * Create a new key pair and store it to a file
 * @param keypair_file file to store the key pair to
 */
function createKeypair(keypair_file: string): void {
    Console.log("Creating a new keypair...");
    let seed = Random.randomBytes(SIGN_SEEDBYTES);
    let keypair = signKeypairFromSeed(seed);
    let fd = FileSystem.open(keypair_file, "w");
    if (fd === null) {
        Console.error("Unable to create the keypair file");
        Process.exit(1);
    }
    let keypair_array = new Array<u8>(keypair.length);
    for (let i = 0; i < keypair.length; i++) {
        keypair_array[i] = keypair[i];
    }
    fd!.write(keypair_array);
    fd!.close();
    Console.log("Key pair created and saved into [" + keypair_file + "]");
    let pk_hex = bin2hex(signPublicKey(keypair));
    Console.log("Public key: [" + pk_hex + "]");
}

/**
 * Create a signature given a file and a key pair
 * @param file file to sign
 * @param keypair_file file containing a key pair
 */
function createSignature(file: string, keypair_file: string): void {
    let fd = FileSystem.open(keypair_file, "r");
    if (fd === null) {
        Console.error("Unable to read the keypair file");
        Process.exit(1);
    }
    let keypair_ = fd!.readAll();
    fd!.close();
    if (keypair_ === null || keypair_.length !== SIGN_KEYPAIRBYTES) {
        Console.error("Invalid keypair file content");
        Process.exit(1);
    }
    let keypair = new Uint8Array(SIGN_KEYPAIRBYTES);
    for (let i = 0; i < SIGN_KEYPAIRBYTES; i++) {
        keypair[i] = keypair_![i];
    }
    fd = FileSystem.open(file, "r");
    if (fd === null) {
        Console.error("Unable to open the file to sign");
        Process.exit(1);
    }
    let data_ = fd!.readAll();
    if (data_ === null) {
        Console.error("Error while reading the file to sign");
        Process.exit(1);
    }
    fd!.close();
    let data = data_!;
    let data_len = data.length;
    let data_u8 = new Uint8Array(data.length);
    for (let i = 0; i < data_len; i++) {
        data_u8[i] = data[i];
    }
    let z = Random.randomBytes(SIGN_RANDBYTES);
    let signature = sign(data_u8, keypair, z);
    let signature_hex = bin2hex(signature);
    Console.log("Signature for that file: [" + signature_hex + "]");
}

/**
 * Verify that a signature is valid for a given file and public key
 * @param file file to verify the signature of
 * @param publickey_hex public key, hex-encoded
 * @param signature_hex signature, hex-encoded
 */
function verifySignature(file: string, publickey_hex: string, signature_hex: string): void {
    let publickey = hex2bin(publickey_hex);
    if (publickey === null || publickey.length !== SIGN_PUBLICKEYBYTES) {
        Console.error("Invalid public key");
        Process.exit(1);
    }
    let signature = hex2bin(signature_hex);
    if (signature === null || signature.length !== SIGN_BYTES) {
        Console.error("Invalid signature");
        Process.exit(1);
    }
    let fd = FileSystem.open(file, "r");
    if (fd === null) {
        Console.error("Unable to open the file to sign");
        Process.exit(1);
    }
    let data_ = fd!.readAll();
    if (data_ === null) {
        Console.error("Error while reading the file to sign");
        Process.exit(1);
    }
    fd!.close();
    let data = data_!;
    let data_len = data.length;
    let data_u8 = new Uint8Array(data.length);
    for (let i = 0; i < data_len; i++) {
        data_u8[i] = data[i];
    }
    if (signVerify(signature!, data_u8, publickey!) === false) {
        Console.error("The signature didn't verify");
    } else {
        Console.log("This is a valid signature for that file")
    }
}

/**
 * Help
 */
function help(): void {
    Console.log("\nUsage:\n\n" +
        "keypair\n" +
        "  create a new key pair and save it as `keypair.bin`\n\n" +
        "sign <file>\n" +
        "  return a signature for the file using the keypair\n\n" +
        "verify <file> <public key> <signature>\n" +
        "  verify that a signature is valid for the given public key\n\n" +
        "\n" +
        "The path to the keypair file can be changed using the optional KEYPAIR_FILE\n" +
        "environment variable.\n"
    );
    Process.exit(0);
}

/**
 * Entry point
 */
export function main(): void {
    let command_line = new CommandLine();
    let args = command_line.all();
    if (args.length < 2) {
        help();
    }
    let command = args[1];
    let environ = new Environ();
    let keypair_file = environ.get("KEYPAIR_FILE");
    if (keypair_file === null) {
        keypair_file = "keypair.bin";
    }
    if (command == "keypair") {
        createKeypair(keypair_file!);
    } else if (command == "sign" && args.length == 3) {
        createSignature(args[2], keypair_file!);
    } else if (command == "verify" && args.length == 5) {
        verifySignature(args[2], args[3], args[4]);
    } else {
        help();
    }
}
