describe("equals", (): void => {
  it("should test for equality", (): void => {
    let x = new Uint8Array(42);
    let y = new Uint8Array(42);
    for (let i = 0; i < 42; i++) {
      x[i] = y[i] = i;
    }
    expect<bool>(equals(x, y)).toBeTruthy();
  });
});

describe("memzero", (): void => {
  it("should zero an array", (): void => {
    let x = new Uint8Array(42);
    let y = new Uint8Array(42);
    for (let i = 0; i < 42; i++) {
      x[i] = y[i] = i;
    }
    memzero(x);
    expect<bool>(equals(x, y)).toBeFalsy();
    memzero(y);
    expect<bool>(equals(x, y)).toBeTruthy();
  });
});

describe("bin2hex", (): void => {
  it("shoud encode to hex", (): void => {
    let bin = new Uint8Array(25);
    for (let i = 0; i < 25; i++) {
      bin[i] = i * 5;
    }
    let hex = bin2hex(bin);
    expect<string>(hex).toBe("00050a0f14191e23282d32373c41464b50555a5f64696e7378");
  })
})

describe("hex2bin", (): void => {
  it("shoud decode from hex", (): void => {
    let hex = "00050a0f14191e23282d32373c41464b50555a5f64696e7378";
    let bin = hex2bin(hex);
    let ref = new Uint8Array(25);
    for (let i = 0; i < 25; i++) {
      ref[i] = i * 5;
    }
    expect<bool>(equals(ref, bin!)).toBeTruthy();
  })
})
