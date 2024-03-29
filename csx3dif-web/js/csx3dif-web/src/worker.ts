import init, { convert_csx, initThreadPool, init_panic_hook } from "csx3dif-web";

await init();
await initThreadPool(navigator.hardwareConcurrency);
init_panic_hook();

// Receive messages
addEventListener("message", async (event) => {
    let [f, filename, engine, version, mb, bsp, ptep, plep] = event.data;
    let csxfile = await f.text();
    let convert_results = convert_csx(csxfile, engine, version, mb, bsp, ptep, plep, (current: number, total: number, status: string, finishStatus: string) => {
        // console.log(`${current} / ${total} - ${status} - ${finishStatus}`);
        postMessage([0, filename, current, total, status, finishStatus]);
    });
    postMessage([1, filename, convert_results]);
});