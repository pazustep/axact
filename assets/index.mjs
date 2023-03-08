import { h, Component, render } from "https://unpkg.com/preact?module";
import htm from "https://unpkg.com/htm?module";

const html = htm.bind(h);

function App({ cpus }) {
  return html`
    <div>
      ${cpus.map(cpu => html`
      <${Cpu} usage=${cpu} />`)}
    </div>`;
}

function Cpu({ usage }) {
  return html`
        <div class="bar">
        <label>${usage.toFixed(2)}%</label>
        <div class="bar-inner" style="width: ${usage}%"></div>
        </div>
    `;
}

const source = new EventSource("/api/cpus");

source.onmessage = ({ data }) => {
  let cpus = JSON.parse(data);
  render(html`<${App} cpus=${cpus} />`, document.body);
};