const jsdom = require("jsdom");
const { JSDOM } = jsdom;
const dom = new JSDOM(`<!DOCTYPE html><p>Hello world</p>`);
const document = dom.window.document;

const div = document.createElement('div');
div.textContent = "MonkeyOS Terminal v0.1\nType 'help' for commands.\n\nroot@monkeyos:/# ";
console.log(JSON.stringify(div.innerHTML));
let innerHtml = div.innerHTML;
let lastNewlineIndex = innerHtml.lastIndexOf('\n');
console.log("lastNewlineIndex:", lastNewlineIndex);
