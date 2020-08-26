const { writeFileSync } = require("fs");

class Vec3 extends Float32Array {
  static X_POS = new Vec3(1, 0, 0);
  static Y_POS = new Vec3(0, 1, 0);
  static Z_POS = new Vec3(0, 0, 1);
  static X_NEG = new Vec3(-1, 0, 0);
  static Y_NEG = new Vec3(0, -1, 0);
  static Z_NEG = new Vec3(0, 0, -1);

  constructor(...args) {
    super(3);
    this.set(args);
  }

  /**
   * Cross product
   * @param {Vec3} r
   * @returns {Vec3}
   */
  cross([rx, ry, rz]) {
    const [lx, ly, lz] = this;
    return new Vec3(ly * rz - lz * ry, lz * rx - lx * rz, lx * ry - ly * rx);
  }

  /**
   * Scalar multiplication.
   * @param {number} s Scalar.
   * @return {Vec3}
   */
  muls(s) {
    const [x, y, z] = this;
    return new Vec3(x * s, y * s, z * s);
  }

  /**
   * Rotate around X.
   * @param {number} a Angle in radians.
   * @return {Vec3}
   */
  rotx(a) {
    const [x, y, z] = this;
    const c = Math.cos(a);
    const s = Math.sin(a);
    return new Vec3(x, y * c + z * s, y * -s + z * c);
  }

  /**
   * Rotate around Y.
   * @param {number} a Angle in radians.
   * @return {Vec3}
   */
  roty(a) {
    const [x, y, z] = this;
    const c = Math.cos(a);
    const s = Math.sin(a);
    return new Vec3(x * c - z * s, y, x * s + z * c);
  }

  /**
   * Rotate around Z.
   * @param {number} a Angle in radians.
   * @return {Vec3}
   */
  rotz(a) {
    const [x, y, z] = this;
    const c = Math.cos(a);
    const s = Math.sin(a);
    return new Vec3(x * c + y * s, x * -s + y * c, z);
  }

  /**
   * Subtraction
   * @param {Vec3} r
   * @returns {Vec3}
   */
  sub([rx, ry, rz]) {
    const [lx, ly, lz] = this;
    return new Vec3(lx - rx, ly - ry, lz - rz);
  }

  /**
   * Addition
   * @param {Vec3} r
   * @returns {Vec3}
   */
  add([rx, ry, rz]) {
    const [lx, ly, lz] = this;
    return new Vec3(lx + rx, ly + ry, lz + rz);
  }
}

/**
 * Return the smallest integer multiple of alignment greater or equal to value.
 * Assumes alignment is a power of two.
 * @param {number} value
 * @param {number} alignment
 * @returns {number}
 */
function align(value, alignment) {
  const mask = alignment - 1;
  return (value + mask) & ~mask;
}

function meshBuilder(vertexCount, indexCount, bufferIndex) {
  const vertexSize = 4 * 3 + 4 * 2 + 4 * 4;
  const vertexEnd = vertexCount * vertexSize;
  const indexStart = align(vertexEnd, 0x100);
  const indexSize = 4;
  const dataSize = indexStart + indexCount * indexSize;
  const data = new DataView(new ArrayBuffer(dataSize));

  let vertexIndex = 0;
  let indexIndex = 0;

  return {
    log(name) {
      console.log(`\
  - name: ${name}
    material: 1
    bindings:
      - binding: 0
        view:
          buffer: ${bufferIndex}
          offset: 0
          size: ${vertexEnd}
    indices:
      count: ${indexCount}
      format: u32
      view:
        buffer: ${bufferIndex}
        offset: ${indexStart}
        size: ${dataSize - indexStart}`);
    },

    vertex(x, y, z, u, v, r, g, b, a = 1) {
      let vertexOffset = vertexIndex * vertexSize;
      // pos
      data.setFloat32(vertexOffset, x, true);
      data.setFloat32(vertexOffset += 4, y, true);
      data.setFloat32(vertexOffset += 4, z, true);
      // uv
      data.setFloat32(vertexOffset += 4, u, true);
      data.setFloat32(vertexOffset += 4, v, true);
      // color
      data.setFloat32(vertexOffset += 4, r, true);
      data.setFloat32(vertexOffset += 4, g, true);
      data.setFloat32(vertexOffset += 4, b, true);
      data.setFloat32(vertexOffset += 4, a, true);
      return vertexIndex++;
    },

    indices(...values) {
      for (const value of values) {
        data.setUint32(indexStart + indexIndex++ * indexSize, value, true);
      }
    },

    write(path) {
      writeFileSync(path, Buffer.from(data.buffer));
    }
  };
}

function createBox(bufferId) {
  const faces = [
    [Vec3.X_POS, Vec3.Y_POS],
    [Vec3.X_NEG, Vec3.Y_POS],
    [Vec3.Y_POS, Vec3.Z_POS],
    [Vec3.Y_NEG, Vec3.Z_POS],
    [Vec3.Z_POS, Vec3.Y_POS],
    [Vec3.Z_NEG, Vec3.Y_POS],
  ];

  const faceCount = faces.length;

  const builder = meshBuilder(4 * faceCount, 6 * faceCount, bufferId);

  builder.log('Box');

  for (const [out, up] of faces) {
    const right = up.cross(out);

    function vertex([x, y, z], u, v) {
      const r = (x + 1) * 0.5;
      const g = (y + 1) * 0.5;
      const b = (z + 1) * 0.5;
      return builder.vertex(x, y, z, u, v, r, g, b);
    }

    const tl = vertex(out.add(up).sub(right), 0, 0);
    const tr = vertex(out.add(up).add(right), 1, 0);
    const bl = vertex(out.sub(up).sub(right), 0, 1);
    const br = vertex(out.sub(up).add(right), 1, 1);
    builder.indices(tl, bl, tr);
    builder.indices(tr, bl, br);
  }

  builder.write('box.bin');
}

function createTorus(bufferId) {
  const outerCount = 24;
  const outerRadius = 2;
  const innerCount = 12;
  const innerRadius = 0.5;
  const builder = meshBuilder(
    (outerCount + 1) * (innerCount + 1) * 4,
    outerCount * innerCount * 6,
    bufferId,
  );

  builder.log('Torus');

  for (let o = 0; o !== outerCount + 1; o++) {
    const u = o / outerCount;
    const oa = -u * 2 * Math.PI;
    const center = Vec3.X_POS.muls(outerRadius);

    for (let i = 0; i !== innerCount + 1; i++) {
      const v = i / innerCount;
      const ia = v * 2 * Math.PI;
      const [x, y, z] = center.add(Vec3.X_NEG.rotz(ia).muls(innerRadius)).roty(oa);

      const [r, g, b] = rgbFromHsl(oa, 1 - v, 0.5);
      builder.vertex(x, y, z, u, v, r, g, b);
    }
  }

  for (let o = 0; o !== outerCount; o++) {
    for (let i = 0; i !== innerCount; i++) {
      const br = o * (innerCount + 1) + i;
      const bl = br + (innerCount + 1);
      const tl = bl + 1;
      const tr = br + 1;
      builder.indices(tl, bl, tr);
      builder.indices(tr, bl, br);
    }
  }

  builder.write('torus.bin');
}

function rgbFromHsl(h, s, l) {
  const a = s * Math.min(l, 1 - l);

  function f(n) {
    const k = (n + h * 6 / Math.PI) % 12;
    return l - a * Math.max(-1, Math.min(k - 3, 9 - k, 1));
  }

  return [f(0), f(8), f(4)];
}

// createBox(1);
createTorus(2);