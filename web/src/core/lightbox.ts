// IMG-VIEW (Kees 10-27): a picture in the chat opens large in the app (zoom and pan) on a
// click, instead of downloading it again. Download stays one button away.
import { mountImage } from "../viewers/image";

export function openLightbox(src: string, name: string): void {
  const d = document.createElement("dialog");
  d.className = "lightbox";
  d.setAttribute("aria-label", name || "Picture");
  const bar = document.createElement("div");
  bar.className = "lightbox-bar";
  const title = document.createElement("span");
  title.className = "lightbox-name";
  title.textContent = name || "Picture";
  const dl = document.createElement("a");
  dl.className = "btn small";
  dl.href = src;
  dl.setAttribute("download", name || "picture");
  dl.textContent = "Download";
  const close = document.createElement("button");
  close.type = "button";
  close.className = "btn small";
  close.textContent = "Close";
  bar.append(title, dl, close);
  const view = document.createElement("div");
  view.className = "lightbox-view";
  d.append(bar, view);
  document.body.append(d);
  const unmount = mountImage(view, src, name);
  const done = () => { unmount(); d.close(); d.remove(); };
  close.addEventListener("click", done);
  d.addEventListener("cancel", (e) => { e.preventDefault(); done(); });
  // A click on the dark backdrop closes it, as in other chat apps.
  d.addEventListener("click", (e) => { if (e.target === d) done(); });
  d.showModal();
  close.focus();
}
