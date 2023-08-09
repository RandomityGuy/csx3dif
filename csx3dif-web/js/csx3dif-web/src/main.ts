import { convert_csx} from "csx3dif-web";

const worker = new Worker(new URL('./worker.ts', import.meta.url), {
  type: 'module',
});

const submitBtn = document.querySelector("#submitBtn") as HTMLInputElement;
const uploadInput = document.querySelector("#csxupload") as HTMLInputElement;
const engineSelect = document.querySelector("#engineSelect") as HTMLInputElement;
const versionSelect = document.querySelector("#versionSelect") as HTMLInputElement;
const mbonly = document.querySelector("#mbonly") as HTMLInputElement;
const bspSelect = document.querySelector("#bspSelect") as HTMLInputElement;
const ptep = document.querySelector("#ptep") as HTMLInputElement;
const plep = document.querySelector("#plep") as HTMLInputElement;
const progressList = document.querySelector('#progresslist') as HTMLDivElement;
const bspReport = document.querySelector("#bsp-report") as HTMLTextAreaElement;
let progressBars: Map<string, { bar: HTMLDivElement, progress: number }> = new Map();

let filename = "";

submitBtn.addEventListener('click', async (e) => {
  e.preventDefault();
  let file = uploadInput.files?.item(0);
  filename = file.name;
  progressList.innerHTML = "";
  bspReport.value = "";
  progressBars.clear();
  worker.postMessage([file, engineSelect.value, versionSelect.value, mbonly.checked, bspSelect.value, ptep.value, plep.value]);
})


worker.addEventListener('message', (e) => {
  if (e.data[0] == 0) {
    let [cmd, current, total, status, finishStatus] = e.data;
    if (total == 0) return;
    if (progressBars.has(status)) {
      let progress = progressBars.get(status);
      progress.progress = current / total;
    } else {
      let clone = (document.querySelector("#progressbartemplate") as HTMLTemplateElement).content.cloneNode(true) as Node;
      progressList.append(clone);
      progressBars.set(status, { bar: progressList.lastElementChild.querySelector(".progress-bar") as HTMLDivElement, progress: current / total });
    }
    for (const [stat, val] of progressBars) {
      val.bar.style.width = `${(val.progress * 100)}%`;
      val.bar.textContent = stat;
    }
  } else if (e.data[0] == 1) {
    let [cmd, result] = e.data;
    let difs = result.data;
    let i = 0;
    let basename = filename.replace(".csx", "");
    for (const arr of difs) {
      let a = document.createElement("a");
      let blob = new Blob([arr]);
      let u = URL.createObjectURL(blob);
      a.href = u;
      if (i !== 0)
        a.download = `${basename}-${i}.dif`;
      else
        a.download = `${basename}.dif`;
      a.click();
      URL.revokeObjectURL(u);
      i++;
    }
    i = 0;
    for (const r of result.bsp_reports) {
      let reportData = "";
      reportData += `BSP Report ${i + 1}:\n`;
      reportData += `Raycast Coverage: ${r.hit}/${r.total} (${r.surface_area_percentage}% of surface area)\n`;
      reportData += `Balance Factor: ${r.balance_factor}\n`;
      bspReport.value += reportData;
      i++;
    }
  }
});