import { h, render } from "https://esm.sh/preact";
import { signal } from "https://esm.sh/@preact/signals";
import htm from "https://esm.sh/htm";

const html = htm.bind(h);

function CpuList({ cpus }) {
  return html`
    <div>
      ${cpus.value.map(cpu => html`
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

const cpus = signal([]);
const source = new EventSource("/api/cpus");

source.onmessage = ({ data }) => {
  cpus.value = JSON.parse(data);
};

render(html`<${CpuList} cpus=${cpus}/>`, document.body);