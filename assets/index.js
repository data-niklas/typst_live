let split = import("./split-grid.js");
const TIMEOUT = 500;
const FONT_NAMES = [
  "DejaVuSansMono-BoldOblique.ttf",
  "LinLibertine_RBI.ttf",
  "NewCM10-BoldItalic.otf",
  "NewCMMath-Book.otf",
  "DejaVuSansMono-Bold.ttf",
  "LinLibertine_RB.ttf",
  "NewCM10-Bold.otf",
  "NewCMMath-Regular.otf",
  "DejaVuSansMono-Oblique.ttf",
  "LinLibertine_RI.ttf",
  "NewCM10-Italic.otf",
  "DejaVuSansMono.ttf",
  "LinLibertine_R.ttf",
  "NewCM10-Regular.otf",
];

function debounce(fn, timeout) {
  let pending = null;
  return function() {
    clearTimeout(pending);
    pending = setTimeout(fn, timeout);
  };
}

class PackageManager{
  constructor(bindings){
    this.bindings = new bindings.PackageManager()
    this.enablePackageInstallation()
  }

  packageToRow(pkg) {
  let row = document.createElement("tr");
  let name = document.createElement("td");
  let namespace = document.createElement("td");
  let version = document.createElement("td");
  let deleteElement = document.createElement("td");
  let deleteButton = document.createElement("button");
  deleteButton.addEventListener("click", (_) => {
    this.bindings.delete_package(pkg);
    this.updatePackageList();
  });
  deleteButton.textContent = "X";
  deleteElement.appendChild(deleteButton);
  name.textContent = pkg.name;
  namespace.textContent = pkg.namespace;
  version.textContent =
    pkg.version.major + "." + pkg.version.minor + "." + pkg.version.patch;

  row.appendChild(namespace);
  row.appendChild(name);
  row.appendChild(version);
  row.appendChild(deleteElement);
  return row;
}

updatePackageList() {
  let packageList = document.getElementById("package-list");
  let packages = this.bindings.list_packages();
  let rows = packages.map(this.packageToRow);
  packageList.replaceChildren(...rows);
}


enablePackageInstallation() {
  let packageInput = document.getElementById("package-input");
  packageInput.addEventListener("keydown", event => {
    if (event.key === "Enter") {
      this.bindings.download_package_from_str(packageInput.value).then(_ => {
        this.updatePackageList();
        packageInput.value = "";
      });
    }
  });
  packageInput.value = "";
}
}

class App {
  constructor(){
    this.initPre()
    document.addEventListener("wasmload", event => {
      this.initWasm(event.detail.bindings)
      this.initPost()
    })
  }


  initPre(){
    this.initDialogs()
    this.initCode()
    this.initSplit()
  }

  initWasm(bindings){
    this.bindings = bindings
    this.typst = new bindings.SystemWorld()
    this.packageManager = new PackageManager(bindings)
  }

  initPost(){
    this.initCodePost()
    this.initVersion()
    this.initSettingsPost()
    this.initFonts()
  }

  initDialogs(){
    this.dialogs = []
    window.addEventListener("keydown", (e) => {
      if (e.key === "Escape") {
        this.dialogs.forEach(dialog => dialog.close())
      }
    })
    // init packages before initDialog, such that packages are updates, before dialog is shown
    this.initPackages()
    this.initDialog("about")
    this.initDialog("package")
    this.initDialog("settings")
  }

  initDialog(name){
    let dialog = document.getElementById(name + "-dialog")
    let button = document.getElementById(name + "-button")
    this.dialogs.push(dialog)
    button.addEventListener('click', _=>{
      dialog.showModal()
    })
    dialog.addEventListener('click', function(event) {
      var rect = dialog.getBoundingClientRect()
      var isInDialog = (rect.top <= event.clientY && event.clientY <= rect.top + rect.height &&
      rect.left <= event.clientX && event.clientX <= rect.left + rect.width)
      if (!isInDialog) {
        dialog.close()
      }
    })
  }

  initFonts(){
    //TODO dl in pre and init in post
  let promises = FONT_NAMES.map((font) => {
    return new Promise((resolve, reject) => {
      fetch(`fonts/${font}`).then((response) => {
        response.arrayBuffer().then(resolve, reject);
      }, reject);
    });
  });
  Promise.all(promises).then((buffers) => {
    this.typst.add_fonts(buffers);
    let code = document.getElementById("code").value;
    this.recompile(code);
  });
  }

  initPackages(){
    this.packageManager = null
    let button = document.getElementById("package-button")
    button.addEventListener("click", e=>{
      if (this.packageManager == null){
        // TODO display error message
        return
      }
      this.packageManager.updatePackageList()
    })
  }


  initCode(){
    let textarea = document.getElementById("code");
    textarea.addEventListener("keydown", (e) => {
      if (e.keyCode === 9) {
        e.preventDefault();

        textarea.setRangeText(
          "  ",
          textarea.selectionStart,
          textarea.selectionStart,
          "end",
        );
      }
    });
  }

  initCodePost(){
    this.loadFromURL()
  }

  initSettingsPost(){
      let toggle = document.getElementById("save-toggle");
  this.setCompileOnWrite(!toggle.checked);
  toggle.addEventListener("change", () => {
    this.setCompileOnWrite(!toggle.checked);
  });
  }



initSplit() {
  Split({
    minSize: 300,
    columnGutters: [
      {
        track: 1,
        element: document.querySelector(".gutter-col-1"),
      },
    ],
  });
}
initVersion(typst) {
  let version = document.getElementById("version");
  version.textContent = "v" + this.bindings.version();
}
onCodeChange() {
  let code = document.getElementById("code").value;
  let encoded_code = this.bindings.encode_string_into_url(code);
  if (encoded_code != null)
    window.history.replaceState(
      window.history.state,
      "",
      "/?text=" + encoded_code,
    );
  this.recompile(code);
}
loadFromURL() {
  let path = window.location.search.slice(1);
  if (path.length <= 5) return;
  let code = decodeURIComponent(path.slice(5));
  code = this.bindings.decode_string_from_url(code);
  document.getElementById("code").value = code;
  if (code == null) {
    window.location.search = "";
  }
}


onCtrlS(e) {
  if (e.ctrlKey && e.key === "s") {
    e.preventDefault();
    this.onCodeChange();
  }
}

setCompileOnWrite(enable) {
const onWritePause = debounce(this.onCodeChange.bind(this), TIMEOUT);
  let code = document.getElementById("code");
  if (enable) {
    code.removeEventListener("keydown",this.onCtrlS);
    code.addEventListener("input", onWritePause);
  } else {
    code.removeEventListener("input", onWritePause);
    code.addEventListener("keydown", this.onCtrlS);
  }
}
recompile(code) {
  try {
    let result = this.typst.compile_to_pdf(code);
    document.getElementById("pdf").src = result;
  } catch (errors) {
    errors.forEach(
      (error) =>
        new Notify({
          status: "error",
          title: "Build failed",
          text: error,
          effect: "fade",
          speed: 300,
          showIcon: true,
          showCloseButton: true,
          autoclose: true,
          autotimeout: 5000,
          gap: 20,
          distance: 20,
          type: 1,
          position: "right top",
        }),
    );
    console.log(errors);
  }
}
}

window.addEventListener("load", _=> new App())
