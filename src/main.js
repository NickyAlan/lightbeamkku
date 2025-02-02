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
const tableDiv = document.getElementById("tableDiv");
let sid = 100;
let criteria = 1;

// Database
const openDb = document.getElementById("openDb");
const saveDb = document.getElementById("saveDb");
const helpBtn = document.getElementById("helpBtn");

let filePathsImage = ["", ""];
let imageSelectCount = 0;
let fileCheckInfoL = [0, 0, 0, 0];
let fileCheckInfoF = [0, 0, 0, 0];
let largeCheck = false;
let smallCheck = false;

async function process() {
  console.log(filePathsImage);
  console.log("processing...");
  // loading screen
  inputDiv.style.display = "none";
  loadingDiv.style.display = "flex";

  // save path
  const currentDateTime = new Date();
  const year = currentDateTime.getFullYear();
  const month = String(currentDateTime.getMonth() + 1).padStart(2, "0"); // Months are 0-indexed
  const day = String(currentDateTime.getDate()).padStart(2, "0");
  const hours = String(currentDateTime.getHours()).padStart(2, "0");
  const minutes = String(currentDateTime.getMinutes()).padStart(2, "0");
  const seconds = String(currentDateTime.getSeconds()).padStart(2, "0");
  const formattedDateTime = `${year}${month}${day}${hours}${minutes}${seconds}`;

  const tempDir = await tempdir();
  const savePath = [
    `${tempDir}${formattedDateTime}.jpg`,
    `${tempDir}${formattedDateTime}+cir.jpg`,
  ];

  const res = await invoke("processing", {
    filePaths: filePathsImage,
    savePath: savePath,
  });

  // get results
  const [x, y, h, k] = res[0];
  let [cir_distance, cir_angle] = res[1];
  let cir_status = "passed";
  let cir_color = "blue";
  if (cir_angle > 3.0) {
    cir_status = "failed";
    cir_color = "red";
  }
  const points = res[2];
  const length = res[3];
  const errCm = [
    length[0][0][1].toFixed(3),
    length[1][0][1].toFixed(3),
    length[2][0][1].toFixed(3),
    length[3][0][1].toFixed(3),
  ];
  const [errPercentage, colStatus] = errPercent(errCm, sid, criteria);
  // const pos = res[4];
  const max_err_pos = res[5];
  const xpoints = res[6];
  const ypoints = res[7];
  const info = res[8];

  // details
  let pixel_size_sup = "-";
  if (info[6] != " - ") {
    pixel_size_sup = `${info[6]}<sup>2</sup>`;
  }

  // add result
  console.log("start");
  await updateTable(
    length,
    errCm,
    max_err_pos,
    savePath,
    errPercentage,
    cir_distance,
    cir_angle,
    cir_status,
    info,
    pixel_size_sup,
    colStatus,
    cir_color
  );

  // SID input
  const inputField = document.getElementById("sidInputCm");
  inputField.addEventListener("input", function (event) {
    let input = event.target.value;
    input = input.replace(/[^0-9]/g, "");
    const num = parseInt(input, 10);
    if (num < 1 || num > 300) {
      input = input.slice(0, -1); // Remove the last character if the number is out of range
    }
    event.target.value = input;
    sid = input;
    updateErr(errCm, sid, criteria);
    updateCir(cir_distance, parseFloat(sid));
    updateRowBackground();
    updateCircleRowBackground();
  });
  // criteria
  document.getElementById("radioForm").addEventListener("change", function () {
    const selectedOption = document.querySelector(
      'input[name="option"]:checked'
    ).value;
    criteria = parseFloat(selectedOption);
    console.log("cri:", criteria);
    updateErr(errCm, sid, criteria);
    console.log("Selected option:", selectedOption);
    updateRowBackground();
  });

  // // DEBUG
  // // const res = await invoke("processing", {
  // //   filePaths: ["c:/Users/alant/Desktop/Project#4/DR-Light-beam-test/lb/smc 2/00000000", "c:/Users/alant/Desktop/Project#4/DR-Light-beam-test/lb/smc 2/00000001"],
  // //   savePath: "c:/Users/alant/Desktop/test-save-file.jpg",
  // // });

  // result screen

  setTimeout(() => {
    circlePlot(x, y, h, k);
    edgePlot(points, xpoints, ypoints);
  }, 50);

  loadingDiv.style.display = "none";
  resultDiv.style.display = "grid";

  // DEBUG
  // setTimeout(() => {
  //   loadingDiv.style.display = "none";
  //   resultDiv.style.display = "grid";
  // }, 2000);
}

