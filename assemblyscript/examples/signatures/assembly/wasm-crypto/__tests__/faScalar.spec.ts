describe("field arithmetic using scalars", (): void => {
  it("should add, sub and multiply", (): void => {
    let x = new Uint8Array(32);
    let y = new Uint8Array(32);
    for (let i = 0; i < 32; i++) {
      x[i] = i;
      y[i] = i + 0x42;
    }
    let x_plus_y_times_10 = faScalarAdd(x, y);
    for (let i = 0; i < 9; i++) {
      x_plus_y_times_10 = faScalarAdd(x_plus_y_times_10, y);
    }

    let _10 = new Uint8Array(32);
    _10[0] = 10;
    let y10 = faScalarMult(y, _10);
    let x_plus_y10 = faScalarAdd(x, y10);
    expect<bool>(equals(x_plus_y_times_10, x_plus_y10)).toBeTruthy();

    let one = new Uint8Array(32);
    one[0] = 1;
    let minus_one = faScalarNegate(one);
    expect<u8>(minus_one[0]).toStrictEqual(236);
    expect<u8>(minus_one[31]).toStrictEqual(16);

    let zero = new Uint8Array(32);
    let minus_one2 = faScalarSub(zero, one);
    expect<bool>(equals(minus_one, minus_one2)).toBeTruthy();

    let x2 = faScalarSub(x_plus_y10, y10);
    x = faScalarReduce(x);
    expect<bool>(equals(x, x2)).toBeTruthy();

    let x_inv = faScalarInverse(x);
    let x_x_inv = faScalarMult(x, x_inv);
    expect<bool>(equals(x_x_inv, one)).toBeTruthy();

    let x_comp = faScalarComplement(x);
    let x_plus_x_comp = faScalarAdd(x, x_comp);
    expect<bool>(equals(x_plus_x_comp, one)).toBeTruthy();

    let x_neg = faScalarNegate(x);
    let x_plus_x_neg = faScalarAdd(x, x_neg);
    expect<bool>(equals(x_plus_x_neg, zero)).toBeTruthy();

    let cof = faScalarCofactorMult(one);
    expect<u8>(cof[0]).toStrictEqual(8);
  });
});