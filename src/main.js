const { invoke } = window.__TAURI__.tauri;
const { tempdir } = window.__TAURI__.os;
const { convertFileSrc } = window.__TAURI__.tauri;

async function process() {
  console.log("hello");
  const res = await invoke("processing", {
    filePaths: ["c:/Users/alant/Desktop/DR-Light-beam-test/images/DICOMOBJ/9x7-cir-L", "c:/Users/alant/Desktop/DR-Light-beam-test/images/DICOMOBJ/9x7-cir"],
    savePath: "c:/Users/alant/Desktop/t0re.jpg",
  });
}

window.addEventListener("DOMContentLoaded", async () => {
  await process();
});
