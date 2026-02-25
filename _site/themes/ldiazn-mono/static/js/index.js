/**
 * The Monospace Web - media grid alignment
 * By Oskar Wickström, MIT License
 * https://github.com/owickstrom/the-monospace-web
 */
function gridCellDimensions() {
  var element = document.createElement("div");
  element.style.position = "fixed";
  element.style.height = "var(--line-height)";
  element.style.width = "1ch";
  document.body.appendChild(element);
  var rect = element.getBoundingClientRect();
  document.body.removeChild(element);
  return { width: rect.width, height: rect.height };
}

function adjustMediaPadding() {
  var cell = gridCellDimensions();

  function setHeightFromRatio(media, ratio) {
    var rect = media.getBoundingClientRect();
    var realHeight = rect.width / ratio;
    var diff = cell.height - (realHeight % cell.height);
    media.style.setProperty("padding-bottom", diff + "px");
  }

  function setFallbackHeight(media) {
    var rect = media.getBoundingClientRect();
    var height = Math.round((rect.width / 2) / cell.height) * cell.height;
    media.style.setProperty("height", height + "px");
  }

  function onMediaLoaded(media) {
    var width, height;
    switch (media.tagName) {
      case "IMG":
        width = media.naturalWidth;
        height = media.naturalHeight;
        break;
      case "VIDEO":
        width = media.videoWidth;
        height = media.videoHeight;
        break;
      default:
        return;
    }
    if (width > 0 && height > 0) {
      setHeightFromRatio(media, width / height);
    } else {
      setFallbackHeight(media);
    }
  }

  var medias = document.querySelectorAll("img, video");
  for (var i = 0; i < medias.length; i++) {
    var media = medias[i];
    switch (media.tagName) {
      case "IMG":
        if (media.complete) {
          onMediaLoaded(media);
        } else {
          media.addEventListener("load", function() { onMediaLoaded(media); });
          media.addEventListener("error", function() { setFallbackHeight(media); });
        }
        break;
      case "VIDEO":
        if (media.readyState >= 2) {
          onMediaLoaded(media);
        } else {
          media.addEventListener("loadeddata", function() { onMediaLoaded(media); });
          media.addEventListener("error", function() { setFallbackHeight(media); });
        }
        break;
    }
  }
}

adjustMediaPadding();
window.addEventListener("load", adjustMediaPadding);
window.addEventListener("resize", adjustMediaPadding);
