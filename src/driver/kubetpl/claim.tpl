{
  "kind": "PersistentVolumeClaim",
  "apiVersion": "v1",
  "metadata": {
    "name": "{{{name}}}"
  },
  "spec": {
    "storageClassName": "{{{node.spec.storage.class_name}}}",
    "accessModes": [
      "ReadWriteMany",
      "ReadWriteOnce"
    ],
    "resources": {
      "requests": {
        "storage": "{{{node.spec.storage.capacity}}}"
      }
    }
  }
}
