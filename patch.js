const fs = require('fs');
let code = fs.readFileSync('src/main.js', 'utf8');

code = code.replace(/if \(prop === 'fd_prestat_get'\) \{[\s\S]*?return 8; \/\/ EBADF\n\s*};\n\s*}/, 
`if (prop === 'fd_prestat_get') {
                return function(fd, bufPtr) {
                    if (fd === 3) {
                        if (window.__WASI_PROXY.wasm) {
                            const view = new DataView(window.__WASI_PROXY.wasm.exports.memory.buffer);
                            view.setUint32(bufPtr, 0, true); // pr_type = WASI_PREOPENTYPE_DIR
                            view.setUint32(bufPtr + 4, 1, true); // pr_name_len = 1 ("/")
                            return 0; // SUCCESS
                        }
                    }
                    return 8; // EBADF
                };
            }`);

code = code.replace(/if \(prop === 'fd_prestat_dir_name'\) \{[\s\S]*?return 8; \/\/ EBADF\n\s*};\n\s*}/,
`if (prop === 'fd_prestat_dir_name') {
                return function(fd, pathPtr, pathLen) {
                    if (fd === 3) {
                        if (window.__WASI_PROXY.wasm) {
                            const memory = new Uint8Array(window.__WASI_PROXY.wasm.exports.memory.buffer);
                            memory[pathPtr] = 47; // '/'
                            return 0; // SUCCESS
                        }
                    }
                    return 8; // EBADF
                };
            }`);

fs.writeFileSync('src/main.js', code);
