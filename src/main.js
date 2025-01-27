const { invoke } = window.__TAURI__.tauri;
const { tempdir } = window.__TAURI__.os;
const { convertFileSrc } = window.__TAURI__.tauri;
const { open, message, save } = window.__TAURI__.dialog;
const { appDataDir } = window.__TAURI__.path;
const { createDir, exists, writeBinaryFile } = window.__TAURI__.fs;

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
const resultImage = document.getElementById("resultImage");
const resultImageCir = document.getElementById("resultImageCir");

// Database
const openDb = document.getElementById("openDb");
const saveDb = document.getElementById("saveDb");

let filePathsImage = ["", ""];
let imageSelectCount = 0;
let largeCheck = false;
let smallCheck = false;

async function process() {
  console.log(filePathsImage);
  console.log("processing...");
  // loading screen
  inputDiv.style.display = "none";
  loadingDiv.style.display = "flex";

  // // save path
  // const currentDateTime = new Date();
  // const year = currentDateTime.getFullYear();
  // const month = String(currentDateTime.getMonth() + 1).padStart(2, "0"); // Months are 0-indexed
  // const day = String(currentDateTime.getDate()).padStart(2, "0");
  // const hours = String(currentDateTime.getHours()).padStart(2, "0");
  // const minutes = String(currentDateTime.getMinutes()).padStart(2, "0");
  // const seconds = String(currentDateTime.getSeconds()).padStart(2, "0");
  // const formattedDateTime = `${year}${month}${day}${hours}${minutes}${seconds}`;

  // const tempDir = await tempdir();
  // const savePath = [
  //   `${tempDir}${formattedDateTime}.jpg`,
  //   `${tempDir}${formattedDateTime}+cir.jpg`,
  // ];

  // const res = await invoke("processing", {
  //   filePaths: filePathsImage,
  //   savePath: savePath,
  // });

  // // DEBUG
  // // const res = await invoke("processing", {
  // //   filePaths: ["c:/Users/alant/Desktop/Project#4/DR-Light-beam-test/lb/smc 2/00000000", "c:/Users/alant/Desktop/Project#4/DR-Light-beam-test/lb/smc 2/00000001"],
  // //   savePath: "c:/Users/alant/Desktop/test-save-file.jpg",
  // // });

  // // result screen
  // resultImage.src = convertFileSrc(savePath[0]);
  // resultImageCir.src = convertFileSrc(savePath[1]);
  // loadingDiv.style.display = "none";
  // resultDiv.style.display = "grid";

  // DEBUG
  // setTimeout(() => {
  //   loadingDiv.style.display = "none";
  //   resultDiv.style.display = "grid";
  // }, 2000);
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
      let savePath = `${tempDir}${size}${imageSelectCount}LB.jpg`;

      if (size == "large") {
        filePathsImage[0] = filePath;
        largeImage.src = "assets/a4.jpg";
        largeText.innerText = "loading";
        console.log(savePath);
        await savePreviewImage(filePath, savePath);
        largeImage.src = convertFileSrc(savePath);
        largeText.innerText = "selected";
        largeCheck = true;
      } else {
        filePathsImage[1] = filePath;
        smallImage.src = "assets/a4.jpg";
        smallText.innerText = "loading";
        await savePreviewImage(filePath, savePath);
        smallImage.src = convertFileSrc(savePath);
        smallText.innerText = "selected";
        smallCheck = true;
      }
      imageSelectCount += 1;
      console.log(imageSelectCount);
      console.log(filePathsImage);
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

// // DEBUG
// window.addEventListener("DOMContentLoaded", async () => {
//   await process();
// });

saveDb.addEventListener("click", async function () {
  try {
    // Open Tauri save dialog to select the file path
    const savePath = await save({
      title: "Save Your Image",
      defaultPath: "result.png",
      filters: [
        { name: "PNG Image", extensions: ["png"] },
      ],
    });

    if (!savePath) {
      console.log("Save operation was canceled.");
      return;
    }

    // Capture the element and convert it to a canvas
    const canvas = await html2canvas(document.getElementById("resultDisplay"), {
      allowTaint: true,
      useCORS: true,
    });

    // Convert the canvas to base64 PNG
    const imgData = canvas.toDataURL("image/png", 0.5);

    // Convert Base64 to binary data (using TextDecoder and Uint8Array for compatibility)
    const base64Data = imgData.split(",")[1];
    const binaryData = new Uint8Array(
      window.atob(base64Data)
        .split("")
        .map((char) => char.charCodeAt(0))
    );

    // Save the binary data to the selected path
    await writeBinaryFile(savePath, binaryData);

    console.log("Image saved successfully to:", savePath);
  } catch (e) {
    console.error("Error saving the image:", e);
  }
});
