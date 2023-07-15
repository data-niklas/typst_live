let split = import('./split-grid.js')

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
  if (e.key == 'Tab') {
    e.preventDefault();
    var start = this.selectionStart;
    var end = this.selectionEnd;

    // set textarea value to: text before caret + tab + text after caret
    textarea.value = textarea.value.substring(0, start) +
      "\t" + textarea.value.substring(end);

    // put caret at right position again
    this.selectionStart =
      this.selectionEnd = start + 1;
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
  let saveToggle = document.getElementById("savetoggle")
  let toggle = saveToggle.firstElementChild
  setCompileOnWrite(!toggle.checked)
  toggle.addEventListener('change', ()=>{
    setCompileOnWrite(!toggle.checked)
  })
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

function updatePackageList(packageManager){
  // TODO list all packages
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
}

function enableDialogs(packageManager){
  enablePackageDialog(packageManager)
  window.addEventListener("keydown", e=>{
    if (e.key === 'Escape') {
      dialogs.forEach(dialog=>dialog.close())
    }
  })
}

function loadFromURL(){
  let path = window.location.search.slice(1)
  if (path.length <= 5)return
  let code = decodeURIComponent(path.slice(5))
  document.getElementById("code").value = code
  if (code != "")recompile(code)
}

function onCodeChange(){
      let code = document.getElementById("code").value
      let encoded_code = encodeURIComponent(code)
      window.history.replaceState(window.history.state, "", "/?text=" + encoded_code)
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

document.addEventListener('wasmload', async function() {
  enableTab()
  enableSplit()
  let rust = await import(window.bindingsfile)
  typst = new rust.SystemWorld();
  pm = new rust.PackageManager();
  enableDialogs(pm)
  loadFromURL()
  enableSaveToggle()
})
