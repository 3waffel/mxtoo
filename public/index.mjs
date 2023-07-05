import { h, render } from "https://unpkg.com/preact?module";
import htm from "https://unpkg.com/htm?module";

const html = htm.bind(h);

function App({ data }) {
  let cpu_data = data.cpu_data;
  let mem_data = data.mem_data;

  return html`
    <div class="justify-center h-full text-center flex all:transition-400">
      <div class="text-white m10">
        <h2 text="3xl">CPU Usage</h2>
        ${cpu_data.map((cpu) => {
          return html`<div class="flex fw100 op60 hover:op100 m1">
            <div class="mt3">${cpu[0] + 1}</div>
            <div class="bar">
              <div class="bar-inner" w="${cpu[1]}%"></div>
              <label>${cpu[1].toFixed(2)}%</label>
            </div>
          </div>`;
        })}
      </div>

      <div class="text-white m10">
        <h2 text="3xl">Memory Usage</h2>
        <pre class="m1">
        <div><span>Memory Total: </span>${toGB(mem_data.total)}GB</div>
        <div><span>Memory Used: </span>${toGB(mem_data.used)}GB</div>
      </pre>
      </div>
    </div>
  `;
}

function toGB(bytes) {
  const gbs = (1.0 * bytes) / (1024 * 1024 * 1024);

  return gbs.toFixed(1);
}

let url = new URL("/realtime/data", window.location.href);
url.protocol = url.protocol.replace("http", "ws");

let ws = new WebSocket(url.href);
ws.onmessage = (ev) => {
  let json = JSON.parse(ev.data);
  render(html`<${App} data=${json}></${App}>`, document.body);
};
