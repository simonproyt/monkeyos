

window.__WASI_PROXY = {
    kernel: null,
};


function wasi_print_js(id, text) {
    console.log("WASI OUT:", text);
}

function escapeHtml(str) {
    return str
        .replace(/&/g, "&amp;")
        .replace(/</g, "&lt;")
        .replace(/>/g, "&gt;")
        .replace(/"/g, "&quot;")
        .replace(/'/g, "&#039;");
}

// Redirect console.log to our boot screen
const originalLog = console.log;
console.log = function(...args) {
    originalLog(...args);
    const bootConsole = document.getElementById('boot-console');
    if (bootConsole) {
        bootConsole.textContent += args.join(' ') + '\n';
        bootConsole.scrollTop = bootConsole.scrollHeight;
    }
};

async function initWebGPU() {
    if (!navigator.gpu) {
        console.error("WebGPU not supported on this browser.");
        return null;
    }

    const adapter = await navigator.gpu.requestAdapter();
    if (!adapter) {
        console.error("No appropriate WebGPU adapter found.");
        return null;
    }

    const device = await adapter.requestDevice();
    const canvas = document.getElementById("os-canvas");
    const context = canvas.getContext("webgpu");

    if (!context) {
        console.error("Failed to get WebGPU context.");
        return null;
    }

    function resizeCanvas() {
        canvas.width = window.innerWidth;
        canvas.height = window.innerHeight;
        const textCanvas = document.getElementById("text-overlay");
        if (textCanvas) {
            textCanvas.width = window.innerWidth;
            textCanvas.height = window.innerHeight;
            window.textCtx = textCanvas.getContext("2d");
        }
        if (window.__WASI_PROXY && window.__WASI_PROXY.kernel) {
            window.__WASI_PROXY.kernel.push_screen_size(canvas.width, canvas.height);
        }
    }
    window.addEventListener('resize', resizeCanvas);
    resizeCanvas();

    const presentationFormat = navigator.gpu.getPreferredCanvasFormat();
    context.configure({
        device,
        format: presentationFormat,
        alphaMode: "premultiplied",
    });

    const wgsl = `
      struct Uniforms {
        screen_size: vec2<f32>,
      }
      @group(0) @binding(0) var<uniform> uniforms: Uniforms;

      struct VertexInput {
        @location(0) position: vec2<f32>,
        @location(1) color: vec4<f32>,
        @location(2) rect_pos: vec2<f32>,
        @location(3) rect_size: vec2<f32>,
        @location(4) radius: f32,
        @location(5) shadow_blur: f32,
      }

      struct VertexOutput {
        @builtin(position) position: vec4<f32>,
        @location(0) color: vec4<f32>,
        @location(1) rect_pos: vec2<f32>,
        @location(2) rect_size: vec2<f32>,
        @location(3) radius: f32,
        @location(4) shadow_blur: f32,
        @location(5) pixel_pos: vec2<f32>,
      }

      @vertex fn vs(input: VertexInput) -> VertexOutput {
        var output: VertexOutput;
        let normalized = (input.position / uniforms.screen_size) * 2.0 - 1.0;
        output.position = vec4<f32>(normalized.x, -normalized.y, 0.0, 1.0); // WebGPU Y is up
        output.color = input.color;
        output.rect_pos = input.rect_pos;
        output.rect_size = input.rect_size;
        output.radius = input.radius;
        output.shadow_blur = input.shadow_blur;
        output.pixel_pos = input.position;
        return output;
      }

      @fragment fn fs(input: VertexOutput) -> @location(0) vec4<f32> {
          let half_size = input.rect_size * 0.5;
          let center = input.rect_pos + half_size;
          let p = input.pixel_pos - center;
          
          let b = half_size - vec2<f32>(input.radius);
          let q = abs(p) - b;
          let dist = length(max(q, vec2<f32>(0.0))) + min(max(q.x, q.y), 0.0) - input.radius;
          
          var final_color = vec4<f32>(0.0);
          let rect_alpha = 1.0 - smoothstep(-0.5, 0.5, dist);
          
          if (input.shadow_blur > 0.0) {
              let shadow_dist = max(0.0, dist);
              let shadow_alpha = exp(-shadow_dist * shadow_dist / (input.shadow_blur * input.shadow_blur)) * 0.5 * input.color.a;
              let shadow_color = vec4<f32>(0.0, 0.0, 0.0, shadow_alpha);
              final_color = mix(shadow_color, input.color, rect_alpha);
          } else {
              final_color = vec4<f32>(input.color.rgb, input.color.a * rect_alpha);
          }
          
          return vec4<f32>(final_color.rgb * final_color.a, final_color.a);
      }
    `;

    const shaderModule = device.createShaderModule({ code: wgsl });

    const pipeline = device.createRenderPipeline({
        layout: "auto",
        vertex: {
            module: shaderModule,
            entryPoint: "vs",
            buffers: [{
                arrayStride: 48,
                attributes: [
                    { shaderLocation: 0, offset: 0, format: "float32x2" },
                    { shaderLocation: 1, offset: 8, format: "float32x4" },
                    { shaderLocation: 2, offset: 24, format: "float32x2" },
                    { shaderLocation: 3, offset: 32, format: "float32x2" },
                    { shaderLocation: 4, offset: 40, format: "float32" },
                    { shaderLocation: 5, offset: 44, format: "float32" }
                ]
            }]
        },
        fragment: {
            module: shaderModule,
            entryPoint: "fs",
            targets: [{ 
                format: presentationFormat,
                blend: {
                    color: { srcFactor: 'one', dstFactor: 'one-minus-src-alpha', operation: 'add' },
                    alpha: { srcFactor: 'one', dstFactor: 'one-minus-src-alpha', operation: 'add' }
                }
            }]
        },
        primitive: { topology: "triangle-list" }
    });

    const uniformBuffer = device.createBuffer({
        size: 8,
        usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST,
    });
    
    const bindGroup = device.createBindGroup({
        layout: pipeline.getBindGroupLayout(0),
        entries: [{ binding: 0, resource: { buffer: uniformBuffer } }]
    });

    let vertices = new Float32Array(10000 * 72); // Max 10k rects (each has 6 vertices * 12 floats)
    let rectCount = 0;
    
    const vertexBuffer = device.createBuffer({
        size: 10000 * 72 * 4, // bytes
        usage: GPUBufferUsage.VERTEX | GPUBufferUsage.COPY_DST,
    });

    window.clear_screen_js = function() {
        rectCount = 0;
        window.gpuDirty = true;
    };

    window.draw_rect_js = function(x, y, w, h, r, g, b, a, radius, shadow_blur) {
        if (rectCount >= 10000) return;
        const idx = rectCount * 72; // 6 vertices * 12 floats
        
        const rect = {x, y, w, h, r, g, b, a, radius, shadow_blur};

        if (window.textCtx && a > 0.8) {
            window.textCtx.clearRect(x, y, w, h);
        }
        
        // Inflate geometry bounds to include the shadow blur margin
        const padding = rect.shadow_blur * 2.0;
        const px = rect.x - padding;
        const py = rect.y - padding;
        const pw = rect.w + padding * 2.0;
        const ph = rect.h + padding * 2.0;

        const pushVertex = (vIdx, vx, vy) => {
            vertices[idx + vIdx*12 + 0] = vx; vertices[idx + vIdx*12 + 1] = vy;
            vertices[idx + vIdx*12 + 2] = rect.r; vertices[idx + vIdx*12 + 3] = rect.g; 
            vertices[idx + vIdx*12 + 4] = rect.b; vertices[idx + vIdx*12 + 5] = rect.a;
            vertices[idx + vIdx*12 + 6] = rect.x; vertices[idx + vIdx*12 + 7] = rect.y;
            vertices[idx + vIdx*12 + 8] = rect.w; vertices[idx + vIdx*12 + 9] = rect.h;
            vertices[idx + vIdx*12 + 10] = rect.radius;
            vertices[idx + vIdx*12 + 11] = rect.shadow_blur;
        };

        pushVertex(0, px, py);
        pushVertex(1, px + pw, py);
        pushVertex(2, px, py + ph);
        
        pushVertex(3, px + pw, py);
        pushVertex(4, px + pw, py + ph);
        pushVertex(5, px, py + ph);

        rectCount++;
    };

    window.renderWebGPU = function() {
        if (rectCount === 0) return;

        device.queue.writeBuffer(vertexBuffer, 0, vertices, 0, rectCount * 72);
        device.queue.writeBuffer(uniformBuffer, 0, new Float32Array([canvas.width, canvas.height]));

        const commandEncoder = device.createCommandEncoder();
        const passEncoder = commandEncoder.beginRenderPass({
            colorAttachments: [{
                view: context.getCurrentTexture().createView(),
                loadOp: "clear",
                clearValue: { r: 0.0, g: 0.0, b: 0.0, a: 0.0 }, // Transparent background so HTML shows
                storeOp: "store",
            }]
        });

        passEncoder.setPipeline(pipeline);
        passEncoder.setBindGroup(0, bindGroup);
        passEncoder.setVertexBuffer(0, vertexBuffer);
        passEncoder.draw(rectCount * 6);
        passEncoder.end();

        device.queue.submit([commandEncoder.finish()]);
    };
    
    // JS side implementations of OS services
    
    // pending variables moved to global scope
    
    console.log("[ OK ] WebGPU subsystem initialized.");
    return { device, context, renderWebGPU };
}

async function bootstrap() {
    console.log("Loading WebAssembly Microkernel from disk...");
    
    // IndexedDB wrapper functions
    function initIndexedDB() {
        return new Promise((resolve, reject) => {
            const request = indexedDB.open("MonkeyOS_DB", 2);
            request.onupgradeneeded = (event) => {
                const db = event.target.result;
                if (!db.objectStoreNames.contains("vfs_store")) {
                    db.createObjectStore("vfs_store");
                }
                if (!db.objectStoreNames.contains("binaries_store")) {
                    db.createObjectStore("binaries_store");
                }
            };
            request.onsuccess = (event) => resolve(event.target.result);
            request.onerror = (event) => reject(event.target.error);
        });
    }

    async function loadVfsFromDB(db) {
        return new Promise((resolve, reject) => {
            const transaction = db.transaction("vfs_store", "readonly");
            const store = transaction.objectStore("vfs_store");
            const request = store.get("vfs");
            request.onsuccess = () => resolve(request.result);
            request.onerror = () => reject(request.error);
        });
    }

    function saveVfsToDB(db, vfsData) {
        return new Promise((resolve, reject) => {
            // Strip out non-serializable or very large properties before saving
            const dataToSave = {};
            for (const path in vfsData) {
                const node = vfsData[path];
                dataToSave[path] = { ...node };
                if (dataToSave[path].module) {
                    delete dataToSave[path].module;
                }
                // Don't save large preloaded executables to DB, they are fetched on boot
                if (path.startsWith("/bin/") && dataToSave[path].type === "executable") {
                    delete dataToSave[path];
                    continue;
                }
            }
            const transaction = db.transaction("vfs_store", "readwrite");
            const store = transaction.objectStore("vfs_store");
            const request = store.put(dataToSave, "vfs");
            request.onsuccess = () => resolve();
            request.onerror = () => reject(request.error);
        });
    }

    // Default VFS Implementation
    let vfs = {
        "/": { type: "dir", children: ["home", "etc", "usr", "var", "tmp"], timestamp: Date.now() },
        "/home": { type: "dir", children: ["monkey"], timestamp: Date.now() },
        "/home/monkey": { type: "dir", children: ["readme.txt"], timestamp: Date.now() },
        "/home/monkey/readme.txt": { type: "file", content: "Hello from MonkeyOS!\n", timestamp: Date.now() },
        "/etc": { type: "dir", children: ["os-release", "passwd"], timestamp: Date.now() },
        "/etc/os-release": { type: "file", content: "NAME=MonkeyOS\nVERSION=0.1.0\n", timestamp: Date.now() },
        "/etc/passwd": { type: "file", content: "root:x:0:0:root:/root:/bin/sh\n", timestamp: Date.now() },
        "/usr": { type: "dir", children: [], timestamp: Date.now() },
        "/var": { type: "dir", children: [], timestamp: Date.now() },
        "/tmp": { type: "dir", children: [], timestamp: Date.now() },
    };

    let db;
    try {
        db = await initIndexedDB();
        const savedVfs = await loadVfsFromDB(db);
        if (savedVfs) {
            vfs = savedVfs;
            // Retrofit timestamps to old nodes if missing
            for (const path in vfs) {
                if (!vfs[path].timestamp) {
                    vfs[path].timestamp = Date.now();
                }
            }
        } else {
            await saveVfsToDB(db, vfs);
        }
    } catch (e) {
        console.error("Failed to initialize IndexedDB for VFS", e);
    }

    // Pre-fetch dynamic executables (caching them to VFS)
    try {
        if (!vfs["/bin"]) {
            vfs["/bin"] = { type: "dir", children: [], timestamp: Date.now() };
            vfs["/"].children.push("bin");
        }
        
        let shouldSaveVfs = false;
        
        // Preload binaries
        const binaries = ['hello.wasm', 'coreutils.wasm', 'sh.wasm', 'edit.wasm', 'calc.wasm', 'fileman.wasm', 'notepad.wasm'];
        
        function getBinaryFromDB(db, name) {
            return new Promise((resolve) => {
                try {
                    const tx = db.transaction("binaries_store", "readonly");
                    const store = tx.objectStore("binaries_store");
                    const req = store.get(name);
                    req.onsuccess = () => resolve(req.result);
                    req.onerror = () => resolve(null);
                } catch(e) { resolve(null); }
            });
        }
        
        function saveBinaryToDB(db, name, buffer, lastModified) {
            return new Promise((resolve) => {
                try {
                    const tx = db.transaction("binaries_store", "readwrite");
                    const store = tx.objectStore("binaries_store");
                    store.put({ buffer, lastModified }, name);
                    tx.oncomplete = () => resolve();
                } catch(e) { resolve(); }
            });
        }

        for (const bin of binaries) {
            try {
                let cached = db ? await getBinaryFromDB(db, bin) : null;
                let buffer = null;
                
                // Do a HEAD request to check Last-Modified
                const headResp = await fetch(`/bin/${bin}?t=${Date.now()}`, { method: 'HEAD' });
                const lastModified = headResp.headers.get("Last-Modified");
                
                if (cached && cached.lastModified === lastModified && cached.buffer) {
                    buffer = cached.buffer;
                } else {
                    const resp = await fetch(`/bin/${bin}?t=${Date.now()}`);
                    if (resp.ok) {
                        buffer = await resp.arrayBuffer();
                        if (db) {
                            await saveBinaryToDB(db, bin, buffer, lastModified);
                        }
                    }
                }
                
                if (buffer) {
                    vfs['/bin/' + bin.split('.')[0]] = {
                        type: "executable",
                        binary: buffer,
                        timestamp: Date.now()
                    };
                    if (!vfs["/bin"].children.includes(bin.split('.')[0])) {
                        vfs["/bin"].children.push(bin.split('.')[0]);
                    }
                }
            } catch (e) {
                console.error(`Failed to load ${bin}`, e);
            }
        }
        
        // create aliases for coreutils commands
        const cmds = ["ls", "cat", "echo", "pwd", "mkdir", "rm", "head", "tail", "wc", "sort", "touch"];
        for (let cmd of cmds) {
            vfs[`/bin/${cmd}`] = vfs["/bin/coreutils"];
            if (!vfs["/bin"].children.includes(cmd)) {
                vfs["/bin"].children.push(cmd);
            }
        }
        shouldSaveVfs = true;
        
        if (shouldSaveVfs && db) {
            await saveVfsToDB(db, vfs);
        }
    } catch (e) {
        console.error("Failed to fetch binary executables", e);
    }

    window.__WASI_FDS = new Map();
    let nextFd = 5;

    function resolvePath(basePath, relativePath) {
        if (relativePath.startsWith("/")) return relativePath;
        let parts = basePath.split("/").filter(Boolean);
        let newParts = relativePath.split("/").filter(Boolean);
        for (let p of newParts) {
            if (p === ".") continue;
            if (p === "..") parts.pop();
            else parts.push(p);
        }
        return "/" + parts.join("/");
    }

    function getVfsPath(path) {
        return path.replace(/\/+/g, "/").replace(/\/$/, "") || "/";
    }

    function getEnvs(cwd) {
        const tzOffsetMins = new Date().getTimezoneOffset();
        const tzOffsetHours = tzOffsetMins / 60;
        const sign = tzOffsetHours > 0 ? "+" : "";
        const tzStr = "TZ=UTC" + sign + tzOffsetHours;
        return ["PWD=" + cwd, "USER=root", tzStr];
    }

    let saveVfsTimeout = null;
    function saveVfs() {
        if (!db) return;
        if (saveVfsTimeout) clearTimeout(saveVfsTimeout);
        saveVfsTimeout = setTimeout(() => {
            saveVfsToDB(db, vfs).catch(e => console.error("Async VFS save failed", e));
        }, 1000); // 1 second debounce
    }

    const wasi_snapshot_preview1 = new Proxy({}, {
        get(target, prop, receiver) {
            if (prop === 'fd_write') {
                return function(fd, iovs_ptr, iovs_len, nwritten_ptr) {
                    if (fd <= 2) {
                        if (window.__WASI_PROXY.wasm !== wasmInstance) {
                            const view = new DataView(window.__WASI_PROXY.wasm.exports.memory.buffer);
                            const memory = new Uint8Array(window.__WASI_PROXY.wasm.exports.memory.buffer);
                            let totalWritten = 0;
                            let outStr = "";
                            for (let i = 0; i < iovs_len; i++) {
                                const iov_ptr = iovs_ptr + i * 8;
                                const buf_ptr = view.getUint32(iov_ptr, true);
                                const buf_len = view.getUint32(iov_ptr + 4, true);
                                outStr += new TextDecoder().decode(memory.subarray(buf_ptr, buf_ptr + buf_len));
                                totalWritten += buf_len;
                            }
                            view.setUint32(nwritten_ptr, totalWritten, true);
                            
                            if (fd === 1 && window.__WASI_PROXY.redirect_stdout) {
                                const targetPath = getVfsPath(resolvePath(window.__WASI_PROXY.current_cwd || "/", window.__WASI_PROXY.redirect_stdout));
                                if (!vfs[targetPath]) {
                                    vfs[targetPath] = { type: "file", content: "", timestamp: Date.now() };
                                }
                                vfs[targetPath].content += outStr;
                                vfs[targetPath].timestamp = Date.now();
                                saveVfs();
                            } else {
                                if (fd === 1 || fd === 2) {
                                    // stdout or stderr
                                    const term_id = window.__WASI_PROXY.current_terminal_id;
                                    if (term_id && window.__WASI_PROXY.kernel) {
                                        for (let i = 0; i < outStr.length; i++) {
                                            window.__WASI_PROXY.kernel.kernel_wasi_print_char(window.__WASI_PROXY.kernelPtr, term_id, outStr.charCodeAt(i));
                                        }
                                    }
                                }
                            }
                            
                            return 0; // SUCCESS
                        }
                        return window.__WASI_PROXY.kernel.sys_fd_write(fd, iovs_ptr, iovs_len, nwritten_ptr);
                    }
                    
                    const openFd = window.__WASI_FDS.get(fd);
                    if (!openFd) return 8; // EBADF
                    const node = vfs[openFd.path];
                    if (!node || node.type !== "file") return 8; // EBADF

                    if (window.__WASI_PROXY.wasm) {
                        const view = new DataView(window.__WASI_PROXY.wasm.exports.memory.buffer);
                        const memory = new Uint8Array(window.__WASI_PROXY.wasm.exports.memory.buffer);
                        
                        let totalWritten = 0;
                        let newContent = node.content.substring(0, openFd.offset);
                        
                        for (let i = 0; i < iovs_len; i++) {
                            const iov_ptr = iovs_ptr + i * 8;
                            const buf_ptr = view.getUint32(iov_ptr, true);
                            const buf_len = view.getUint32(iov_ptr + 4, true);
                            
                            for (let j = 0; j < buf_len; j++) {
                                newContent += String.fromCharCode(memory[buf_ptr + j]);
                            }
                            totalWritten += buf_len;
                        }
                        
                        newContent += node.content.substring(openFd.offset + totalWritten);
                        node.content = newContent;
                        node.timestamp = Date.now();
                        openFd.offset += totalWritten;
                        
                        saveVfs();
                        
                        if (nwritten_ptr) {
                            view.setUint32(nwritten_ptr, totalWritten, true);
                        }
                        return 0; // SUCCESS
                    }
                    return 8; // EBADF
                };
            }
            if (prop === 'fd_read') {
                return function(fd, iovs_ptr, iovs_len, nread_ptr) {
                    if (fd <= 2) {
                        if (fd === 0 && window.__WASI_PROXY.redirect_stdin && window.__WASI_PROXY.wasm !== wasmInstance) {
                            const targetPath = getVfsPath(resolvePath(window.__WASI_PROXY.current_cwd || "/", window.__WASI_PROXY.redirect_stdin));
                            const node = vfs[targetPath];
                            if (!node || node.type !== "file") return 8; // EBADF
                            
                            // Initialize offset if not present
                            if (typeof window.__WASI_PROXY.redirect_stdin_offset === 'undefined') {
                                window.__WASI_PROXY.redirect_stdin_offset = 0;
                            }
                            
                            const view = new DataView(window.__WASI_PROXY.wasm.exports.memory.buffer);
                            const memory = new Uint8Array(window.__WASI_PROXY.wasm.exports.memory.buffer);
                            
                            let totalRead = 0;
                            for (let i = 0; i < iovs_len; i++) {
                                const iov_ptr = iovs_ptr + i * 8;
                                const buf_ptr = view.getUint32(iov_ptr, true);
                                const buf_len = view.getUint32(iov_ptr + 4, true);
                                
                                const remaining = node.content.length - window.__WASI_PROXY.redirect_stdin_offset;
                                if (remaining <= 0) break;
                                
                                const toRead = Math.min(buf_len, remaining);
                                for (let j = 0; j < toRead; j++) {
                                    memory[buf_ptr + j] = node.content.charCodeAt(window.__WASI_PROXY.redirect_stdin_offset + j);
                                }
                                
                                window.__WASI_PROXY.redirect_stdin_offset += toRead;
                                totalRead += toRead;
                                
                                if (toRead < buf_len) break; // EOF
                            }
                            
                            if (nread_ptr) {
                                view.setUint32(nread_ptr, totalRead, true);
                            }
                            return 0; // SUCCESS
                        }
                        return window.__WASI_PROXY.kernel.sys_fd_read(fd, iovs_ptr, iovs_len, nread_ptr);
                    }
                    
                    const openFd = window.__WASI_FDS.get(fd);
                    if (!openFd || openFd.type !== "file") return 8; // EBADF
                    
                    const node = vfs[openFd.path];
                    if (!node || node.type !== "file") return 8; // EBADF

                    if (window.__WASI_PROXY.wasm) {
                        const view = new DataView(window.__WASI_PROXY.wasm.exports.memory.buffer);
                        const memory = new Uint8Array(window.__WASI_PROXY.wasm.exports.memory.buffer);
                        
                        let totalRead = 0;
                        for (let i = 0; i < iovs_len; i++) {
                            const iov_ptr = iovs_ptr + i * 8;
                            const buf_ptr = view.getUint32(iov_ptr, true);
                            const buf_len = view.getUint32(iov_ptr + 4, true);
                            
                            const remaining = node.content.length - openFd.offset;
                            if (remaining <= 0) break;
                            
                            const toRead = Math.min(buf_len, remaining);
                            for (let j = 0; j < toRead; j++) {
                                memory[buf_ptr + j] = node.content.charCodeAt(openFd.offset + j);
                            }
                            
                            openFd.offset += toRead;
                            totalRead += toRead;
                            
                            if (toRead < buf_len) break; // EOF
                        }
                        
                        if (nread_ptr) {
                            view.setUint32(nread_ptr, totalRead, true);
                        }
                        return 0; // SUCCESS
                    }
                    return 8; // EBADF
                };
            }
            if (prop === 'proc_exit') {
                return function(code) { return 0; };
            }
            if (prop === 'environ_sizes_get') {
                return function(environ_count_ptr, environ_size_ptr) {
                    if (window.__WASI_PROXY.wasm) {
                        const view = new DataView(window.__WASI_PROXY.wasm.exports.memory.buffer);
                        const cwd = window.__WASI_PROXY.current_cwd || "/";
                        const envs = getEnvs(cwd);
                        let totalLen = 0;
                        for (let e of envs) totalLen += e.length + 1; // +1 for null byte
                        
                        view.setUint32(environ_count_ptr, envs.length, true);
                        view.setUint32(environ_size_ptr, totalLen, true);
                    }
                    return 0; // SUCCESS
                };
            }
            if (prop === 'environ_get') {
                return function(environ_ptr, environ_buf_ptr) {
                    if (window.__WASI_PROXY.wasm) {
                        const view = new DataView(window.__WASI_PROXY.wasm.exports.memory.buffer);
                        const memory = new Uint8Array(window.__WASI_PROXY.wasm.exports.memory.buffer);
                        const cwd = window.__WASI_PROXY.current_cwd || "/";
                        const envs = getEnvs(cwd);
                        
                        let current_buf_ptr = environ_buf_ptr;
                        let current_ptr = environ_ptr;
                        
                        for (let env of envs) {
                            view.setUint32(current_ptr, current_buf_ptr, true);
                            current_ptr += 4;
                            
                            const envStr = env + "\0";
                            for (let i = 0; i < envStr.length; i++) {
                                memory[current_buf_ptr + i] = envStr.charCodeAt(i);
                            }
                            current_buf_ptr += envStr.length;
                        }
                    }
                    return 0; // SUCCESS
                };
            }
            if (prop === 'args_sizes_get') {
                return function(argc_ptr, argv_buf_size_ptr) {
                    if (window.__WASI_PROXY.wasm) {
                        const view = new DataView(window.__WASI_PROXY.wasm.exports.memory.buffer);
                        const args = window.__WASI_PROXY.current_args || ["monkeyos"];
                        let totalSize = 0;
                        for (let arg of args) {
                            totalSize += arg.length + 1; // +1 for null byte
                        }
                        view.setUint32(argc_ptr, args.length, true);
                        view.setUint32(argv_buf_size_ptr, totalSize, true);
                    }
                    return 0; // SUCCESS
                };
            }
            if (prop === 'args_get') {
                return function(argv_ptr, argv_buf_ptr) {
                    if (window.__WASI_PROXY.wasm) {
                        const view = new DataView(window.__WASI_PROXY.wasm.exports.memory.buffer);
                        const memory = new Uint8Array(window.__WASI_PROXY.wasm.exports.memory.buffer);
                        const args = window.__WASI_PROXY.current_args || ["monkeyos"];
                        
                        let current_argv_buf_ptr = argv_buf_ptr;
                        let current_argv_ptr = argv_ptr;
                        
                        for (let arg of args) {
                            view.setUint32(current_argv_ptr, current_argv_buf_ptr, true);
                            current_argv_ptr += 4;
                            
                            const argStr = arg + "\0";
                            for (let i = 0; i < argStr.length; i++) {
                                memory[current_argv_buf_ptr + i] = argStr.charCodeAt(i);
                            }
                            current_argv_buf_ptr += argStr.length;
                        }
                    }
                    return 0; // SUCCESS
                };
            }
            if (prop === 'path_open') {
                return function(fd, dirflags, path_ptr, path_len, oflags, fs_rights_base, fs_rights_inheriting, fdflags, opened_fd_ptr) {
                    let basePath = "/";
                    if (fd !== 3) {
                        const openFd = window.__WASI_FDS.get(fd);
                        if (!openFd || openFd.type !== "dir") return 8; // EBADF
                        basePath = openFd.path;
                    }

                    if (window.__WASI_PROXY.wasm) {
                        const view = new DataView(window.__WASI_PROXY.wasm.exports.memory.buffer);
                        const memory = new Uint8Array(window.__WASI_PROXY.wasm.exports.memory.buffer);
                        let pathStr = "";
                        for (let i = 0; i < path_len; i++) {
                            pathStr += String.fromCharCode(memory[path_ptr + i]);
                        }
                        
                        const fullPath = getVfsPath(resolvePath(basePath, pathStr));
                        const node = vfs[fullPath];
                        
                        // Check if file doesn't exist but we want to create it
                        if (!node) {
                            // If O_CREAT flag is set (oflags & 1)
                            if (oflags & 1) {
                                // Create new empty file
                                const parentPath = getVfsPath(resolvePath(fullPath, ".."));
                                const parent = vfs[parentPath];
                                if (parent && parent.type === "dir") {
                                    const filename = fullPath.substring(parentPath.length === 1 ? 1 : parentPath.length + 1);
                                    if (!parent.children.includes(filename)) {
                                        parent.children.push(filename);
                                    }
                                    vfs[fullPath] = { type: "file", content: "", timestamp: Date.now() };
                                    saveVfs();
                                } else {
                                    return 44; // ENOENT
                                }
                            } else {
                                return 44; // ENOENT
                            }
                        } else if (node.type === "file" && (oflags & 8)) {
                            // O_TRUNC is set
                            node.content = "";
                            node.timestamp = Date.now();
                            saveVfs();
                        }

                        const newFd = nextFd++;
                        window.__WASI_FDS.set(newFd, {
                            path: fullPath,
                            type: vfs[fullPath] ? vfs[fullPath].type : "file", // in case it was just created
                            offset: 0
                        });
                        
                        view.setUint32(opened_fd_ptr, newFd, true);
                        return 0; // SUCCESS
                    }
                    return 8; // EBADF
                };
            }
            if (prop === 'fd_readdir') {
                return function(fd, buf, buf_len, cookie, bufused_ptr) {
                    const openFd = fd === 3 ? { path: "/", type: "dir", offset: Number(cookie) } : window.__WASI_FDS.get(fd);
                    if (!openFd || openFd.type !== "dir") return 8; // EBADF
                    
                    const node = vfs[openFd.path];
                    if (!node || node.type !== "dir") return 20; // ENOTDIR

                    if (window.__WASI_PROXY.wasm) {
                        const view = new DataView(window.__WASI_PROXY.wasm.exports.memory.buffer);
                        const memory = new Uint8Array(window.__WASI_PROXY.wasm.exports.memory.buffer);
                        
                        let cookieIdx = Number(cookie);
                        let bufUsed = 0;
                        
                        while (cookieIdx < node.children.length) {
                            const filename = node.children[cookieIdx];
                            const entrySize = 24 + filename.length;
                            
                            if (bufUsed + entrySize > buf_len) {
                                break; // Buffer is full
                            }
                            
                            const entryPtr = buf + bufUsed;
                            view.setBigUint64(entryPtr, BigInt(cookieIdx + 1), true); // d_next
                            view.setBigUint64(entryPtr + 8, 100n + BigInt(cookieIdx), true); // d_ino
                            view.setUint32(entryPtr + 16, filename.length, true); // d_namlen
                            
                            const childPath = getVfsPath(resolvePath(openFd.path, filename));
                            const childNode = vfs[childPath];
                            view.setUint8(entryPtr + 20, childNode && childNode.type === "dir" ? 3 : 4); // d_type
                            
                            for (let i = 0; i < filename.length; i++) {
                                memory[entryPtr + 24 + i] = filename.charCodeAt(i);
                            }
                            
                            bufUsed += entrySize;
                            cookieIdx++;
                        }
                        
                        view.setUint32(bufused_ptr, bufUsed, true);
                        if (fd !== 3) openFd.offset = cookieIdx;
                        return 0; // SUCCESS
                    }
                    return 8; // EBADF
                };
            }
            if (prop === 'path_filestat_get') {
                return function(fd, flags, path_ptr, path_len, buf_ptr) {
                    let basePath = "/";
                    if (fd !== 3) {
                        const openFd = window.__WASI_FDS.get(fd);
                        if (!openFd || openFd.type !== "dir") return 8;
                        basePath = openFd.path;
                    }

                    if (window.__WASI_PROXY.wasm) {
                        const view = new DataView(window.__WASI_PROXY.wasm.exports.memory.buffer);
                        const memory = new Uint8Array(window.__WASI_PROXY.wasm.exports.memory.buffer);
                        let pathStr = "";
                        for (let i = 0; i < path_len; i++) {
                            pathStr += String.fromCharCode(memory[path_ptr + i]);
                        }
                        
                        const fullPath = getVfsPath(resolvePath(basePath, pathStr));
                        const node = vfs[fullPath];
                        if (!node) return 44; // ENOENT
                        
                        view.setUint8(buf_ptr + 16, node.type === "dir" ? 3 : 4); // filetype
                        view.setBigUint64(buf_ptr + 24, 1n, true); // nlink = 1
                        view.setBigUint64(buf_ptr + 32, node.type === "file" ? BigInt(node.content.length) : 0n, true); // size
                        
                        const timeNs = BigInt(node.timestamp || 0) * 1000000n;
                        view.setBigUint64(buf_ptr + 40, timeNs, true); // atim
                        view.setBigUint64(buf_ptr + 48, timeNs, true); // mtim
                        view.setBigUint64(buf_ptr + 56, timeNs, true); // ctim
                        return 0; // SUCCESS
                    }
                    return 8; // EBADF
                };
            }
            if (prop === 'fd_filestat_get') {
                return function(fd, buf_ptr) {
                    if (window.__WASI_PROXY.wasm) {
                        const view = new DataView(window.__WASI_PROXY.wasm.exports.memory.buffer);
                        const openFd = window.__WASI_FDS.get(fd);
                        if (!openFd) {
                            if (fd === 3) {
                                view.setUint8(buf_ptr + 16, 3); // dir
                                return 0;
                            }
                            return 8;
                        }
                        const node = vfs[openFd.path];
                        if (!node) return 8;

                        view.setUint8(buf_ptr + 16, node.type === "dir" ? 3 : 4); // filetype
                        view.setBigUint64(buf_ptr + 24, 1n, true); // nlink
                        view.setBigUint64(buf_ptr + 32, node.type === "file" ? BigInt(node.content.length) : 0n, true); // size
                        
                        const timeNs = BigInt(node.timestamp || 0) * 1000000n;
                        view.setBigUint64(buf_ptr + 40, timeNs, true); // atim
                        view.setBigUint64(buf_ptr + 48, timeNs, true); // mtim
                        view.setBigUint64(buf_ptr + 56, timeNs, true); // ctim
                        return 0; // SUCCESS
                    }
                    return 8; // EBADF
                };
            }
            if (prop === 'fd_prestat_get') {
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
            }
            if (prop === 'fd_prestat_dir_name') {
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
            }
            if (prop === 'fd_fdstat_get') {
                return function(fd, stat_ptr) {
                    if (window.__WASI_PROXY.wasm && stat_ptr) {
                        const view = new DataView(window.__WASI_PROXY.wasm.exports.memory.buffer);
                        if (fd <= 2) {
                            view.setUint8(stat_ptr, 2); // fs_filetype = character device
                            view.setUint16(stat_ptr + 2, 0, true); // fs_flags
                            return 0; // SUCCESS
                        } else if (fd === 3) {
                            view.setUint8(stat_ptr, 3); // fs_filetype = directory
                            view.setUint16(stat_ptr + 2, 0, true); // fs_flags
                            return 0; // SUCCESS
                        }
                        
                        const openFd = window.__WASI_FDS.get(fd);
                        if (openFd) {
                            view.setUint8(stat_ptr, openFd.type === "dir" ? 3 : 4);
                            view.setUint16(stat_ptr + 2, 0, true);
                            return 0;
                        }
                    }
                    return 8; // EBADF
                }
            }
            if (prop === 'random_get') {
                return function(buf_ptr, buf_len) {
                    if (window.__WASI_PROXY.wasm) {
                        const memory = new Uint8Array(window.__WASI_PROXY.wasm.exports.memory.buffer);
                        for (let i = 0; i < buf_len; i++) {
                            memory[buf_ptr + i] = Math.floor(Math.random() * 256);
                        }
                        return 0; // SUCCESS
                    }
                    return 52; // ENOSYS
                };
            }
            if (prop === 'clock_time_get') {
                return function(clock_id, precision, time_ptr) {
                    if (window.__WASI_PROXY.wasm) {
                        const view = new DataView(window.__WASI_PROXY.wasm.exports.memory.buffer);
                        const now = BigInt(Date.now()) * 1000000n;
                        view.setBigUint64(time_ptr, now, true);
                        return 0; // SUCCESS
                    }
                    return 52; // ENOSYS
                };
            }
            if (prop === 'fd_close') {
                return function(fd) {
                    if (window.__WASI_FDS.has(fd)) {
                        window.__WASI_FDS.delete(fd);
                        return 0; // SUCCESS
                    }
                    return 8; // EBADF
                };
            }
            if (prop === 'fd_seek') {
                return function(fd, offset, whence, newoffset_ptr) {
                    const openFd = window.__WASI_FDS.get(fd);
                    if (!openFd) return 8; // EBADF
                    const node = vfs[openFd.path];
                    if (!node || node.type !== "file") return 28; // EINVAL
                    
                    let newOffset;
                    if (whence === 0) newOffset = Number(offset); // SET
                    else if (whence === 1) newOffset = openFd.offset + Number(offset); // CUR
                    else if (whence === 2) newOffset = node.content.length + Number(offset); // END
                    else return 28; // EINVAL
                    
                    openFd.offset = newOffset;
                    if (window.__WASI_PROXY.wasm && newoffset_ptr) {
                        const view = new DataView(window.__WASI_PROXY.wasm.exports.memory.buffer);
                        view.setBigUint64(newoffset_ptr, BigInt(newOffset), true);
                    }
                    return 0; // SUCCESS
                };
            }
            if (prop === 'path_readlink') {
                return function(fd, path_ptr, path_len, buf_ptr, buf_len, bufused_ptr) {
                    return 28; // EINVAL (not a symlink)
                };
            }
            if (prop === 'path_create_directory') {
                return function(fd, path_ptr, path_len) {
                    let basePath = "/";
                    if (fd !== 3) {
                        const openFd = window.__WASI_FDS.get(fd);
                        if (!openFd || openFd.type !== "dir") return 8;
                        basePath = openFd.path;
                    }
                    if (window.__WASI_PROXY.wasm) {
                        const memory = new Uint8Array(window.__WASI_PROXY.wasm.exports.memory.buffer);
                        let pathStr = "";
                        for (let i = 0; i < path_len; i++) pathStr += String.fromCharCode(memory[path_ptr + i]);
                        
                        const fullPath = getVfsPath(resolvePath(basePath, pathStr));
                        if (vfs[fullPath]) return 17; // EEXIST
                        
                        const parentPath = getVfsPath(resolvePath(fullPath, ".."));
                        const parent = vfs[parentPath];
                        if (!parent || parent.type !== "dir") return 44; // ENOENT
                        
                        const filename = fullPath.substring(parentPath.length === 1 ? 1 : parentPath.length + 1);
                        parent.children.push(filename);
                        vfs[fullPath] = { type: "dir", children: [], timestamp: Date.now() };
                        saveVfs();
                        return 0; // SUCCESS
                    }
                    return 8;
                };
            }
            if (prop === 'path_unlink_file' || prop === 'path_remove_directory') {
                return function(fd, path_ptr, path_len) {
                    let basePath = "/";
                    if (fd !== 3) {
                        const openFd = window.__WASI_FDS.get(fd);
                        if (!openFd || openFd.type !== "dir") return 8;
                        basePath = openFd.path;
                    }
                    if (window.__WASI_PROXY.wasm) {
                        const memory = new Uint8Array(window.__WASI_PROXY.wasm.exports.memory.buffer);
                        let pathStr = "";
                        for (let i = 0; i < path_len; i++) pathStr += String.fromCharCode(memory[path_ptr + i]);
                        
                        const fullPath = getVfsPath(resolvePath(basePath, pathStr));
                        const node = vfs[fullPath];
                        if (!node) return 44; // ENOENT
                        
                        if (prop === 'path_remove_directory') {
                            if (node.type !== "dir") return 20; // ENOTDIR
                            if (node.children.length > 0) return 39; // ENOTEMPTY
                        } else {
                            if (node.type === "dir") return 21; // EISDIR
                        }
                        
                        const parentPath = getVfsPath(resolvePath(fullPath, ".."));
                        const parent = vfs[parentPath];
                        if (parent && parent.type === "dir") {
                            const filename = fullPath.substring(parentPath.length === 1 ? 1 : parentPath.length + 1);
                            parent.children = parent.children.filter(c => c !== filename);
                        }
                        delete vfs[fullPath];
                        saveVfs();
                        return 0; // SUCCESS
                    }
                    return 8;
                };
            }
            if (prop === 'path_rename') {
                return function(fd, old_path_ptr, old_path_len, new_fd, new_path_ptr, new_path_len) {
                    let oldBasePath = "/";
                    let newBasePath = "/";
                    
                    if (fd !== 3) {
                        const openFd = window.__WASI_FDS.get(fd);
                        if (!openFd || openFd.type !== "dir") return 8;
                        oldBasePath = openFd.path;
                    }
                    if (new_fd !== 3) {
                        const openFd = window.__WASI_FDS.get(new_fd);
                        if (!openFd || openFd.type !== "dir") return 8;
                        newBasePath = openFd.path;
                    }

                    if (window.__WASI_PROXY.wasm) {
                        const memory = new Uint8Array(window.__WASI_PROXY.wasm.exports.memory.buffer);
                        let oldPathStr = "";
                        for (let i = 0; i < old_path_len; i++) oldPathStr += String.fromCharCode(memory[old_path_ptr + i]);
                        let newPathStr = "";
                        for (let i = 0; i < new_path_len; i++) newPathStr += String.fromCharCode(memory[new_path_ptr + i]);
                        
                        const oldFullPath = getVfsPath(resolvePath(oldBasePath, oldPathStr));
                        const newFullPath = getVfsPath(resolvePath(newBasePath, newPathStr));
                        
                        const node = vfs[oldFullPath];
                        if (!node) return 44; // ENOENT
                        if (vfs[newFullPath]) return 17; // EEXIST
                        
                        const oldParentPath = getVfsPath(resolvePath(oldFullPath, ".."));
                        const newParentPath = getVfsPath(resolvePath(newFullPath, ".."));
                        const oldParent = vfs[oldParentPath];
                        const newParent = vfs[newParentPath];
                        
                        if (!oldParent || oldParent.type !== "dir" || !newParent || newParent.type !== "dir") return 44; // ENOENT
                        
                        const oldFilename = oldFullPath.substring(oldParentPath.length === 1 ? 1 : oldParentPath.length + 1);
                        const newFilename = newFullPath.substring(newParentPath.length === 1 ? 1 : newParentPath.length + 1);
                        
                        oldParent.children = oldParent.children.filter(c => c !== oldFilename);
                        newParent.children.push(newFilename);
                        
                        vfs[newFullPath] = node;
                        delete vfs[oldFullPath];
                        
                        // We also need to rename all children if it's a directory
                        if (node.type === "dir") {
                            const toRename = Object.keys(vfs).filter(p => p.startsWith(oldFullPath + "/"));
                            for (let p of toRename) {
                                const newP = newFullPath + p.substring(oldFullPath.length);
                                vfs[newP] = vfs[p];
                                delete vfs[p];
                            }
                        }
                        
                        saveVfs();
                        return 0; // SUCCESS
                    }
                    return 8;
                };
            }
            if (prop === 'path_filestat_set_times') {
                return function(fd, flags, path_ptr, path_len, atim, mtim, fst_flags) {
                    let basePath = "/";
                    if (fd !== 3) {
                        const openFd = window.__WASI_FDS.get(fd);
                        if (!openFd || openFd.type !== "dir") return 8;
                        basePath = openFd.path;
                    }
                    if (window.__WASI_PROXY.wasm) {
                        const memory = new Uint8Array(window.__WASI_PROXY.wasm.exports.memory.buffer);
                        let pathStr = "";
                        for (let i = 0; i < path_len; i++) {
                            pathStr += String.fromCharCode(memory[path_ptr + i]);
                        }
                        const fullPath = getVfsPath(resolvePath(basePath, pathStr));
                        const node = vfs[fullPath];
                        if (!node) return 44; // ENOENT
                        node.timestamp = Date.now();
                        saveVfs();
                        return 0; // SUCCESS
                    }
                    return 8;
                };
            }
            if (prop === 'fd_filestat_set_times') {
                return function(fd, atim, mtim, fst_flags) {
                    const openFd = window.__WASI_FDS.get(fd);
                    if (!openFd) return 8;
                    const node = vfs[openFd.path];
                    if (node) {
                        node.timestamp = Date.now();
                        saveVfs();
                    }
                    return 0;
                };
            }
            return function(...args) {
                console.log("UNIMPLEMENTED WASI STUB CALLED: " + prop + " with args: " + JSON.stringify(args, (k, v) => typeof v === 'bigint' ? v.toString() + 'n' : v));
                // Return ENOSYS for unimplemented functions
                return 52; 
            };
        }
    });

    let wasmInstance;

    const env = {
        console_log: (ptr, len) => {
            const memory = new Uint8Array(wasmInstance.exports.memory.buffer);
            const str = new TextDecoder().decode(memory.subarray(ptr, ptr + len));
            console.log(str);
        },
        sys_time_ms: () => {
            return BigInt(Date.now());
        },
        sys_timezone_offset_ms: () => {
            return BigInt(new Date().getTimezoneOffset() * 60 * 1000);
        },
        wasi_print_js: (id, ptr, len) => {
            const wasm = window.__WASI_PROXY.wasm || wasmInstance;
            const memory = new Uint8Array(wasm.exports.memory.buffer);
            const str = new TextDecoder().decode(memory.subarray(ptr, ptr + len));
            wasi_print_js(id, str);
        },
        draw_rect_js: (x, y, w, h, r, g, b, a, radius, shadow_blur) => window.draw_rect_js(x, y, w, h, r, g, b, a, radius, shadow_blur),
        draw_text_js: (x, y, ptr, len, font_size, r, g, b, a) => {
            if (!window.textCtx) {
                const tc = document.getElementById("text-overlay");
                if (tc) {
                    tc.width = window.innerWidth;
                    tc.height = window.innerHeight;
                    window.textCtx = tc.getContext("2d", { alpha: true });
                    window.textCtxCache = { font: "", fillStyle: "", baseline: "" };
                }
            }
            if (!window.globalTextDecoder) {
                window.globalTextDecoder = new TextDecoder();
            }
            if (!window.textCtxCache) {
                window.textCtxCache = { font: "", fillStyle: "", baseline: "" };
            }
            if (!window.textCtx) return;
            const wasm = window.__WASI_PROXY.wasm || wasmInstance;
            const memory = new Uint8Array(wasm.exports.memory.buffer);
            const text = window.globalTextDecoder.decode(memory.subarray(ptr, ptr + len));
            
            const font = `${font_size}px monospace`;
            if (window.textCtxCache.font !== font) {
                window.textCtx.font = font;
                window.textCtxCache.font = font;
            }
            
            const fillStyle = `rgba(${r * 255}, ${g * 255}, ${b * 255}, ${a})`;
            if (window.textCtxCache.fillStyle !== fillStyle) {
                window.textCtx.fillStyle = fillStyle;
                window.textCtxCache.fillStyle = fillStyle;
            }
            
            if (window.textCtxCache.baseline !== 'top') {
                window.textCtx.textBaseline = 'top';
                window.textCtxCache.baseline = 'top';
            }
            
            window.textCtx.fillText(text, x, y);
        },
        measure_text_js: (ptr, len, font_size) => {
            if (!window.textCtx) {
                const tc = document.getElementById("text-overlay");
                if (tc) {
                    window.textCtx = tc.getContext("2d", { alpha: true });
                }
            }
            if (!window.globalTextDecoder) {
                window.globalTextDecoder = new TextDecoder();
            }
            if (!window.textCtx) return 0.0;
            const wasm = window.__WASI_PROXY.wasm || wasmInstance;
            const memory = new Uint8Array(wasm.exports.memory.buffer);
            const text = window.globalTextDecoder.decode(memory.subarray(ptr, ptr + len));
            
            const font = `${font_size}px monospace`;
            if (!window.textCtxCache) {
                window.textCtxCache = { font: "", fillStyle: "", baseline: "" };
            }
            if (window.textCtxCache.font !== font) {
                window.textCtx.font = font;
                window.textCtxCache.font = font;
            }
            
            return window.textCtx.measureText(text).width;
        },
        draw_centered_text_js: (x, y, ptr, len, font_size, r, g, b, a) => {
            if (!window.textCtx) {
                const tc = document.getElementById("text-overlay");
                if (tc) {
                    tc.width = window.innerWidth;
                    tc.height = window.innerHeight;
                    window.textCtx = tc.getContext("2d", { alpha: true });
                    window.textCtxCache = { font: "", fillStyle: "", baseline: "" };
                }
            }
            if (!window.globalTextDecoder) {
                window.globalTextDecoder = new TextDecoder();
            }
            if (!window.textCtxCache) {
                window.textCtxCache = { font: "", fillStyle: "", baseline: "" };
            }
            if (!window.textCtx) return;
            const wasm = window.__WASI_PROXY.wasm || wasmInstance;
            const memory = new Uint8Array(wasm.exports.memory.buffer);
            const text = window.globalTextDecoder.decode(memory.subarray(ptr, ptr + len));
            
            const font = `${font_size}px monospace`;
            if (window.textCtxCache.font !== font) {
                window.textCtx.font = font;
                window.textCtxCache.font = font;
            }
            
            const fillStyle = `rgba(${r * 255}, ${g * 255}, ${b * 255}, ${a})`;
            if (window.textCtxCache.fillStyle !== fillStyle) {
                window.textCtx.fillStyle = fillStyle;
                window.textCtxCache.fillStyle = fillStyle;
            }
            
            window.textCtx.textBaseline = 'middle';
            window.textCtx.textAlign = 'center';
            window.textCtxCache.baseline = 'middle'; // invalidate cache for normal text
            
            window.textCtx.fillText(text, x, y);
            
            // Restore alignment
            window.textCtx.textAlign = 'start';
        },
        clip_text_js: (x, y, w, h) => {
            if (window.textCtx) {
                window.textCtx.save();
                window.textCtx.beginPath();
                window.textCtx.rect(x, y, w, h);
                window.textCtx.clip();
            }
        },
        clear_clip_text_js: () => {
            if (window.textCtx) {
                window.textCtx.restore();
            }
        },
        clear_text_js: () => {
            if (!window.textCtx) {
                const tc = document.getElementById("text-overlay");
                if (tc) {
                    tc.width = window.innerWidth;
                    tc.height = window.innerHeight;
                    window.textCtx = tc.getContext("2d");
                }
            }
            if (window.textCtx) {
                window.textCtx.clearRect(0, 0, window.innerWidth, window.innerHeight);
            }
        },
        clear_screen_js: () => window.clear_screen_js(),
        draw_gui_app_js: (id, x, y, w, h) => {
            const app = window.gui_apps.find(a => a.__window_id === id);
            if (app && app.exports.tick) {
                const prev = window.__WASI_PROXY.wasm;
                window.__WASI_PROXY.wasm = app;
                app.exports.tick(x, y, w, h);
                window.__WASI_PROXY.wasm = prev;
            }
        },
        gui_app_mouse_move_js: (id, mx, my) => {
            const app = window.gui_apps.find(a => a.__window_id === id);
            if (app && app.exports.handle_mouse_move) {
                const prev = window.__WASI_PROXY.wasm;
                window.__WASI_PROXY.wasm = app;
                const ret = app.exports.handle_mouse_move(mx, my);
                window.__WASI_PROXY.wasm = prev;
                return ret;
            }
            return 0;
        },
        gui_app_mouse_down_js: (id, mx, my) => {
            const app = window.gui_apps.find(a => a.__window_id === id);
            if (app && app.exports.handle_mouse_down) {
                const prev = window.__WASI_PROXY.wasm;
                window.__WASI_PROXY.wasm = app;
                const ret = app.exports.handle_mouse_down(mx, my);
                window.__WASI_PROXY.wasm = prev;
                return ret;
            }
            return 0;
        },
        gui_app_mouse_up_js: (id, mx, my) => {
            const app = window.gui_apps.find(a => a.__window_id === id);
            if (app && app.exports.handle_mouse_up) {
                const prev = window.__WASI_PROXY.wasm;
                window.__WASI_PROXY.wasm = app;
                const ret = app.exports.handle_mouse_up(mx, my);
                window.__WASI_PROXY.wasm = prev;
                return ret;
            }
            return 0;
        },
        gui_app_key_down_js: (id, key_code) => {
            const app = window.gui_apps.find(a => a.__window_id === id);
            if (app && app.exports.handle_key_down) {
                const prev = window.__WASI_PROXY.wasm;
                window.__WASI_PROXY.wasm = app;
                const ret = app.exports.handle_key_down(key_code);
                window.__WASI_PROXY.wasm = prev;
                return ret;
            }
            return 0;
        },
        sys_create_window: (x, y, w, h, app_type) => {
            if (wasmInstance && wasmInstance.exports.kernel_create_window && window.__WASI_PROXY.kernelPtr) {
                const id = wasmInstance.exports.kernel_create_window(window.__WASI_PROXY.kernelPtr, x, y, w, h, app_type);
                if (window.__WASI_PROXY.wasm) {
                    window.__WASI_PROXY.wasm.__window_id = id;
                }
                return id;
            }
            return 0;
        },
        sys_execve: (args_ptr, args_len, cwd_ptr, cwd_len, stdin_ptr, stdin_len, stdout_ptr, stdout_len, terminal_id) => {
            if (!window.__WASI_PROXY.wasm) return -1;
            const memory = new Uint8Array(window.__WASI_PROXY.wasm.exports.memory.buffer);
            let argsStr = "";
            for (let i = 0; i < args_len; i++) {
                argsStr += String.fromCharCode(memory[args_ptr + i]);
            }
            
            let cwdStr = "";
            for (let i = 0; i < cwd_len; i++) {
                cwdStr += String.fromCharCode(memory[cwd_ptr + i]);
            }

            let stdinStr = "";
            for (let i = 0; i < stdin_len; i++) {
                stdinStr += String.fromCharCode(memory[stdin_ptr + i]);
            }
            let stdoutStr = "";
            for (let i = 0; i < stdout_len; i++) {
                stdoutStr += String.fromCharCode(memory[stdout_ptr + i]);
            }
            
            const args = argsStr.split('\0').filter(s => s.length > 0);
            if (args.length === 0) return -1;
            
            const pathStr = args[0];
            window.__WASI_PROXY.current_args = args;
            window.__WASI_PROXY.current_cwd = cwdStr;
            window.__WASI_PROXY.redirect_stdin = stdinStr.length > 0 ? stdinStr : null;
            window.__WASI_PROXY.redirect_stdout = stdoutStr.length > 0 ? stdoutStr : null;
            window.__WASI_PROXY.redirect_stdin_offset = 0;
            
            if (window.__WASI_PROXY.redirect_stdout) {
                const targetPath = getVfsPath(resolvePath(window.__WASI_PROXY.current_cwd || "/", window.__WASI_PROXY.redirect_stdout));
                const parentPath = getVfsPath(resolvePath(targetPath, ".."));
                const parent = vfs[parentPath];
                if (parent && parent.type === "dir") {
                    const filename = targetPath.substring(parentPath.length === 1 ? 1 : parentPath.length + 1);
                    if (!parent.children.includes(filename)) {
                        parent.children.push(filename);
                    }
                }
                vfs[targetPath] = { type: "file", content: "", timestamp: Date.now() };
                saveVfs();
            }

            const node = vfs[pathStr];
            if (!node || node.type !== "executable") {
                console.error("sys_execve: Executable not found in VFS: " + pathStr);
                return -1;
            }
            
            try {
                // Synchronously instantiate the binary using the pre-compiled module if available
                const childModule = node.module || new WebAssembly.Module(node.binary);
                const childInstance = new WebAssembly.Instance(childModule, {
                    wasi_snapshot_preview1: wasi_snapshot_preview1,
                    env: env
                });
                
                // Context Switch!
                const parentWasm = window.__WASI_PROXY.wasm;
                const prevCwd = window.__WASI_PROXY.current_cwd;
                const prevStdout = window.__WASI_PROXY.redirect_stdout;
                const prevStdin = window.__WASI_PROXY.redirect_stdin;
                const prevTerminalId = window.__WASI_PROXY.current_terminal_id;
                
                window.__WASI_PROXY.wasm = childInstance;
                window.__WASI_PROXY.current_terminal_id = (terminal_id !== undefined && terminal_id !== 0) ? terminal_id : prevTerminalId;
                
                try {
                    if (childInstance.exports.tick) {
                        if (childInstance.exports.init) childInstance.exports.init();
                        window.gui_apps.push(childInstance);
                    } else if (childInstance.exports._start) {
                        childInstance.exports._start();
                    }
                } catch (e) {
                    if (e.message !== "unreachable") {
                        console.error("Process exited with exception:", e);
                    }
                } finally {
                    window.__WASI_PROXY.wasm = parentWasm;
                    window.__WASI_PROXY.current_cwd = prevCwd;
                    window.__WASI_PROXY.redirect_stdout = prevStdout;
                    window.__WASI_PROXY.redirect_stdin = prevStdin;
                    window.__WASI_PROXY.current_terminal_id = prevTerminalId;
                }
                return 0; // SUCCESS
            } catch (e) {
                console.error("Failed to execute process:", e);
                return -1;
            }
        }
    };

    const response = await fetch('./kernel.wasm?t=' + Date.now());
    const wasmBytes = await response.arrayBuffer();
    const result = await WebAssembly.instantiate(wasmBytes, {
        wasi_snapshot_preview1,
        env
    });
    
    // Install kernel (no need to save to VFS DB since it's the core OS)
    vfs["/kernel"] = { type: "executable", binary: wasmBytes, module: await WebAssembly.compile(wasmBytes), timestamp: Date.now() };

    // Compile any cached executables in the VFS
    for (const path in vfs) {
        const node = vfs[path];
        if (node.type === "executable" && node.binary && !node.module) {
            try {
                node.module = await WebAssembly.compile(node.binary);
            } catch (e) {
                console.error("Failed to compile WebAssembly module for " + path, e);
            }
        }
    }
    
    window.__WASI_PROXY.wasm = result.instance; // Must be set before any Rust code runs!
    wasmInstance = result.instance;
    const exports = wasmInstance.exports;
    
    const kernelPtr = exports.kernel_new();
    window.__WASI_PROXY.kernelPtr = kernelPtr;

    const kernel = {
        tick: () => exports.kernel_tick(kernelPtr),
        push_mouse_move: (x, y) => exports.kernel_push_mouse_move(kernelPtr, x, y),
        push_mouse_button: (down) => exports.kernel_push_mouse_button(kernelPtr, down),
        push_key_event: (code) => exports.kernel_push_key_event(kernelPtr, code),
        push_screen_size: (w, h) => {
            if (exports.kernel_push_screen_size) {
                exports.kernel_push_screen_size(kernelPtr, w, h);
            }
        },
        sys_fd_write: (fd, iovs_ptr, iovs_len, nwritten_ptr) => exports.sys_fd_write(fd, iovs_ptr, iovs_len, nwritten_ptr),
        sys_fd_read: (fd, iovs_ptr, iovs_len, nread_ptr) => exports.sys_fd_read(fd, iovs_ptr, iovs_len, nread_ptr),
        kernel_wasi_print_char: (kernelPtr, term_id, charCode) => exports.kernel_wasi_print_char(kernelPtr, term_id, charCode),
    };
    
    window.__WASI_PROXY.kernel = kernel;
    kernel.push_screen_size(window.innerWidth, window.innerHeight);

    const gpuContext = await initWebGPU();
    window.gui_apps = [];
    if (!gpuContext) {
        document.body.innerHTML = "<h1 style='color: red; text-align: center; margin-top: 20%;'>WebGPU is required for MonkeyOS.</h1>";
        return;
    }

    window.gpuDirty = true;
    function loop() {
        kernel.tick();
        if (window.gpuDirty) {
            gpuContext.renderWebGPU();
            window.gpuDirty = false;
        }
        requestAnimationFrame(loop);
    }
    requestAnimationFrame(loop);

    window.addEventListener('mousemove', (e) => kernel.push_mouse_move(Math.round(e.clientX), Math.round(e.clientY)));
    window.addEventListener('mousedown', (e) => { if (e.button === 0) kernel.push_mouse_button(true); });
    window.addEventListener('mouseup', (e) => { if (e.button === 0) kernel.push_mouse_button(false); });
    window.addEventListener('keydown', (e) => {
        if (e.ctrlKey) {
            if (e.key === 's' || e.key === 'S') {
                e.preventDefault();
                kernel.push_key_event(19);
                return;
            }
            if (e.key === 'q' || e.key === 'Q') {
                e.preventDefault();
                kernel.push_key_event(17);
                return;
            }
        }
        
        if (e.key.length === 1) {
            kernel.push_key_event(e.key.charCodeAt(0));
        } else if (e.key === 'Enter') {
            kernel.push_key_event(13);
        } else if (e.key === 'Backspace') {
            kernel.push_key_event(8);
        } else if (e.key === 'ArrowLeft') {
            kernel.push_key_event(1037);
        } else if (e.key === 'ArrowRight') {
            kernel.push_key_event(1039);
        } else if (e.key === 'ArrowUp') {
            kernel.push_key_event(1038);
        } else if (e.key === 'ArrowDown') {
            kernel.push_key_event(1040);
        }
    });

    console.log = originalLog;
}

// HTML Overlay API for the Kernel

window.append_html_overlay_text_js = function(id, text) {
    const div = document.getElementById('overlay-' + id);
    if (div) {
        let newText = div.textContent;
        for (let i = 0; i < text.length; i++) {
            if (text[i] === '\x08') {
                newText = newText.slice(0, -1);
            } else {
                newText += text[i];
            }
        }
        div.textContent = newText;
        div.scrollTop = div.scrollHeight;
    } else {
        pendingText[id] = (pendingText[id] || "") + text;
    }
};

window.update_html_overlay_input_line_js = function(id, prompt, input, cursor_pos) {
    const div = document.getElementById('overlay-' + id);
    
    let beforeCursor = escapeHtml(input.substring(0, cursor_pos));
    let cursorChar = escapeHtml(input.substring(cursor_pos, cursor_pos + 1) || ' ');
    let afterCursor = escapeHtml(input.substring(cursor_pos + 1));
    
    let htmlLine = escapeHtml(prompt) + beforeCursor + '<span class="cursor">' + cursorChar + '</span>' + afterCursor;

    if (div) {
        let innerHtml = div.innerHTML;
        // Find the last newline. In innerHTML with pre-wrap, newlines are preserved as \n
        let lastNewlineIndex = innerHtml.lastIndexOf('\n');
        if (lastNewlineIndex !== -1) {
            div.innerHTML = innerHtml.substring(0, lastNewlineIndex + 1) + htmlLine;
        } else {
            div.innerHTML = htmlLine;
        }
        div.scrollTop = div.scrollHeight;
    } else {
        pendingLastLine[id] = { prompt, input, cursor_pos };
    }
};

window.draw_editor_js = function(id, content, cursor_pos) {
    const div = document.getElementById('overlay-' + id);
    if (!div) return;

    let beforeCursor = escapeHtml(content.substring(0, cursor_pos));
    
    let cursorCharStr = content.substring(cursor_pos, cursor_pos + 1);
    let afterCursorStr = content.substring(cursor_pos + 1);
    if (cursorCharStr === '\n') {
        cursorCharStr = ' ';
        afterCursorStr = '\n' + afterCursorStr;
    } else if (!cursorCharStr) {
        cursorCharStr = ' ';
    }
    
    let cursorChar = escapeHtml(cursorCharStr);
    let afterCursor = escapeHtml(afterCursorStr);
    
    let htmlContent = "--- Edit Mode (Ctrl+S to save, Ctrl+Q to quit) ---\n\n" + beforeCursor + '<span class="cursor">' + cursorChar + '</span>' + afterCursor;
    div.innerHTML = htmlContent;
};

window.clear_html_overlay_text_js = function(id) {
    const div = document.getElementById('overlay-' + id);
    if (div) {
        div.innerHTML = '';
    } else {
        pendingText[id] = '';
        delete pendingLastLine[id];
    }
};

bootstrap().catch(console.error);

window.destroy_html_overlay_js = function(id) {
    const div = document.getElementById('overlay-' + id);
    if (div) {
        div.remove();
    }
};
