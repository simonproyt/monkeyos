import puppeteer from 'puppeteer';

(async () => {
    try {
        console.log("Launching browser...");
        // Use a headless browser to inspect the DOM
        const browser = await puppeteer.launch({
            args: ['--no-sandbox', '--disable-setuid-sandbox']
        });
        const page = await browser.newPage();
        
        page.on('console', msg => console.log('BROWSER LOG:', msg.text()));
        page.on('pageerror', err => console.log('BROWSER ERROR:', err.message));
        
        await page.goto('http://localhost:5173');
        await new Promise(r => setTimeout(r, 2000));
        
        // Extract z-index and opacity of overlays
        const overlays = await page.evaluate(() => {
            const canvas = document.getElementById('os-canvas');
            const result = { canvasZ: canvas.style.zIndex, overlays: [] };
            
            document.querySelectorAll('.os-window-content').forEach(el => {
                result.overlays.push({
                    id: el.id,
                    zIndex: el.style.zIndex,
                    opacity: el.style.opacity,
                    color: el.style.color,
                    text: el.textContent
                });
            });
            return result;
        });
        
        console.log("DOM State:", JSON.stringify(overlays, null, 2));
        
        await browser.close();
    } catch(e) {
        console.log("Script Error:", e);
    }
})();
