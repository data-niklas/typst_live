let split = import('./split-grid.js')

function debounce(fn, timeout){
  let pending = null
  return function(){
    clearTimeout(pending)
    pending = setTimeout(fn, timeout)
  }
}

function enable_tab(){
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

function enable_split(){
  Split({
    minSize: 300,
      columnGutters: [{
          track: 1,
          element: document.querySelector('.gutter-col-1'),
      }],
  })
}

const TIMEOUT = 500

document.addEventListener('wasmload', async function() {
    enable_tab()
  enable_split()
    let rust = await import(window.bindingsfile)
    let typst = new rust.SystemWorld();
    document.getElementById("code").addEventListener("input", debounce(_=>{
      let code = document.getElementById("code").value
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
    }, TIMEOUT))
})
