const { invoke } = window.__TAURI__.tauri;
const { tempdir } = window.__TAURI__.os;
const { convertFileSrc } = window.__TAURI__.tauri;
const { open, message } = window.__TAURI__.dialog;
const { appDataDir } = window.__TAURI__.path;
const { createDir, exists } = window.__TAURI__.fs;

// load image
const inputDiv = document.querySelector(".file-input-container");
const largeField = document.getElementById("largeField");
const largeImage = document.getElementById("largeImage");
const largeText = document.getElementById("largeText");
const smallField = document.getElementById("smallField");
const smallImage = document.getElementById("smallImage");
const smallText = document.getElementById("smallText");
const processBtn = document.getElementById("processBtn");

// loading process
const loadingDiv = document.querySelector(".loading");

// show result
const resultDiv = document.querySelector(".result");
const backBtn = document.getElementById("backBtn");

// Database
const openDb = document.getElementById("openDb");
const saveDb = document.getElementById("saveDb");

let filePathsImage = [];
let imageSelectCount = 0;
let largeCheck = false;
let smallCheck = false;

async function process() {
  console.log("processing...");
  // inputDiv.style.display = "none";
  // loadingDiv.style.display = "flex";

  // setTimeout(() => {
  //   loadingDiv.style.display = "none";
  //   resultDiv.style.display = "grid";
  // }, 3000);

  const res = await invoke("processing", {
    filePaths: ["c:/Users/alant/Desktop/Project#4/DR-Light-beam-test/lb/3/00000000", "c:/Users/alant/Desktop/Project#4/DR-Light-beam-test/lb/3/00000001"],
    savePath: "c:/Users/alant/Desktop/test-save-file.jpg",
  });
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
        largeImage.src = "assets/a4.jpg";
        largeText.innerText = "loading";
        console.log(savePath);
        await savePreviewImage(filePath, savePath);
        largeImage.src = convertFileSrc(savePath);
        largeText.innerText = "selected";
        largeCheck = true;
      } else {
        smallImage.src = "assets/a4.jpg";
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
    processBtn.style.background = "blue";
    processBtn.style.color = "white";
    processBtn.style.cursor = "pointer";
    processBtn.addEventListener("click", process);
  } else {
    processBtn.style.background = "bisque";
    processBtn.style.color = "white";
    processBtn.style.cursor = "default";
    processBtn.removeEventListener("click", process);
  }
}

async function openFolder() {
  const dataDir = await appDataDir();
  const dirExists = await exists(dataDir);
  if (!dirExists) {
    await createDir(dataDir, { recursive: true });
  }
  return new Promise((resolve, reject) => {
    open({
      multiple: false,
      title: "Open Database",
      directory: true,
      defaultPath: dataDir,
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

async function loadDb() {
  const folderPath = await openFolder();
  console.log(folderPath);
}

async function saveToFolder() {
  // create top folder
  const dataDir = await appDataDir();
  const dirExists = await exists(dataDir);
  if (!dirExists) {
    await createDir(dataDir, { recursive: true });
  }
  // save file
  let folderName = "3";
  let folderPath = `${dataDir}\\${folderName}`;
  const folderExists = await exists(folderPath);
  if (!folderExists) {
    await createDir(folderPath, { recursive: true });
  }

  console.log("App Data Directory:", dataDir);
  // await message('Save Complete', 'Sucessfully Saved');
}

largeField.addEventListener("click", (event) => {
  event.preventDefault();
  readFile("large");
});

smallField.addEventListener("click", (event) => {
  event.preventDefault();
  readFile("small");
});

backBtn.addEventListener("click", (event) => {
  event.preventDefault();
  resultDiv.style.display = "none";
  inputDiv.style.display = "grid";
});

// Database Pop-Up
// openDb.addEventListener("click", (event) => {
//   event.preventDefault();
//   loadDb();
// });

// saveDb.addEventListener("click", (event) => {
//   event.preventDefault();
//   saveToFolder();
// });

const popup = document.getElementById("popup");
const overlay = document.getElementById("overlay");
const closeBtn = document.getElementById("closeBtn");

// Open popup
openDb.addEventListener("click", () => {
  let spanList = [40, 402, 35035, 305, 10313, 100, 50, 100, 350, 305, 130];
  const popupContent = document.querySelector(".popup-content"); // Select the popup content div

  // Clear previous content
  popupContent.innerHTML = "";
  spanList.forEach((item) => {
    const span = document.createElement("span"); // Create a new span element
    const link = document.createElement("a"); // Create a new anchor element
    link.href = "#"; // Set the href attribute (can be modified as needed)
    link.textContent = item; // Set the link text
    span.appendChild(link); // Append the link to the span
    popupContent.appendChild(span); // Append the span to the popup content

    // Add click event listener to the link
    span.addEventListener("click", (event) => {
      event.preventDefault(); // Prevent default anchor behavior
      console.log(item); // Log the current item to the console
    });
  });

  popup.style.display = "block";
  overlay.style.display = "block";
});

// Close popup
closeBtn.addEventListener("click", () => {
  popup.style.display = "none";
  overlay.style.display = "none";
});

// Close when clicking outside the popup
overlay.addEventListener("click", () => {
  popup.style.display = "none";
  overlay.style.display = "none";
});

// DEBUG
window.addEventListener("DOMContentLoaded", async () => {
  await process();
});

// Save as Image
saveDb.addEventListener("click", function() {
  const element = document.getElementById("resultDisplay"); // The element to capture

  html2canvas(element).then(function(canvas) {
    // Convert the canvas to an image (base64 format)
    const imgData = canvas.toDataURL("image/png");

    // Create a link to download the image
    const downloadLink = document.createElement("a");
    downloadLink.href = imgData;
    downloadLink.download = "screenshot.png"; // File name for download

    // Trigger the download
    downloadLink.click();
  });
});
