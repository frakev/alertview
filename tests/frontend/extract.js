// Pulls named functions and constants out of static/app.js so they can be run
// for real, rather than re-implemented in a test and drifting from the page.
//
// There is no bundler and no module system in app.js: it is one script the
// browser evaluates whole. Rather than restructure it for testability, this
// lifts the declarations it needs and evaluates them in a scope where the few
// globals they touch (App, AppConfig, TV) are supplied by the test.
const fs = require('fs');
const path = require('path');

const APP_JS = process.env.APP_JS
  || path.join(__dirname, '..', '..', 'static', 'app.js');
const src = fs.readFileSync(APP_JS, 'utf8');

/* Delimiter counting only: no declaration in app.js carries an unbalanced one in
   a string, and an extraction that went wrong would fail to parse below.
   Brackets as well as braces, so a multi-line array constant can be lifted —
   a const whose value is a table of rows is as much a declaration as an object. */
function braceBlock(from) {
  const first = (ch) => { const i = src.indexOf(ch, from); return i === -1 ? Infinity : i; };
  const [open, close] = first('{') < first('[') ? ['{', '}'] : ['[', ']'];
  let depth = 0, started = false;
  for (let i = src.indexOf(open, from); i < src.length; i++) {
    if (src[i] === open) { depth++; started = true; }
    else if (src[i] === close) {
      depth--;
      if (started && depth === 0) return src.slice(from, i + 1);
    }
  }
  throw new Error('unbalanced block from offset ' + from);
}

function grab(names, kind) {
  return names.map(name => {
    const re = kind === 'fn'
      ? new RegExp('^function ' + name + '\\s*\\(', 'm')
      : new RegExp('^(?:const|let) ' + name + '\\s*=', 'm');
    const m = re.exec(src);
    if (!m) throw new Error('not found in app.js: ' + name);

    if (kind === 'const') {
      const eol = src.indexOf('\n', m.index);
      const line = src.slice(m.index, eol);
      if (line.trimEnd().endsWith(';')) return line;
      return braceBlock(m.index) + ';';
    }

    // Skip the parameter list before counting braces: a destructured parameter
    // such as ({ key, value }) is a brace of its own.
    let i = src.indexOf('(', m.index), depth = 0;
    for (; i < src.length; i++) {
      if (src[i] === '(') depth++;
      else if (src[i] === ')' && --depth === 0) break;
    }
    return src.slice(m.index, m.index + braceBlock(i).length + (i - m.index));
  }).join('\n\n');
}

/** Evaluates the named declarations with the given globals, and returns them. */
function load(fns, consts, globals) {
  const names = Object.keys(globals);
  const body = grab(consts, 'const') + '\n' + grab(fns, 'fn')
    + '\nreturn {' + fns.join(',') + '};';
  return new Function(...names, body)(...names.map(n => globals[n]));
}

module.exports = { grab, load, src };
