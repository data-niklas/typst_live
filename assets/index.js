let split = import('./split-grid.js')
const FONT_NAMES = [
  "DejaVuSansMono-BoldOblique.ttf", "LinLibertine_RBI.ttf", "NewCM10-BoldItalic.otf", "NewCMMath-Book.otf", "DejaVuSansMono-Bold.ttf","LinLibertine_RB.ttf", "NewCM10-Bold.otf","NewCMMath-Regular.otf",
"DejaVuSansMono-Oblique.ttf", "LinLibertine_RI.ttf","NewCM10-Italic.otf",
"DejaVuSansMono.ttf","LinLibertine_R.ttf","NewCM10-Regular.otf"
]

function debounce(fn, timeout){
  let pending = null
  return function(){
    clearTimeout(pending)
    pending = setTimeout(fn, timeout)
  }
}

function enableTab(){
  let textarea = document.getElementById('code')
  textarea.addEventListener('keydown', e=>{
  if (e.keyCode === 9) {
    e.preventDefault()

    textarea.setRangeText(
      '  ',
      textarea.selectionStart,
      textarea.selectionStart,
      'end'
    )
  }
  })
}

function enableSplit(){
  Split({
    minSize: 300,
      columnGutters: [{
          track: 1,
          element: document.querySelector('.gutter-col-1'),
      }],
  })
}

function enableSaveToggle(){
  let toggle = document.getElementById("save-toggle")
  setCompileOnWrite(!toggle.checked)
  toggle.addEventListener('change', ()=>{
    setCompileOnWrite(!toggle.checked)
  })
}

function loadFonts(typst){
  let promises = FONT_NAMES.map(font=>{
    return new Promise((resolve, reject)=>{
      fetch(`fonts/${font}`).then(response=>{
        response.arrayBuffer().then(resolve, reject)
      }, reject)
    })
  })
  Promise.all(promises).then(buffers=>{
    typst.add_fonts(buffers)
    let code = document.getElementById("code").value
    recompile(code)
  })
}


function enableSettings(typst){
    enableSaveToggle()
}

const TIMEOUT = 500
let typst = null

async function recompile(code){
      try {
          let result = typst.compile_to_pdf(code);
          document.getElementById("pdf").src = result;
      } catch (errors) {
        errors.forEach(error=>new Notify({
          status: 'error',
          title: 'Build failed',
          text: error,
          effect: 'fade',
          speed: 300,
          showIcon: true,
          showCloseButton: true,
          autoclose: true,
          autotimeout: 5000,
          gap: 20,
          distance: 20,
          type: 1,
          position: 'right top'
        }))
        console.log(errors)
      }
}

function packageToRow(packageManager, pkg){
  let row = document.createElement("tr")
  let name = document.createElement("td")
  let namespace = document.createElement("td")
  let version = document.createElement("td")
  let deleteElement = document.createElement("td")
  let deleteButton = document.createElement("button")
  deleteButton.addEventListener("click", _=>{
    packageManager.delete_package(pkg)
    updatePackageList(packageManager)
  })
  deleteButton.textContent = "X"
  deleteElement.appendChild(deleteButton)
  name.textContent = pkg.name
  namespace.textContent = pkg.namespace
  version.textContent = pkg.version.major + "." + pkg.version.minor + "." + pkg.version.patch

  row.appendChild(namespace)
  row.appendChild(name)
  row.appendChild(version)
  row.appendChild(deleteElement)
  return row
}

function updatePackageList(packageManager){
  // TODO list all packages
  let packageList = document.getElementById("package-list")
  let packages = packageManager.list_packages()
  let rows = packages.map(pkg=>packageToRow(packageManager, pkg))
  packageList.replaceChildren(...rows)
}


let dialogs = []
function enablePackageDialog(packageManager){
  let packageDialog = document.getElementById("package-dialog")
  dialogs.push(packageDialog)
  let packageButton = document.getElementById("package-button")
  packageButton.addEventListener("click", _=>{
    updatePackageList(packageManager)
    packageDialog.showModal()
  })
  let packageInput = document.getElementById("package-input")
  packageInput.addEventListener("keydown", e=>{
    if (event.key === "Enter") {
      packageManager.download_package_from_str(packageInput.value).then(_=>{
        updatePackageList(packageManager)
        packageInput.value = ""
      })
    }
  })
  packageInput.value = ""
}

function enableSettingsDialog(){
  let settingsDialog = document.getElementById("settings-dialog")
  dialogs.push(settingsDialog)
  let settingsButton = document.getElementById("settings-button")
  settingsButton.addEventListener("click", _=>{
    settingsDialog.showModal()
  })
}

function enableDialogs(packageManager){
  enablePackageDialog(packageManager)
  enableSettingsDialog()
  window.addEventListener("keydown", e=>{
    if (e.key === 'Escape') {
      dialogs.forEach(dialog=>dialog.close())
    }
  })
}

function loadFromURL(decode_url){
  let path = window.location.search.slice(1)
  if (path.length <= 5)return
  let code = decodeURIComponent(path.slice(5))
  code = decode_url(code)
  document.getElementById("code").value = code
  if (code == null){
    window.location.search = ""
  }
}

function enableVersion(typst){
  let version = document.getElementById("version")
  version.textContent = "v" + typst.version()
}

function onCodeChange(){
      let code = document.getElementById("code").value
      // let encoded_code = encodeURIComponent(code)
    let encoded_code = rust.encode_string_into_url(code)
    if (encoded_code != null)window.history.replaceState(window.history.state, "", "/?text=" + encoded_code)
      recompile(code)
}

const onWritePause = debounce(onCodeChange, TIMEOUT)


function onCtrlS(e){
  if (e.ctrlKey && e.key === 's') {
    e.preventDefault();
    onCodeChange()
  }
}

function setCompileOnWrite(enable){
  let code = document.getElementById("code")
  if (enable){
    code.removeEventListener("keydown", onCtrlS)
    code.addEventListener("input", onWritePause)
  }
  else {
    code.removeEventListener("input", onWritePause)
    code.addEventListener("keydown", onCtrlS)
  }
}
let rust = null
document.addEventListener('wasmload', async function() {
  enableTab()
  enableSplit()
  rust = await import(window.bindingsfile)
  typst = new rust.SystemWorld();
  pm = new rust.PackageManager();
  enableVersion(rust)
  enableDialogs(pm)
  loadFromURL(rust.decode_string_from_url)
  enableSettings(typst)
  loadFonts(typst)
})
