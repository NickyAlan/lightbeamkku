const { invoke } = window.__TAURI__.tauri;
const { tempdir } = window.__TAURI__.os;
const { convertFileSrc } = window.__TAURI__.tauri;
const { open, message } = window.__TAURI__.dialog;

const largeField = document.getElementById("largeField");
const largeImage = document.getElementById("largeImage");
const largeText = document.getElementById("largeText");
const smallField = document.getElementById("smallField");
const smallImage = document.getElementById("smallImage");
const smallText = document.getElementById("smallText");
const processBtn = document.getElementById("processBtn");
let filePathsImage = [];

async function process() {
  console.log("processing...");
  // const res = await invoke("processing", {
  //   filePaths: ["c:/Users/alant/Desktop/DR-Light-beam-test/images/DICOMOBJ/9x7-cir-L", "c:/Users/alant/Desktop/DR-Light-beam-test/images/DICOMOBJ/9x7-cir"],
  //   savePath: "c:/Users/alant/Desktop/t0re.jpg",
  // });
}

async function savePreviewImage(filePath, savePath) {
  console.log(filePath);
  const res = await invoke("preview", {
    filePath: filePath,
    savePath: savePath,
  });
  console.log("DONE");
}

function openFilefn() {
  return new Promise((resolve, reject) => {
    open({
      multiple: false,
      title: "Open DICOM files",
      filters: [
        {
          name: "DICOM",
          extensions: ["*", "dcm", "dicom"],
        },
      ],
    })
      .then((filePaths) => {
        if (filePaths) {
          resolve(filePaths);
        } else {
          reject("No file selected");
        }
      })
      .catch(reject);
  });
}

async function readFile(size) {
  const filePath = await openFilefn();
  if (filePath) {
    const lowerCasePath = filePath.toLowerCase();
    const split_ = lowerCasePath.split("\\");
    const file_type = split_[split_.length - 1].split(".")[1];
    if (!file_type || file_type == "dcm" || file_type == "dicom") {
      filePathsImage.push(filePath);
      console.log(file_type, filePathsImage);
    }
    if (size == "large") {
      const tempDir = await tempdir();
      let savePath = `${tempDir}${size}LB.jpg`;
      console.log(savePath);
      await savePreviewImage(filePath, savePath);
      largeImage.src = convertFileSrc(savePath);
      largeText.innerText = "selected";
    } else {
      const tempDir = await tempdir();
      let savePath = `${tempDir}${size}LB.jpg`;
      await savePreviewImage(filePath, savePath);
      smallImage.src = convertFileSrc(savePath);
      smallText.innerText = "selected";
    }
  }
}

largeField.addEventListener("click", (event) => {
  event.preventDefault();
  readFile("large");
});

smallField.addEventListener("click", (event) => {
  event.preventDefault();
  readFile("small");
});

processBtn.addEventListener("click", (event) => {
  console.log("SEND..");
});

window.addEventListener("DOMContentLoaded", async () => {
  await process();
});
