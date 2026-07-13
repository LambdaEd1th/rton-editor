import { readFileSync } from "node:fs";

const file = process.argv[2];
if (!file) {
  throw new Error("usage: verify-shared-worker.mjs <worker.wasm>");
}

const bytes = readFileSync(file);
let offset = 8;

function readLeb128() {
  let value = 0;
  let shift = 0;
  while (offset < bytes.length) {
    const byte = bytes[offset++];
    value += (byte & 0x7f) * 2 ** shift;
    if ((byte & 0x80) === 0) return value;
    shift += 7;
  }
  throw new Error("truncated WebAssembly LEB128 value");
}

let sharedMemory = false;

function readString() {
  const length = readLeb128();
  offset += length;
}

function readLimits() {
  const flags = readLeb128();
  readLeb128();
  if ((flags & 1) !== 0) readLeb128();
  sharedMemory ||= (flags & 2) !== 0;
}

while (offset < bytes.length) {
  const sectionId = bytes[offset++];
  const sectionLength = readLeb128();
  const sectionEnd = offset + sectionLength;
  if (sectionId === 2) {
    const importCount = readLeb128();
    for (let index = 0; index < importCount; index += 1) {
      readString();
      readString();
      const kind = bytes[offset++];
      if (kind === 0) {
        readLeb128();
      } else if (kind === 1) {
        offset += 1;
        readLimits();
      } else if (kind === 2) {
        readLimits();
      } else if (kind === 3) {
        offset += 2;
      } else if (kind === 4) {
        offset += 1;
        readLeb128();
      } else {
        throw new Error(`unknown WebAssembly import kind: ${kind}`);
      }
    }
  } else if (sectionId === 5) {
    const memoryCount = readLeb128();
    for (let index = 0; index < memoryCount; index += 1) {
      readLimits();
    }
  }
  offset = sectionEnd;
}

if (!sharedMemory) {
  throw new Error(`${file} does not declare shared WebAssembly memory`);
}

console.log(`verified shared WebAssembly memory: ${file}`);
