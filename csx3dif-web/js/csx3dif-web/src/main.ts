import { convert_csx } from "csx3dif-web";

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
const exportContent = document.querySelector("#exportcontent") as HTMLDivElement;
const dndOverlay = document.querySelector("#dndoverlay") as HTMLDivElement;
let csxFiles: Map<string, CSXEntry> = new Map();

interface CSXEntry {
  filename: string,
  progressBars: Map<string, { bar: HTMLProgressElement, progress: number, status: HTMLParagraphElement }>,
  progressList: HTMLDivElement,
  cardElement: HTMLDivElement
}

const createCSXCard = (filename: string) => {
  let clone = (document.querySelector("#exporttemplate") as HTMLTemplateElement).content.cloneNode(true) as Node;
  exportContent.append(clone);
  (exportContent.lastElementChild.querySelector(".card-title") as HTMLHeadingElement).textContent = filename;
  let entry: CSXEntry = {
    filename: filename,
    progressBars: new Map(),
    progressList: exportContent.lastElementChild.querySelector('#progresslist') as HTMLDivElement,
    cardElement: exportContent.lastElementChild as HTMLDivElement
  }
  return entry;
}


window.addEventListener('dragenter', (e) => {
  dndOverlay.style.display = "flex";
});

window.addEventListener('drop', (e) => {
  e.preventDefault();
  dndOverlay.style.display = "none";
  for (let file of e.dataTransfer.files) {
    if (!file.name.endsWith(".csx")) continue;
    let filename = file.name;
    exportContent.hidden = false;

    let entry = createCSXCard(filename);
    csxFiles.set(filename, entry);
    worker.postMessage([file, filename, engineSelect.value, versionSelect.value, mbonly.checked, bspSelect.value, ptep.value, plep.value]);
  }
});

window.addEventListener('dragover', (e) => {
  e.preventDefault();
});

dndOverlay.addEventListener('dragleave', (e) => {
  dndOverlay.style.display = "none";
})

uploadInput.addEventListener('change', (e) => {
  if (uploadInput.files.length !== 0) {
    submitBtn.disabled = false;
  } else {
    submitBtn.disabled = true;
  }
});


submitBtn.addEventListener('click', async (e) => {
  e.preventDefault();
  for (let file of uploadInput.files) {
    if (!file.name.endsWith(".csx")) continue;
    let filename = file.name;
    exportContent.hidden = false;

    let entry = createCSXCard(filename);
    csxFiles.set(filename, entry);
    worker.postMessage([file, filename, engineSelect.value, versionSelect.value, mbonly.checked, bspSelect.value, ptep.value, plep.value]);
  }
})


worker.addEventListener('message', (e) => {
  if (e.data[0] == 0) {
    let [cmd, filename, current, total, status, finishStatus] = e.data;
    let entry = csxFiles.get(filename);

    // console.log(`cmd: ${cmd}, current: ${current}, total: ${total}, status: ${status}, finishStatus: ${finishStatus}`);
    if (total == 0) return;
    if (entry.progressBars.has(status)) {
      let progress = entry.progressBars.get(status);
      progress.progress = current / total;
    } else {
      let clone = (document.querySelector("#progressbartemplate") as HTMLTemplateElement).content.cloneNode(true) as Node;
      entry.progressList.append(clone);
      entry.progressBars.set(status, {
        bar: entry.progressList.lastElementChild.querySelector("progress") as HTMLProgressElement,
        status: entry.progressList.lastElementChild.querySelector("#progresslabel") as HTMLParagraphElement,
        progress: current / total
      });
    }
    for (const [stat, val] of entry.progressBars) {
      val.bar.value = val.progress;
      val.status.textContent = stat;
      // val.bar.style.width = `${(val.progress * 100)}%`;
      // val.bar.textContent = stat;
    }
  } else if (e.data[0] == 1) {
    let [cmd, filename, result] = e.data;

    // remove the progress bars
    let entry = csxFiles.get(filename);
    entry.progressList.innerHTML = "";
    entry.progressList.hidden = true;
    // Add the completed
    (entry.cardElement.querySelector(".card-title") as HTMLHeadingElement).innerHTML = `${filename} <p class="text-right">Completed</p>`;

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
    const bspReportsElement = entry.cardElement.querySelector(".collapse-content") as HTMLDivElement;
    i = 0;
    for (const r of result.bsp_reports) {
      let clone = (document.querySelector("#reporttemplate") as HTMLTemplateElement).content.cloneNode(true) as Node;
      bspReportsElement.append(clone);
      let elem = bspReportsElement.lastElementChild as HTMLDivElement;
      elem.querySelector("#reportname").textContent = `Report ${i + 1}`;
      elem.querySelector("#balancefactor").textContent = `Balance Factor: ${r.balance_factor}`;
      elem.querySelector(".radial-progress").textContent = `${Math.trunc(r.surface_area_percentage)}%`;
      (elem.querySelector(".radial-progress") as HTMLDivElement).style.setProperty("--value", `${r.surface_area_percentage}`);
      elem.querySelector("#coveragetext").textContent = `${r.hit}/${r.total}`;
      // bspReport.value += reportData;
      i++;
    }
  }
});