export function createLynxFiberHost(parentComponentId) {
  return {
    createElement(tag) {
      if (tag === "view") {
        return __CreateView(parentComponentId);
      }
      if (tag === "text") {
        return __CreateText(parentComponentId);
      }
      if (tag === "image") {
        return __CreateImage(parentComponentId);
      }
      if (tag === "scroll-view") {
        return __CreateScrollView(parentComponentId);
      }
      return __CreateElement(tag, parentComponentId);
    },

    createRawText(text) {
      return __CreateRawText(text);
    },

    appendElement(parent, child) {
      __AppendElement(parent, child);
    },

    insertElementBefore(parent, child, reference) {
      __InsertElementBefore(parent, child, reference);
    },

    removeElement(parent, child) {
      __RemoveElement(parent, child);
    },

    setAttribute(element, name, value) {
      __SetAttribute(element, name, value);
    },

    setId(element, value) {
      __SetID(element, value);
    },

    setClasses(element, value) {
      __SetClasses(element, value);
    },

    setInlineStyles(element, value) {
      __SetInlineStyles(element, value);
    },

    addEventListener(element, name, callback, options) {
      __AddEventListener(element, name, callback, options);
    },

    removeEventListener(element, name, callback, options) {
      __RemoveEventListener(element, name, callback, options);
    },

    flush(root) {
      __FlushElementTree(root);
    },
  };
}
