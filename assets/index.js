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

let png_output = false
function update_output_view(value){
  png_output = value
  let output = document.getElementById("output")
  output.children[0].style.display = value ? "none" : ""
  output.children[1].style.display = value ? "" : "none"
}

function enable_output_toggle(){
  let outputtoggle = document.getElementById("outputtoggle")
  let toggle = outputtoggle.firstElementChild
  update_output_view(toggle.checked)
  toggle.addEventListener('change', ()=>{
    update_output_view(toggle.checked)
  })
}

const TIMEOUT = 500

document.addEventListener('wasmload', async function() {
  enable_tab()
  enable_split()
  enable_output_toggle()
    let rust = await import(window.bindingsfile)
    let typst = new rust.SystemWorld();
    document.getElementById("code").addEventListener("input", debounce(_=>{
      let code = document.getElementById("code").value
      try {
        if (png_output){
          let result = typst.compile_to_images(code, 2.0);
          let objects = result.map(url=>{
            let dom_object = document.createElement("embed");
            dom_object.src = url;
            return dom_object;
          });
          let images = document.getElementById("images")
          images.children = [];
          for (object of objects){
            images.appendChild(object)
          }
          // document.getElementById("images").children = objects
        }
        else {
          let result = typst.compile_to_pdf(code);
          document.getElementById("pdf").src = result;
        }
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
