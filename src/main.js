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
let imageSelectCount = 0;
let largeCheck = false;
let smallCheck = false;

async function process() {
  console.log("processing...");
  // const res = await invoke("processing", {
  //   filePaths: ["c:/Users/alant/Desktop/DR-Light-beam-test/images/DICOMOBJ/9x7-cir-L", "c:/Users/alant/Desktop/DR-Light-beam-test/images/DICOMOBJ/9x7-cir"],
  //   savePath: "c:/Users/alant/Desktop/t0re.jpg",
  // });
}

async function savePreviewImage(filePath, savePath) {
  const res = await invoke("preview", {
    filePath: filePath,
    savePath: savePath,
  });
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
      const tempDir = await tempdir();
      filePathsImage.push(filePath);
      let savePath = `${tempDir}${size}${imageSelectCount}LB.jpg`;
      if (size == "large") {
        largeImage.src = "assets/t4.jpg";
        largeText.innerText = "loading";
        console.log(savePath);
        await savePreviewImage(filePath, savePath);
        largeImage.src = convertFileSrc(savePath);
        largeText.innerText = "selected";
        largeCheck = true;
      } else {
        smallImage.src = "assets/t4.jpg";
        smallText.innerText = "loading";
        await savePreviewImage(filePath, savePath);
        smallImage.src = convertFileSrc(savePath);
        smallText.innerText = "selected";
        smallCheck = true;
      }
      imageSelectCount += 1;
      console.log(imageSelectCount);
    } else {
      if (size == "large") {
        largeText.innerText = "wrong";
        largeImage.src = "assets/t4.jpg";
        largeCheck = false;
      } else {
        smallText.innerText = "wrong";
        smallImage.src = "assets/t4.jpg";
        smallCheck = false;
      }
    }
  }
  // update process button
  console.log(processBtn.style.cursor);
  if (largeCheck && smallCheck) {
    console.log(largeCheck, smallCheck);
    processBtn.style.background = "black";
    processBtn.style.cursor = "pointer";
    processBtn.addEventListener("click", process);
  } else {
    processBtn.style.background = "beige";
    processBtn.style.cursor = "default";
    processBtn.removeEventListener("click", process);
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

window.addEventListener("DOMContentLoaded", async () => {
  await process();
});