// DEBUG
// inputDiv.style.display = "none";
// resultDiv.style.display = "grid";

async function savePreviewImage(filePath, savePath, isLarge) {
  const res = await invoke("preview", {
    filePath: filePath,
    savePath: savePath,
  });
  if (isLarge) {
    for (let i = 0; i < 4; i++) {
      fileCheckInfoL[i] = res[i];
    }
  } else {
    for (let i = 0; i < 4; i++) {
      fileCheckInfoF[i] = res[i];
    }
  }
}

function openFilefn() {
  return new Promise((resolve, reject) => {
    open({
      multiple: false,
      title: "Open a DICOM file",
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

async function updateTable(
  length,
  errCm,
  max_err_pos,
  savePath,
  errPercentage,
  cir_distance,
  cir_angle,
  cir_status,
  info,
  pixel_size_sup,
  colStatus,
  cir_color
) {
  console.log(cir_distance, cir_angle, cir_status);
  tableDiv.innerHTML = `
            <h1>Result Report</h1>
          <span id="sidInput"
            ><p id="colRes">Collimator Alignment <p id="colStatus">(${colStatus})</p></p>
            <p>— SID:</p>
            <input
              type="number"
              id="sidInputCm"
              value="100"
            />
            <p>cm, </p>
            <form id="radioForm">
              <label class="radio-rectangle">
                <input type="radio" name="option" value="1" checked/>
                <span>1%</span>
              </label>
              <label class="radio-rectangle">
                <input type="radio" name="option" value="2" />
                <span>2%</span>
              </label>
            </form>
            <p>criteria</p></span
          >
          <table>
            <tr>
              <th>Position</th>
              <th>Length (cm)</th>
              <th>Error (cm)</th>
              <th>Error (%)</th>
              <th>Most Error</th>
              <th>Status</th>
            </tr>
            <tr>
              <td>X<sub>1</sub></td>
              <td>${length[0][0][0].toFixed(3)}</td>
              <td>${errCm[0]}</td>
              <td id="err1">${errPercentage[0][0]}</td>
              <td>${max_err_pos[0]}</td>
              <td id="sta1">${errPercentage[0][1]}</td>
            </tr>
            <tr>
              <td>X<sub>2</sub></td>
              <td>${length[1][0][0].toFixed(3)}</td>
              <td>${errCm[1]}</td>
              <td id="err2">${errPercentage[1][0]}</td>
              <td>${max_err_pos[1]}</td>
              <td id="sta2">${errPercentage[1][1]}</td>
            </tr>
            <tr>
              <td>Y<sub>1</sub></td>
              <td>${length[2][0][0].toFixed(3)}</td>
              <td>${errCm[2]}</td>
              <td id="err3">${errPercentage[2][0]}</td>
              <td>${max_err_pos[2]}</td>
              <td id="sta3">${errPercentage[2][1]}</td>
            </tr>
            <tr>
              <td>Y<sub>2</sub></td>
              <td>${length[3][0][0].toFixed(3)}</td>
              <td>${errCm[3]}</td>
              <td id="err4">${errPercentage[3][0]}</td>
              <td>${max_err_pos[3]}</td>
              <td id="sta4">${errPercentage[3][1]}</td>
            </tr>
          </table>

          <div class="lower-res">
            <div class="left-res">
              <span id="beamRes">Beam Aliagnment <p id="cirStatusC">(${cir_status})</p></span>
              <table>
                <tr>
                  <th>Length (cm)</th>
                  <th>Angle</th>
                  <th>Status</th>
                </tr>
                <tr>
                  <td>${cir_distance.toFixed(3)}</td>
                  <td id="cirAngle" >${cir_angle.toFixed(3)}°</td>
                  <td id="cirStatus" >${cir_status}</td>
                </tr>
              </table>
              <div class="circleImage">
                <img id="resultImageCir" src="${convertFileSrc(savePath[1])}" />
                <canvas id="canvasCir"></canvas>
              </div>
            </div>

            <div class="right-res">
              <div class="detectorDetails">
                <h2>Information</h2>
                <p>Hospital: ${info[0]}</p>
                <p>Manufacturer: ${info[1]}</p>
                <p>Institution Address: ${info[2]}</p>
                <p>Acquisition Date: ${info[3]}</p>
                <p>Detector Type: ${info[4]}</p>
                <p>Detector ID: ${info[5]}</p>
                <p>Pixel Size: ${pixel_size_sup}</p>
                <p>Matrix Size: ${info[7]}</p>
                <p>Bit Depth: ${info[8]}</p>
                <span class="note"
                  ><p>Note:</p>
                  <textarea rows="4"></textarea>
                </span>
              </div>
            </div>
          </div>
  `;

  document.getElementById("resultImage").src = convertFileSrc(savePath[0]);
  document.getElementById("cirStatusC").style.color = cir_color;

  updateColorCol(colStatus);
  updateRowBackground();
  updateCircleRowBackground();
}

async function readFile(size) {
  const filePath = await openFilefn();
  if (filePath) {
    const lowerCasePath = filePath.toLowerCase();
    const split_ = lowerCasePath.split("\\");
    const length = split_.length;
    const file_type = split_[length - 1].split(".")[1];

    if (!file_type || file_type == "dcm" || file_type == "dicom") {
      const tempDir = await tempdir();
      let savePath = `${tempDir}${size}${imageSelectCount}LB.jpg`;

      if (size == "large") {
        filePathsImage[0] = filePath;
        largeImage.src = "assets/largeload.png";
        largeText.innerText = "loading";
        console.log(savePath);
        await savePreviewImage(filePath, savePath, true);
        console.log(filePath.split("\\"));
        largeImage.src = convertFileSrc(savePath);
        largeText.innerText = `../${split_[length - 2]}/${split_[length - 1]}`;
        largeCheck = true;
      } else {
        filePathsImage[1] = filePath;
        smallImage.src = "assets/fitload.png";
        smallText.innerText = "loading";
        await savePreviewImage(filePath, savePath, false);
        smallImage.src = convertFileSrc(savePath);
        smallText.innerText = `../${split_[length - 2]}/${split_[length - 1]}`;
        smallCheck = true;
      }
      imageSelectCount += 1;
      console.log(imageSelectCount);
      console.log(filePathsImage);
    } else {
      if (size == "large") {
        largeText.innerText = `../${split_[length - 2]}/${split_[length - 1]}`;
        largeImage.src = "assets/wrong.png";
        largeCheck = false;
      } else {
        smallText.innerText = `../${split_[length - 2]}/${split_[length - 1]}`;
        smallImage.src = "assets/wrong.png";
        smallCheck = false;
      }
    }
  }
  // update process button
  console.log(processBtn.style.cursor);
  if (largeCheck && smallCheck) {
    // check is same file
    if (filePathsImage[0] == filePathsImage[1]) {
      alert("It's the same File!");
      removeBtnProcess();
    } else if (!isSameDetector(fileCheckInfoL, fileCheckInfoF)) {
      // check is same detector
      alert("Not same Position!");
      removeBtnProcess();
    } else {
      console.log(largeCheck, smallCheck);

      processBtn.style.background = "blue";
      processBtn.style.color = "white";
      processBtn.style.cursor = "pointer";
      processBtn.addEventListener("click", process);
    }
  } else {
    removeBtnProcess();
  }
}

function removeBtnProcess() {
  processBtn.style.background = "bisque";
  processBtn.style.color = "white";
  processBtn.style.cursor = "default";
  processBtn.removeEventListener("click", process);
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
  let folderName = "test-folder";
  let folderPath = `${dataDir}\\${folderName}`;
  const folderExists = await exists(folderPath);
  if (!folderExists) {
    await createDir(folderPath, { recursive: true });
  }

  console.log("App Data Directory:", folderPath);
  // appName.textContent = folderPath;
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

  // clear canvas
  clearCanvasById("canvasImage");
  sid = 100;
  criteria = 1;
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
// window.addEventListener("DOMContentLoaded", async () => {
//   // await process();
//   // saveToFolder();
//   const [x, y, h, k] = [55, 46, 60, 60];
//   const points = [
//     [227, 250],
//     [1381, 235],
//     [237, 1060],
//     [1387, 1045],
//   ];
//   // circlePlot(x, y, h, k);
//   // edgePlot(points);
// });

saveDb.addEventListener("click", async function () {
  try {
    // Open Tauri save dialog to select the file path
    const savePath = await save({
      title: "Save Your Image",
      defaultPath: "result.png",
      filters: [{ name: "PNG Image", extensions: ["png"] }],
    });

    if (!savePath) {
      console.log("Save operation was canceled.");
      return;
    }

    // Capture the element and convert it to a canvas
    const canvas = await html2canvas(document.getElementById("resultDisplay"), {
      allowTaint: true,
      useCORS: true,
      scale: 2, // Scale factor for higher quality
    });

    // Convert the canvas to base64 PNG
    const imgData = canvas.toDataURL("image/png", 1.0);

    // Convert Base64 to binary data (using TextDecoder and Uint8Array for compatibility)
    const base64Data = imgData.split(",")[1];
    const binaryData = new Uint8Array(
      window
        .atob(base64Data)
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

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

async function circlePlot(x, y, h, k) {
  console.log("run c");
  const image = document.getElementById("resultImageCir");
  console.log(image.src);
  const canvas = document.getElementById("canvasCir");
  const ctx = canvas.getContext("2d");

  const W = image.naturalWidth;
  const H = image.naturalHeight;
  const NW = image.offsetWidth;
  const NH = image.offsetHeight;
  console.log(W, H, NW, NH);
  // adjust ratio
  const RW = NW / W;
  const RH = NH / H;
  canvas.width = NW;
  canvas.height = NH;

  // 2 points
  const color = "blue";
  const dotX = Math.round(h * RW);
  const dotY = Math.round(k * RH);
  const dotRadius = 3;

  const devX = Math.round(x * RW);
  const devY = Math.round(y * RW);

  ctx.beginPath();
  ctx.arc(dotX, dotY, dotRadius, 0, Math.PI * 2);
  // await sleep(500); // Pause for 500ms between points
  ctx.arc(devX, devY, dotRadius, 0, Math.PI * 2);
  ctx.fillStyle = color;
  ctx.fill();

  // line
  ctx.lineWidth = 2; // Thickness of the line
  ctx.strokeStyle = color; // Color of the line

  // Draw the dashed line
  ctx.beginPath();
  // await sleep(500); // Pause for 500ms between points
  ctx.moveTo(dotX, dotY); // Starting point (x, y)
  ctx.lineTo(devX, devY); // Ending point (x, y)
  ctx.stroke();
}

async function edgePlot(points, xpoints, ypoints) {
  // points; [[top-left x, top-left y], ...]
  console.log("run i");
  let [
    [top_xl, top_yl],
    [top_xr, top_yr],
    [bottom_xl, bottom_yl],
    [bottom_xr, bottom_yr],
  ] = points;

  const image = document.getElementById("resultImage");
  const canvas = document.getElementById("canvasImage");
  const ctx = canvas.getContext("2d");

  const W = image.naturalWidth;
  const H = image.naturalHeight;
  const NW = image.offsetWidth;
  const NH = image.offsetHeight;
  console.log(W, H, NW, NH);

  // adjust ratio
  const RW = NW / W;
  const RH = NH / H;
  canvas.width = NW;
  canvas.height = NH;

  const color = "yellow";

  top_xl = Math.round(top_xl * RW);
  top_yl = Math.round(top_yl * RH);
  top_xr = Math.round(top_xr * RW);
  top_yr = Math.round(top_yr * RH);
  bottom_xl = Math.round(bottom_xl * RW);
  bottom_yl = Math.round(bottom_yl * RH);
  bottom_xr = Math.round(bottom_xr * RW);
  bottom_yr = Math.round(bottom_yr * RH);

  // const p = [
  //   [top_xl, top_yl],
  //   [bottom_xl, bottom_yl],
  //   [bottom_xr, bottom_yr],
  //   [top_xr, top_yr],
  // ];

  // ctx.beginPath(); // Start a new path
  // ctx.strokeStyle = color; // Set the line color to yellow
  // ctx.lineWidth = 2; // Optional: Set line thickness
  // for (let i = 0; i < p.length; i++) {
  //   const [x, y] = p[i];
  //   if (i === 0) {
  //     // Move to the first point without drawing
  //     ctx.moveTo(x, y);
  //   } else {
  //     // Draw a line to the next point
  //     ctx.lineTo(x, y);
  //   }
  //   ctx.stroke(); // Render the current line segment
  //   await sleep(500); // Pause for 500ms between points
  // }
  // // Close the rectangle
  // ctx.closePath();
  // ctx.stroke();
  const offset = 50;
  let x1Pos = [
    (xpoints[0] * 3) / 4 + (xpoints[1] * 1) / 4,
    ypoints[1] - offset,
  ];
  x1Pos = [Math.round(x1Pos[0] * RW), Math.round(x1Pos[1] * RH)];
  let x2Pos = [
    (xpoints[1] * 1) / 4 + (xpoints[2] * 3) / 4,
    ypoints[1] - offset,
  ];
  x2Pos = [Math.round(x2Pos[0] * RW), Math.round(x2Pos[1] * RH)];
  let y1Pos = [
    xpoints[1] + offset,
    (ypoints[0] * 3) / 4 + (ypoints[1] * 1) / 4,
  ];
  y1Pos = [Math.round(y1Pos[0] * RW), Math.round(y1Pos[1] * RH)];
  let y2Pos = [
    xpoints[1] + offset,
    (ypoints[1] * 1) / 4 + (ypoints[2] * 3) / 4,
  ];
  y2Pos = [Math.round(y2Pos[0] * RW), Math.round(y2Pos[1] * RH)];

  // text
  ctx.font = "16px Arial";
  ctx.fillStyle = color;
  ctx.textAlign = "center";
  // Draw text on the canvas
  ctx.fillText("X1", x1Pos[0], x1Pos[1]);
  ctx.fillText("X2", x2Pos[0], x2Pos[1]);
  ctx.fillText("Y1", y1Pos[0], y1Pos[1]);
  ctx.fillText("Y2", y2Pos[0], y2Pos[1]);

  ctx.beginPath();
  ctx.moveTo(top_xl, top_yl);
  ctx.lineTo(bottom_xl, bottom_yl);
  ctx.lineTo(bottom_xr, bottom_yr);
  ctx.lineTo(top_xr, top_yr);
  ctx.lineWidth = 2;
  ctx.strokeStyle = color;
  ctx.closePath();
  ctx.stroke();
}

function errPercent(errCm, sid, criteria) {
  let errP = [];
  let colStatus = "passed";
  for (let i = 0; i < errCm.length; i++) {
    let percent = (Math.abs(parseFloat(errCm[i])) / sid) * 100;
    let status = "passed";
    if (percent > criteria) {
      status = "failed";
      colStatus = "failed";
    }
    if (percent == "Infinity") {
      errP.push(["-", status]);
    } else {
      errP.push([percent.toFixed(3), status]);
    }
  }

  return [errP, colStatus];
}

function clearCanvasById(canvasId) {
  console.log("clear canvas");
  const canvas = document.getElementById(canvasId); // Get the canvas by its ID
  const ctx = canvas.getContext("2d"); // Get the 2D context of the canvas
  ctx.clearRect(0, 0, canvas.width, canvas.height); // Clear the entire canvas
}

function updateErr(errCm, sid, criteria) {
  console.log("update");
  const [errPercentage, colStatus] = errPercent(errCm, sid, criteria);
  console.log(errPercentage);
  const errP = [
    document.getElementById("err1"),
    document.getElementById("err2"),
    document.getElementById("err3"),
    document.getElementById("err4"),
  ];
  const statuss = [
    document.getElementById("sta1"),
    document.getElementById("sta2"),
    document.getElementById("sta3"),
    document.getElementById("sta4"),
  ];
  for (let i = 0; i < errPercentage.length; i++) {
    errP[i].textContent = errPercentage[i][0];
    statuss[i].textContent = errPercentage[i][1];
  }

  document.getElementById("colStatus").textContent = `(${colStatus})`;
  updateColorCol(colStatus);
}

function updateCir(cir_distance, sid) {
  const cirAngleElm = document.getElementById("cirAngle");
  if (!isNaN(sid)) {
    const cirStatusElm = document.getElementById("cirStatus");
    const cirStatusCElm = document.getElementById("cirStatusC");

    // calculate angle
    const radians = Math.atan(cir_distance / sid);
    const cirAngle = radians * (180 / Math.PI);
    console.log(sid, radians, cirAngle);

    // check is angle <3.0
    let status = "passed";
    let color = "blue";
    if (cirAngle > 3.0) {
      status = "failed";
      color = "red";
    }

    cirAngleElm.textContent = cirAngle.toFixed(3);
    cirStatusElm.textContent = status;
    cirStatusCElm.textContent = `(${status})`;
    cirStatusCElm.style.color = color;
  } else {
    cirAngleElm.textContent = "-";
  }
}

function updateColorCol(colStatus) {
  const colColor = document.getElementById("colStatus");
  let color = "blue";
  if (colStatus != "passed") {
    color = "red";
  }
  colColor.style.color = color;
}

// table passed or failed color chnage
function updateRowBackground() {
  for (let i = 1; i <= 4; i++) {
    // Assuming 4 rows with IDs `sta1`, `sta2`, etc.
    const statusCell = document.getElementById(`sta${i}`);
    const row = statusCell.parentElement; // Get the parent <tr> of the status cell

    if (statusCell.textContent.trim() === "failed") {
      row.style.backgroundColor = "rgba(255, 0, 0, 0.5)";
    } else {
      row.style.backgroundColor = ""; // Reset to default if not failed
    }
  }
}

function updateCircleRowBackground() {
  const statusCell = document.getElementById("cirStatus");
  const row = statusCell.parentElement;

  if (statusCell.textContent.trim() === "failed") {
    row.style.backgroundColor = "rgba(255, 0, 0, 0.5)"; // Light red background
  } else {
    row.style.backgroundColor = ""; // Reset to default if not failed
  }
}

function isSameDetector(fileCheckInfoL, fileCheckInfoF) {
  const [
    detector_idL,
    addressL,
    acquisition_dateL,
    acquisition_timeL,
  ] = fileCheckInfoL;
  const [
    detector_idF,
    addressF,
    acquisition_dateF,
    acquisition_timeF,
  ] = fileCheckInfoF;
  if (
    detector_idL == detector_idF &&
    addressL == addressF &&
    acquisition_dateL == acquisition_dateF
  ) {
    // check is +/- hrs time
    const hr = 10000;
    const delta = Math.abs(acquisition_timeL - acquisition_timeF);
    return delta <= hr;
  }

  return false;
}

helpBtn.addEventListener("click", () => {
  alert("Not Available!");
});
