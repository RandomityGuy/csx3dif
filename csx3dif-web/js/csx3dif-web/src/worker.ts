import init, { convert_csx, initThreadPool, init_panic_hook } from "csx3dif-web";

init().then(() => initThreadPool(navigator.hardwareConcurrency).then(() => {
    init_panic_hook();
    main();
}));

const main = () => {
    // Receive messages
    addEventListener("message", async (event) => {
        let [f, engine, version, mb, bsp, ptep, plep] = event.data;
        let csxfile = await f.text();
        let convert_results = convert_csx(csxfile, engine, version, mb, bsp, ptep, plep, (current: number, total: number, status: string, finishStatus: string) => {
            // console.log(`${current} / ${total} - ${status} - ${finishStatus}`);
            postMessage([0, current, total, status, finishStatus]);
        });
        postMessage([1, convert_results]);
    });
}