{
  "kind": "PersistentVolumeClaim",
  "apiVersion": "v1",
  "metadata": {
    "name": "{{{name}}}"
  },
  "spec": {
    "storageClassName": "{{{storage.class_name}}}",
    "accessModes": [
     "{{{storage.access_mode}}}"
    ],
    "resources": {
      "requests": {
        "storage": "{{{storage.capacity}}}"
      }
    }
  }
}
