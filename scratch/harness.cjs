// 临时诊断脚本: 在 Cursor harness bundle 中查上下文 / protobuf DSL。
// 用法: node harness.cjs ctx <needle> [pad] | node harness.cjs dsl <Name>...
const fs = require('fs');
const path = require('path');

const bundle = path.join(
    process.env.APPDATA,
    'Cursor',
    'agent-harness',
    'versions',
    'b951911e447652508ba64b4b209ceb5762710e1423deddf7082a6be93bca7cbc',
    'agent-harness.cjs',
);
const text = fs.readFileSync(bundle, 'utf8');
const mode = process.argv[2];

if (mode === 'ctx') {
    const needle = process.argv[3];
    const pad = Number(process.argv[4] || 1200);
    let index = 0;
    let hits = 0;
    while ((index = text.indexOf(needle, index)) >= 0) {
        hits += 1;
        console.log(`\n=== hit ${hits} at ${index} ===`);
        console.log(text.slice(Math.max(0, index - pad), index + pad));
        index += needle.length;
        if (hits >= 5) break;
    }
    console.log(`\n-- hits: ${hits}`);
} else if (mode === 'all') {
    const needle = process.argv[3];
    const pad = Number(process.argv[4] || 400);
    let index = 0;
    let hits = 0;
    while ((index = text.indexOf(needle, index)) >= 0) {
        hits += 1;
        console.log(`\n=== hit ${hits} at ${index} ===`);
        console.log(text.slice(Math.max(0, index - pad), index + pad));
        index += needle.length;
        if (hits >= 40) break;
    }
    console.log(`\n-- hits: ${hits}`);
} else if (mode === 'dsl') {
    const classToDsl = new Map();
    const classPattern = /([A-Za-z0-9_$]+)=class [\s\S]{0,8000}?static \$\(\)\{return\["([A-Za-z0-9_.]+)\|/g;
    let match;
    while ((match = classPattern.exec(text)) !== null) {
        if (!classToDsl.has(match[1])) classToDsl.set(match[1], match[2]);
    }
    const dslPattern = /"([A-Za-z0-9_.]+)\|([^"]*)"((?:,[A-Za-z0-9_$]+)*)\]/g;
    const byName = new Map();
    while ((match = dslPattern.exec(text)) !== null) {
        if (byName.has(match[1])) continue;
        const refs = match[3] ? match[3].split(',').filter(Boolean) : [];
        byName.set(match[1], { fields: match[2], refs });
    }
    for (const name of process.argv.slice(3)) {
        const entry = byName.get(name);
        if (!entry) {
            console.log(`### ${name}: not found`);
            continue;
        }
        const types = entry.refs.map((ref) => `${ref} -> ${classToDsl.get(ref) || '?'}`);
        console.log(`### ${name}\n${entry.fields}\ntypes: ${types.join(' | ')}\n`);
    }
}
