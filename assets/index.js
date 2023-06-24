function debounce(fn, timeout){
  let pending = null
  return function(){
    clearTimeout(pending)
    pending = setTimeout(fn, timeout)
  }
}

document.addEventListener('wasmload', async function() {
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
    }, 2000))
})
