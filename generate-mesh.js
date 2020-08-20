class Vec3 extends Float32Array {
  static X_POS = new Vec3(1, 0, 0);
  static Y_POS = new Vec3(0, 1, 0);
  static Z_POS = new Vec3(0, 0, 1);
  static X_NEG = new Vec3(-1, 0, 0);
  static Y_NEG = new Vec3(0, -1, 0);
  static Z_NEG = new Vec3(0, 0, -1);

  /**
   * Cross product
   * @param {Vec3} l
   * @param {Vec3} r
   * @returns {Vec3}
   */
  static cross([lx, ly, lz], [rx, ry, rz]) {
    return new Vec3(ly * rz - lz * ry, lz * rx - lx * rz, lx * ry - ly * rx);
  }

  constructor(...args) {
    super(3);
    this.set(args);
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

function createBoxMesh() {
  const faces = [
    [Vec3.X_POS, Vec3.Y_POS],
    [Vec3.X_NEG, Vec3.Y_POS],
    [Vec3.Y_POS, Vec3.Z_POS],
    [Vec3.Y_NEG, Vec3.Z_POS],
    [Vec3.Z_POS, Vec3.Y_POS],
    [Vec3.Z_NEG, Vec3.Y_POS],
  ];

  const faceCount = faces.length;

  const vertexSize = 4 * 3 + 4 * 2 + 4 * 4;
  const vertexCount = 4 * faceCount;
  const vertexEnd = vertexCount * vertexSize;
  const indexStart = align(vertexEnd, 0x100);
  const indexSize = 4;
  const indexCount = 6 * faceCount;
  const dataSize = indexStart + indexCount * indexSize;
  const data = new DataView(new ArrayBuffer(dataSize));

  console.log({ vertexEnd, indexStart, indexSize: dataSize - indexStart, dataSize });

  for (let fi = 0; fi !== faces.length; fi++) {
    const [out, up] = faces[fi];
    const right = Vec3.cross(up, out);
    const bl = out.sub(up).sub(right);
    const br = out.sub(up).add(right);
    const tl = out.add(up).sub(right);
    const tr = out.add(up).add(right);

    vertex(0, tl, 0, 0);
    vertex(1, tr, 1, 0);
    vertex(2, bl, 0, 1);
    vertex(3, br, 1, 1);
    function vertex(vi, pos, u, v) {
      const i = fi * 4 + vi;
      let vertexOffset = i * vertexSize;
      // pos
      data.setFloat32(vertexOffset, pos[0], true);
      data.setFloat32(vertexOffset + 4, pos[1], true);
      data.setFloat32(vertexOffset + 8, pos[2], true);
      // uv
      data.setFloat32(vertexOffset + 12, u, true);
      data.setFloat32(vertexOffset + 16, v, true);
      // color
      data.setFloat32(vertexOffset + 20, (pos[0] + 1) * 0.5, true);
      data.setFloat32(vertexOffset + 24, (pos[1] + 1) * 0.5, true);
      data.setFloat32(vertexOffset + 28, (pos[2] + 1) * 0.5, true);
      data.setFloat32(vertexOffset + 32, 1, true);
    }

    let vi = fi * 4;

    let indexOffset = indexStart + fi * 6 * indexSize;
    // indices: TL, BR, TR
    data.setUint32(indexOffset, vi, true);
    data.setUint32(indexOffset + 4, vi + 2, true);
    data.setUint32(indexOffset + 8, vi + 1, true);
    // indices: TR, BL, BR
    data.setUint32(indexOffset + 12, vi + 1, true);
    data.setUint32(indexOffset + 16, vi + 2, true);
    data.setUint32(indexOffset + 20, vi + 3, true);
  }

  return data.buffer;
}

require("fs").writeFileSync("box.bin", Buffer.from(createBoxMesh()));
