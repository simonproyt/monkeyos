import puppeteer from 'puppeteer';

(async () => {
    const browser = await puppeteer.launch({ headless: "new", args: ['--no-sandbox'] });
    const page = await browser.newPage();

    page.on('console', msg => {
        console.log(`[Browser Console] ${msg.type()}: ${msg.text()}`);
    });

    await page.goto('http://localhost:5173');
    await page.waitForTimeout(5000);
    await browser.close();
    process.exit(0);
})();
