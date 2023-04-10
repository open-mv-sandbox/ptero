var viewer = null;

async function init() {
  // Load the viewer WebAssembly module
  console.log('initializing module...');
  const {Viewer} = await import('../pkg');

  // Create the viewer
  const canvas = document.getElementById("viewer");
  viewer = await Viewer.from_canvas(canvas);

  window.requestAnimationFrame(tick);
}

function tick() {
  viewer.tick();

  window.requestAnimationFrame(tick);
}

init();
